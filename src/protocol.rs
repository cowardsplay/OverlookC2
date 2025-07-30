use crate::{types::*, crypto::EncryptionKey};
use serde_json;
use anyhow::{Result, anyhow};
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

/// Protocol handler for encrypted WebSocket communication
pub struct Protocol {
    encryption_key: EncryptionKey,
}

impl Protocol {
    /// Create a new protocol handler with encryption key
    pub fn new(encryption_key: EncryptionKey) -> Self {
        Self { encryption_key }
    }
    
    /// Serialize and encrypt a message
    pub fn serialize_message(&self, message: &Message) -> Result<String> {
        let json = serde_json::to_string(message)
            .map_err(|e| anyhow!("Serialization failed: {}", e))?;
        self.encryption_key.encrypt_b64(json.as_bytes())
    }
    
    /// Decrypt and deserialize a message
    pub fn deserialize_message(&self, encrypted_data: &str) -> Result<Message> {
        let decrypted = self.encryption_key.decrypt_b64(encrypted_data)?;
        let json = String::from_utf8(decrypted)
            .map_err(|e| anyhow!("UTF-8 decode failed: {}", e))?;
        serde_json::from_str(&json)
            .map_err(|e| anyhow!("Deserialization failed: {}", e))
    }
}

/// WebSocket connection manager for agents
pub struct AgentConnection {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}

impl AgentConnection {
    /// Create a new agent connection
    pub async fn connect(url: &str, encryption_key: EncryptionKey) -> Result<Self> {
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| anyhow!("WebSocket connection failed: {}", e))?;
        
        let protocol = Protocol::new(encryption_key);
        
        Ok(Self {
            ws_stream,
            protocol,
            agent_id: None,
        })
    }
    
    /// Send a message to the server
    pub async fn send_message(&mut self, message: &Message) -> Result<()> {
        let encrypted = self.protocol.serialize_message(message)?;
        self.ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(encrypted)).await
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
    
    /// Receive a message from the server
    pub async fn receive_message(&mut self) -> Result<Option<Message>> {
        if let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    let message = self.protocol.deserialize_message(&text)?;
                    Ok(Some(message))
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    Ok(None)
                }
                Err(e) => Err(anyhow!("WebSocket error: {}", e)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
    
    /// Set the agent ID for this connection
    pub fn set_agent_id(&mut self, agent_id: AgentId) {
        self.agent_id = Some(agent_id);
    }
    
    /// Get the agent ID
    pub fn get_agent_id(&self) -> Option<AgentId> {
        self.agent_id
    }
    
    /// Send a heartbeat message
    pub async fn send_heartbeat(&mut self) -> Result<()> {
        if let Some(agent_id) = self.agent_id {
            let heartbeat = Message::Heartbeat {
                agent_id,
                timestamp: Utc::now(),
            };
            self.send_message(&heartbeat).await
        } else {
            Err(anyhow!("No agent ID set"))
        }
    }
    
    /// Register with the server
    pub async fn register(&mut self, agent_info: AgentInfo) -> Result<()> {
        let register_msg = Message::Register { agent_info };
        self.send_message(&register_msg).await
    }
    
    /// Send a command response
    pub async fn send_response(&mut self, command_id: Uuid, response: crate::types::CommandResponse) -> Result<()> {
        let response_msg = Message::Response { command_id, response };
        self.send_message(&response_msg).await
    }
}

/// Server-side connection manager for TLS connections
pub struct ServerConnection {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}

impl ServerConnection {
    /// Create a new server connection from an accepted WebSocket (TLS)
    pub fn new(ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>, encryption_key: EncryptionKey) -> Self {
        let protocol = Protocol::new(encryption_key);
        Self {
            ws_stream,
            protocol,
            agent_id: None,
        }
    }
    
    /// Send a message to the agent
    pub async fn send_message(&mut self, message: &Message) -> Result<()> {
        let encrypted = self.protocol.serialize_message(message)?;
        self.ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(encrypted)).await
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
    
