// Agent Manager - Process spawning and management
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::sync::Arc;
use std::process::Stdio;
use tokio::sync::{Mutex, RwLock};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
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
    child: Arc<Mutex<tokio::process::Child>>,
    output_buffer: Arc<RwLock<Vec<String>>>,
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
        
        info!("Spawning {} agent (ID: {})", config.agent_type, agent_id);
        
        // Build command using tokio::process instead of PTY for simplicity
        let mut cmd = Command::new(config.agent_type.to_string());
        
        // Set up pipes for stdin/stdout/stderr
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        // Add workspace path if specified
        if let Some(workspace) = &config.workspace_path {
            cmd.current_dir(workspace);
        }
        
        // No need to pass API keys - claude and gemini CLIs handle their own auth
        
        // Spawn the process
        let mut child = cmd.spawn()?;
        
        let status = Arc::new(RwLock::new(AgentStatus {
            id: agent_id.clone(),
            agent_type: config.agent_type.to_string(),
            running: true,
            start_time: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            commands_sent: 0,
            workspace: config.workspace_path.clone(),
        }));
        
        let output_buffer = Arc::new(RwLock::new(Vec::new()));
        
        // Start output reader task for stdout
        if let Some(stdout) = child.stdout.take() {
            let buffer_clone = output_buffer.clone();
            let status_clone = status.clone();
            
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                
                loop {
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            info!("Agent stdout stream ended");
                            break;
                        }
                        Ok(_) => {
                            // Update buffer
                            let mut buffer = buffer_clone.write().await;
                            buffer.push(line.clone());
                            
                            // Keep buffer size reasonable
                            if buffer.len() > 10000 {
                                buffer.drain(0..1000);
                            }
                            
                            // Update last activity
                            status_clone.write().await.last_activity = chrono::Utc::now();
                            
                            line.clear();
                        }
                        Err(e) => {
                            error!("Error reading agent stdout: {}", e);
                            break;
                        }
                    }
                }
            });
        }
        
        // Start output reader task for stderr
        if let Some(stderr) = child.stderr.take() {
            let buffer_clone = output_buffer.clone();
            
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                
                loop {
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
                        Ok(_) => {
                            // Prefix stderr lines
                            let mut buffer = buffer_clone.write().await;
                            buffer.push(format!("[stderr] {}", line));
                            line.clear();
                        }
                        Err(_) => break,
                    }
                }
            });
        }
        
        Ok(AgentProcess {
            id: agent_id,
            agent_type: config.agent_type,
            child: Arc::new(Mutex::new(child)),
            output_buffer,
            status,
        })
    }
}

impl AgentProcess {
    pub async fn send_command(&self, command: &str) -> Result<()> {
        let mut child = self.child.lock().await;
        
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(format!("{}\n", command).as_bytes()).await?;
            stdin.flush().await?;
            
            // Update status
            let mut status = self.status.write().await;
            status.commands_sent += 1;
            status.last_activity = chrono::Utc::now();
            
            debug!("Sent command to agent {}: {}", self.id, command);
        } else {
            return Err(anyhow::anyhow!("Agent stdin not available"));
        }
        
        Ok(())
    }
    
    pub async fn kill(&self) -> Result<()> {
        info!("Killing agent {}", self.id);
        
        let mut child = self.child.lock().await;
        child.kill().await?;
        
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
    
    pub async fn get_output_buffer(&self, lines: usize) -> Vec<String> {
        let buffer = self.output_buffer.read().await;
        let start = if buffer.len() > lines {
            buffer.len() - lines
        } else {
            0
        };
        buffer[start..].to_vec()
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        // Best effort cleanup
        let child = self.child.clone();
        let id = self.id.clone();
        tokio::spawn(async move {
            let mut c = child.lock().await;
            let _ = c.kill().await;
            info!("Cleaned up agent {}", id);
        });
    }
}