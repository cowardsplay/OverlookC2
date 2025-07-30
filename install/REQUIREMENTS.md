# C2 Framework - Requirements & Dependencies

## Quick Installation

**Windows:**
```cmd
# Run as Administrator
install.bat
```

**Linux/macOS:**
```bash
chmod +x install_dependencies.sh
./install_dependencies.sh
```

## System Requirements

### Minimum Requirements
- **OS**: Windows 10/11, Linux (Ubuntu 18.04+), macOS 10.15+
- **RAM**: 4GB (8GB recommended)
- **Storage**: 2GB free space
- **Network**: Internet connection for dependency downloads

### Windows Requirements
- **Rust**: 1.70+ (automatically installed)
- **Visual Studio Build Tools 2022**: C++ workload (automatically installed)
- **Windows 10 SDK**: 10.0.19041+ (included with Build Tools)

### Linux Requirements
- **Rust**: 1.70+ (automatically installed)
- **Build Essentials**: gcc, make, etc. (automatically installed)
- **OpenSSL**: Development libraries

### macOS Requirements
- **Rust**: 1.70+ (automatically installed)
- **Xcode Command Line Tools**: 12.0+ (automatically installed)

## Rust Dependencies

All Rust dependencies are managed via `Cargo.toml`:

### Core Dependencies
- **tokio**: Async runtime (1.35+)
- **serde**: Serialization (1.0+)
- **clap**: Command-line interface (4.4+)
- **aes-gcm**: Encryption (0.10+)
- **sysinfo**: System information (0.30+)

### Networking
- **tokio-tungstenite**: WebSocket support (0.21+)
- **tungstenite**: WebSocket protocol (0.21+)

### Cryptography
- **aes-gcm**: AES-GCM encryption
- **rand**: Random number generation
- **base64**: Base64 encoding/decoding
- **sha2**: SHA-2 hashing
- **hmac**: HMAC authentication

### System Operations
- **sysinfo**: System information gathering
- **chrono**: Date/time handling
- **uuid**: UUID generation
- **hostname**: Hostname detection
- **num_cpus**: CPU core detection

### Windows-Specific
- **windows-sys**: Windows API bindings (0.48+)

## Installation Methods

### Method 1: Automatic Installation (Recommended)
```cmd
# Windows
install.bat

# Linux/macOS
./install_dependencies.sh
```

### Method 2: Manual Installation

#### Windows
1. **Install Rust:**
   ```cmd
   # Download and run rustup-init.exe from https://rustup.rs/
   # Or use winget:
   winget install Rustlang.Rust
   ```

2. **Install Visual Studio Build Tools:**
   ```cmd
   # Using winget (recommended)
   winget install Microsoft.VisualStudio.2022.BuildTools
   
   # Or download from:
   # https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
   ```

3. **Add Windows target:**
   ```cmd
   rustup target add x86_64-pc-windows-msvc
   ```

#### Linux
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential curl

# CentOS/RHEL/Fedora
sudo yum groupinstall "Development Tools"  # CentOS/RHEL
sudo dnf groupinstall "Development Tools"  # Fedora

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Method 3: Using Package Managers

#### Windows (Chocolatey)
```cmd
choco install rust visualstudio2022buildtools
```

#### Windows (Scoop)
```cmd
scoop install rust
# Then install Visual Studio Build Tools manually
```

#### Linux (Snap)
```bash
sudo snap install rustup --classic
rustup default stable
```

## Verification

After installation, verify everything works:

```cmd
# Check Rust
rustc --version
cargo --version

# Check build tools (Windows)
where cl

# Check project dependencies
cargo check

# Build the project
cargo build --release
```

## Troubleshooting

### Common Issues

1. **"rustc not found"**
   - Restart terminal after Rust installation
   - Add `%USERPROFILE%\.cargo\bin` to PATH

2. **"cl not found" (Windows)**
   - Install Visual Studio Build Tools
   - Run Developer Command Prompt

3. **Build failures**
   ```cmd
   cargo clean
   cargo update
   cargo build --release
   ```

4. **Permission errors**
   - Run as Administrator (Windows)
   - Use `sudo` (Linux/macOS)

### Getting Help

- **Rust**: https://doc.rust-lang.org/book/
- **Cargo**: https://doc.rust-lang.org/cargo/
- **Visual Studio Build Tools**: https://docs.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools

## Development Setup

For development work, additional tools are recommended:

```cmd
# Install additional Rust tools
rustup component add rustfmt clippy rust-analyzer

# Install development dependencies
cargo install cargo-watch  # For auto-rebuilding
cargo install cargo-audit  # For security audits
```

## Security Notes

- All dependencies are pinned in `Cargo.lock`
- Regular security audits: `cargo audit`
- Keep Rust updated: `rustup update`
- Use `cargo update` to update dependencies when needed 