    /// Receive a message from the agent
    pub async fn receive_message(&mut self) -> Result<Option<Message>> {
        if let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    let message = self.protocol.deserialize_message(&text)?;
                    Ok(Some(message))
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    Ok(None)
                }
                Err(e) => Err(anyhow!("WebSocket error: {}", e)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
    
    /// Set the agent ID for this connection
    pub fn set_agent_id(&mut self, agent_id: AgentId) {
        self.agent_id = Some(agent_id);
    }
    
    /// Get the agent ID
    pub fn get_agent_id(&self) -> Option<AgentId> {
        self.agent_id
    }
    
    /// Send a command to the agent
    pub async fn send_command(&mut self, command_id: Uuid, command: crate::types::CommandType) -> Result<()> {
        let command_msg = Message::Command { command_id, command };
        self.send_message(&command_msg).await
    }
}

/// Server-side connection manager for plain TCP connections
pub struct ServerConnectionPlain {
    ws_stream: WebSocketStream<TcpStream>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}

impl ServerConnectionPlain {
    /// Create a new server connection from an accepted WebSocket (plain TCP)
    pub fn new(ws_stream: WebSocketStream<TcpStream>, encryption_key: EncryptionKey) -> Self {
        let protocol = Protocol::new(encryption_key);
        Self {
            ws_stream,
            protocol,
            agent_id: None,
        }
    }
    
    /// Send a message to the agent
    pub async fn send_message(&mut self, message: &Message) -> Result<()> {
        let encrypted = self.protocol.serialize_message(message)?;
        self.ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(encrypted)).await
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;
        Ok(())
    }
    
    /// Receive a message from the agent
    pub async fn receive_message(&mut self) -> Result<Option<Message>> {
        if let Some(msg) = self.ws_stream.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    let message = self.protocol.deserialize_message(&text)?;
                    Ok(Some(message))
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    Ok(None)
                }
                Err(e) => Err(anyhow!("WebSocket error: {}", e)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }
    
    /// Set the agent ID for this connection
    pub fn set_agent_id(&mut self, agent_id: AgentId) {
        self.agent_id = Some(agent_id);
    }
    
    /// Get the agent ID
    pub fn get_agent_id(&self) -> Option<AgentId> {
        self.agent_id
    }
    
    /// Send a command to the agent
    pub async fn send_command(&mut self, command_id: Uuid, command: crate::types::CommandType) -> Result<()> {
        let command_msg = Message::Command { command_id, command };
        self.send_message(&command_msg).await
    }
}

/// Session manager for tracking agent connections
pub struct SessionManager {
    sessions: HashMap<AgentId, Session>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }
    
    /// Add a new agent session
    pub fn add_session(&mut self, agent_id: AgentId, session: Session) {
        println!("[DEBUG][SessionManager] Adding session for agent_id: {}", agent_id);
        println!("[DEBUG][SessionManager] Session details: {:?}", session);
        self.sessions.insert(agent_id, session);
        println!("[DEBUG][SessionManager] Total sessions after add: {}", self.sessions.len());
    }
    
    /// Remove an agent session
    pub fn remove_session(&mut self, agent_id: &AgentId) {
        self.sessions.remove(agent_id);
    }
    
    /// Get a session by agent ID
    pub fn get_session(&self, agent_id: &AgentId) -> Option<&Session> {
        self.sessions.get(agent_id)
    }
    
    /// Get a mutable session by agent ID
    pub fn get_session_mut(&mut self, agent_id: &AgentId) -> Option<&mut Session> {
        self.sessions.get_mut(agent_id)
    }
    
    /// Update agent heartbeat
    pub fn update_heartbeat(&mut self, agent_id: &AgentId) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(agent_id) {
            session.last_heartbeat = Utc::now();
            session.status = AgentStatus::Online;
            Ok(())
        } else {
            Err(anyhow!("Agent session not found"))
        }
    }
    
    /// Get all online agents
    pub fn get_online_agents(&self) -> Vec<AgentId> {
        self.sessions
            .iter()
            .filter(|(_, session)| matches!(session.status, AgentStatus::Online))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get all sessions
    pub fn get_all_sessions(&self) -> &HashMap<AgentId, Session> {
        println!("[DEBUG][SessionManager] get_all_sessions called. Total: {}", self.sessions.len());
        for (id, sess) in &self.sessions {
            println!("[DEBUG][SessionManager] Session: id={} status={:?}", id, sess.status);
        }
        &self.sessions
    }
    
    /// Clean up offline sessions
    pub fn cleanup_offline_sessions(&mut self, timeout_seconds: u64) {
        let now = Utc::now();
        let timeout_duration = chrono::Duration::seconds(timeout_seconds as i64);
        
        self.sessions.retain(|_, session| {
            let time_since_heartbeat = now - session.last_heartbeat;
            if time_since_heartbeat > timeout_duration {
                session.status = AgentStatus::Offline;
            }
            true // Keep all sessions, just mark as offline
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
} 