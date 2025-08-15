// 🔱 Agent Orchestrator Module
mod agent_manager;
mod ipc_bridge;
mod session_state;

pub use agent_manager::{AgentManager, AgentConfig, AgentType, AgentProcess};
pub use ipc_bridge::IpcBridge;
pub use session_state::SessionState;

use anyhow::Result;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, debug};
use uuid::Uuid;

pub struct AgentOrchestrator {
    agents: Arc<DashMap<String, Arc<AgentProcess>>>,
    session: Arc<RwLock<SessionState>>,
    ipc_bridge: Arc<IpcBridge>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            session: Arc::new(RwLock::new(SessionState::new())),
            ipc_bridge: Arc::new(IpcBridge::new()),
        }
    }

    pub async fn spawn_agent(&self, config: AgentConfig) -> Result<String> {
        let agent_id = config.agent_id.clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        info!("Spawning {} agent with ID: {}", config.agent_type, agent_id);
        
        let agent = AgentManager::spawn(config).await?;
        
        // Register with session
        self.session.write().await.register_agent(
            agent_id.clone(),
            agent.agent_type.to_string(),
        );
        
        // Store agent process
        self.agents.insert(agent_id.clone(), Arc::new(agent));
        
        info!("✅ Agent {} spawned successfully", agent_id);
        Ok(agent_id)
    }

    pub async fn send_command(&self, agent_id: &str, command: &str) -> Result<()> {
        let agent = self.agents.get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found", agent_id))?;
        
        debug!("Sending command to agent {}: {}", agent_id, command);
        agent.send_command(command).await?;
        
        // Log to session
        self.session.write().await.log_command(agent_id, command);
        
        Ok(())
    }

    pub async fn kill_agent(&self, agent_id: &str) -> Result<()> {
        if let Some((_, agent)) = self.agents.remove(agent_id) {
            info!("Killing agent {}", agent_id);
            agent.kill().await?;
            
            // Update session
            self.session.write().await.unregister_agent(agent_id);
        }
        Ok(())
    }

    pub async fn get_agent_status(&self, agent_id: &str) -> Result<Value> {
        let agent = self.agents.get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found", agent_id))?;
        
        Ok(agent.get_status().await)
    }

    pub async fn list_agents(&self) -> Vec<Value> {
        let mut agents = Vec::new();
        for entry in self.agents.iter() {
            let status = entry.value().get_status().await;
            agents.push(status);
        }
        agents
    }

    pub async fn get_agent_output(&self, agent_id: &str, lines: usize) -> Result<Vec<String>> {
        let agent = self.agents.get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found", agent_id))?;
        
        Ok(agent.get_output_buffer(lines).await)
    }

    pub async fn broadcast_to_strategy(&self, message: &str) -> Result<()> {
        // Broadcast strategic message to all agents
        for entry in self.agents.iter() {
            if let Err(e) = entry.value().send_command(message).await {
                error!("Failed to broadcast to agent {}: {}", entry.key(), e);
            }
        }
        Ok(())
    }
}

impl Drop for AgentOrchestrator {
    fn drop(&mut self) {
        info!("Shutting down Agent Orchestrator...");
        // Agents will be cleaned up by their Drop implementations
    }
}