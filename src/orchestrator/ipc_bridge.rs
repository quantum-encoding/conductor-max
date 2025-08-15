// IPC Bridge for frontend communication
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub agent_id: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Output,
    Input,
    Status,
    Error,
    SystemEvent,
}

pub struct IpcBridge {
    sender: broadcast::Sender<IpcMessage>,
    receiver: Arc<tokio::sync::Mutex<broadcast::Receiver<IpcMessage>>>,
}

impl IpcBridge {
    pub fn new() -> Self {
        let (sender, receiver) = broadcast::channel(1000);
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }
    
    pub fn send_message(&self, message: IpcMessage) -> Result<()> {
        debug!("Sending IPC message: {:?}", message.message_type);
        self.sender.send(message)?;
        Ok(())
    }
    
    pub async fn subscribe(&self) -> broadcast::Receiver<IpcMessage> {
        self.sender.subscribe()
    }
    
    pub fn broadcast_output(&self, agent_id: String, output: String) -> Result<()> {
        self.send_message(IpcMessage {
            agent_id,
            message_type: MessageType::Output,
            payload: serde_json::json!({ "text": output }),
            timestamp: chrono::Utc::now(),
        })
    }
    
    pub fn broadcast_error(&self, agent_id: String, error: String) -> Result<()> {
        self.send_message(IpcMessage {
            agent_id,
            message_type: MessageType::Error,
            payload: serde_json::json!({ "error": error }),
            timestamp: chrono::Utc::now(),
        })
    }
}