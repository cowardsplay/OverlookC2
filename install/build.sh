#!/bin/bash

echo "Building Rust C2 Framework..."

echo
echo "Checking Rust installation..."
if ! command -v rustc &> /dev/null; then
    echo "Error: Rust is not installed or not in PATH"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

rustc --version

echo
echo "Building in release mode..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "Error: Build failed"
    exit 1
fi

echo
echo "Build successful!"
echo
echo "Binaries created:"
echo "  target/release/teamserver"
echo "  target/release/client"
echo
echo "To run the server:"
echo "  ./target/release/teamserver"
echo
echo "To run an agent:"
echo "  ./target/release/client"
echo

# Make binaries executable
chmod +x target/release/teamserver
chmod +x target/release/client 
