#!/bin/bash

echo "========================================"
echo "C2 Framework - Dependency Installer"
echo "========================================"
echo

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    else
        echo "unknown"
    fi
}

OS=$(detect_os)

echo "Detected OS: $OS"
echo

echo "Checking and installing dependencies..."
echo

# 1. Check for Rust installation
echo "[1/4] Checking Rust installation..."
if ! command_exists rustc; then
    echo "Rust not found. Installing Rust..."
    echo
    
    if [[ "$OS" == "linux" ]]; then
        echo "Installing Rust on Linux..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    elif [[ "$OS" == "macos" ]]; then
        echo "Installing Rust on macOS..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
    else
        echo "Please install Rust manually from: https://rustup.rs/"
        exit 1
    fi
    
    echo "Rust installation complete. Please restart your terminal and run this script again."
    exit 0
else
    echo "✓ Rust is installed"
    rustc --version
fi

echo

# 2. Check for build essentials (Linux) or Xcode (macOS)
echo "[2/4] Checking build tools..."
if [[ "$OS" == "linux" ]]; then
    if ! command_exists gcc; then
        echo "Build tools not found. Installing build essentials..."
        if command_exists apt-get; then
            sudo apt-get update
            sudo apt-get install -y build-essential
        elif command_exists yum; then
            sudo yum groupinstall -y "Development Tools"
        elif command_exists dnf; then
            sudo dnf groupinstall -y "Development Tools"
        else
            echo "Please install build tools manually for your distribution"
            exit 1
        fi
    else
        echo "✓ Build tools are available"
    fi
elif [[ "$OS" == "macos" ]]; then
    if ! command_exists xcode-select; then
        echo "Xcode Command Line Tools not found. Installing..."
        xcode-select --install
        echo "Please complete the Xcode installation and run this script again."
        exit 0
    else
        echo "✓ Xcode Command Line Tools are available"
    fi
fi

echo

# 3. Add appropriate target for Rust
echo "[3/4] Adding Rust target..."
if [[ "$OS" == "linux" ]]; then
    rustup target add x86_64-unknown-linux-gnu
elif [[ "$OS" == "macos" ]]; then
    rustup target add x86_64-apple-darwin
fi

if [ $? -ne 0 ]; then
    echo "Error: Failed to add Rust target"
    exit 1
else
    echo "✓ Rust target added successfully"
fi

echo

# 4. Update Rust toolchain
echo "[4/4] Updating Rust toolchain..."
rustup update
if [ $? -ne 0 ]; then
    echo "Warning: Failed to update Rust toolchain"
else
    echo "✓ Rust toolchain updated"
fi

echo
echo "========================================"
echo "Dependency installation complete!"
echo "========================================"
echo
echo "Next steps:"
echo "1. Run: ./build.sh"
echo "2. Or run: cargo build --release"
echo
echo "For development:"
echo "- Run: Scripts/setup_logging.bat (Windows only)"
echo "- Run: cargo build (for debug builds)"
echo 