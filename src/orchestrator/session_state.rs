// Session State Management
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub agents: HashMap<String, AgentSession>,
    pub task_history: Vec<TaskRecord>,
    pub total_commands: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub agent_type: String,
    pub started_at: DateTime<Utc>,
    pub commands_sent: usize,
    pub last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub agent_id: String,
    pub command: String,
    pub timestamp: DateTime<Utc>,
    pub v_level: Option<u8>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            agents: HashMap::new(),
            task_history: Vec::new(),
            total_commands: 0,
        }
    }
    
    pub fn register_agent(&mut self, agent_id: String, agent_type: String) {
        self.agents.insert(
            agent_id.clone(),
            AgentSession {
                id: agent_id,
                agent_type,
                started_at: Utc::now(),
                commands_sent: 0,
                last_activity: Utc::now(),
            },
        );
    }
    
    pub fn unregister_agent(&mut self, agent_id: &str) {
        self.agents.remove(agent_id);
    }
    
    pub fn log_command(&mut self, agent_id: &str, command: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.commands_sent += 1;
            agent.last_activity = Utc::now();
        }
        
        self.task_history.push(TaskRecord {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            command: command.to_string(),
            timestamp: Utc::now(),
            v_level: None,
        });
        
        self.total_commands += 1;
    }
    
    pub fn export(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
}