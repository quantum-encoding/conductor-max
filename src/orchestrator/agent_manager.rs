// Agent Manager - Real PTY terminal spawning and management
use anyhow::Result;
use portable_pty::{CommandBuilder, PtySize, native_pty_system, PtyPair};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::sync::Arc;
use std::io::{Read, Write};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task;
use tracing::{info, error, debug};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    Claude,
    Gemini,
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::Claude => write!(f, "claude"),
            AgentType::Gemini => write!(f, "gemini"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub api_key: String, // Not used - relies on existing CLI auth
    pub agent_id: Option<String>,
    pub workspace_path: Option<String>,
}

pub struct AgentProcess {
    pub id: String,
    pub agent_type: AgentType,
    pty_pair: Arc<Mutex<PtyPair>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output_sender: mpsc::Sender<Vec<u8>>,
    output_receiver: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    status: Arc<RwLock<AgentStatus>>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentStatus {
    id: String,
    agent_type: String,
    running: bool,
    start_time: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    commands_sent: usize,
    workspace: Option<String>,
}

pub struct AgentManager;

impl AgentManager {
    pub async fn spawn(config: AgentConfig) -> Result<AgentProcess> {
        let agent_id = config.agent_id.clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        info!("Spawning {} agent (ID: {}) with real PTY", config.agent_type, agent_id);
        
        // Create PTY system
        let pty_system = native_pty_system();
        
        // Create PTY pair with size
        let pty_pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        
        // Build command - use the actual CLI commands
        let mut cmd = CommandBuilder::new(config.agent_type.to_string());
        
        // Add workspace path if specified
        if let Some(workspace) = &config.workspace_path {
            cmd.cwd(workspace);
        }
        
        // Set environment for better terminal compatibility
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        
        // The CLIs handle their own auth - no API keys needed
        
        // Spawn the child process
        let _child = pty_pair.slave.spawn_command(cmd)?;
        info!("Spawned {} process", config.agent_type);
        
        // Get writer for sending input
        let writer = pty_pair.master.take_writer()?;
        
        // Create channel for output streaming
        let (output_sender, output_receiver) = mpsc::channel::<Vec<u8>>(100);
        
        // Start reader task for PTY output
        let mut reader = pty_pair.master.try_clone_reader()?;
        let sender_clone = output_sender.clone();
        let agent_type_str = config.agent_type.to_string();
        let agent_id_clone = agent_id.clone();
        
        // Spawn blocking reader in separate task
        task::spawn_blocking(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("PTY for {} agent {} closed", agent_type_str, agent_id_clone);
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        if let Err(e) = sender_clone.blocking_send(data) {
                            error!("Failed to send PTY output: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading PTY: {}", e);
                        break;
                    }
                }
            }
        });
        
        let status = Arc::new(RwLock::new(AgentStatus {
            id: agent_id.clone(),
            agent_type: config.agent_type.to_string(),
            running: true,
            start_time: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            commands_sent: 0,
            workspace: config.workspace_path.clone(),
        }));
        
        Ok(AgentProcess {
            id: agent_id,
            agent_type: config.agent_type,
            pty_pair: Arc::new(Mutex::new(pty_pair)),
            writer: Arc::new(Mutex::new(writer)),
            output_sender,
            output_receiver: Arc::new(Mutex::new(output_receiver)),
            status,
        })
    }
}

impl AgentProcess {
    pub async fn send_command(&self, command: &str) -> Result<()> {
        let mut writer = self.writer.lock().await;
        
        // Send command with newline
        writer.write_all(format!("{}\n", command).as_bytes())?;
        writer.flush()?;
        
        // Update status
        let mut status = self.status.write().await;
        status.commands_sent += 1;
        status.last_activity = chrono::Utc::now();
        
        debug!("Sent command to agent {}: {}", self.id, command);
        Ok(())
    }
    
    pub async fn send_raw(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.write_all(data)?;
        writer.flush()?;
        
        // Update activity
        self.status.write().await.last_activity = chrono::Utc::now();
        Ok(())
    }
    
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let pty_pair = self.pty_pair.lock().await;
        pty_pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        info!("Resized PTY for agent {} to {}x{}", self.id, cols, rows);
        Ok(())
    }
    
    pub async fn get_output(&self) -> Option<Vec<u8>> {
        let mut receiver = self.output_receiver.lock().await;
        receiver.recv().await
    }
    
    pub async fn kill(&self) -> Result<()> {
        info!("Killing agent {}", self.id);
        
        // Send Ctrl+C first to try graceful shutdown
        self.send_raw(b"\x03").await.ok();
        
        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Send Ctrl+D to PTY
        self.send_raw(b"\x04").await.ok();
        
        let mut status = self.status.write().await;
        status.running = false;
        
        Ok(())
    }
    
    pub async fn get_status(&self) -> serde_json::Value {
        let status = self.status.read().await;
        json!({
            "id": status.id,
            "type": status.agent_type,
            "running": status.running,
            "start_time": status.start_time.to_rfc3339(),
            "last_activity": status.last_activity.to_rfc3339(),
            "commands_sent": status.commands_sent,
            "workspace": status.workspace,
        })
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        // Best effort cleanup
        let id = self.id.clone();
        let writer = self.writer.clone();
        tokio::spawn(async move {
            let mut w = writer.lock().await;
            let _ = w.write_all(b"\x04"); // Ctrl+D
            info!("Cleaned up agent {}", id);
        });
    }
}