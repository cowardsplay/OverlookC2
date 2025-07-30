@echo off
echo ========================================
echo C2 Framework - Dependency Checker
echo ========================================
echo.

echo Checking if all dependencies are installed...
echo.

REM 1. Check for Rust installation
echo [1/4] Checking Rust installation...
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Rust is not installed
    echo.
    echo To install Rust:
    echo 1. Run: install\install.bat
    echo 2. Or visit: https://rustup.rs/
    echo.
    pause
    exit /b 1
) else (
    echo ✓ Rust is installed
    rustc --version
)

echo.

REM 2. Check for Windows build tools
echo [2/4] Checking Windows build tools...
where cl >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Visual Studio Build Tools not found
    echo.
    echo To install Visual Studio Build Tools:
    echo 1. Run: install\install.bat
    echo 2. Or download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
    echo 3. Or run: winget install Microsoft.VisualStudio.2022.BuildTools
    echo.
    pause
    exit /b 1
) else (
    echo ✓ Windows build tools are available
    where cl
)

echo.

REM 3. Check for MSVC linker
echo [3/4] Checking MSVC linker...
where link >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ MSVC linker (link.exe) not found
    echo.
    echo This may cause build issues. To fix:
    echo 1. Run: Scripts\setup_vs_env.bat
    echo 2. Or restart your terminal
    echo 3. Or run from a Developer Command Prompt
    echo.
    pause
    exit /b 1
) else (
    echo ✓ MSVC linker (link.exe) is available
    where link
)

echo.

REM 4. Check Windows target for Rust
echo [4/4] Checking Windows target...
rustup target list --installed | findstr x86_64-pc-windows-msvc >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Windows target not installed
    echo.
    echo Installing Windows target...
    rustup target add x86_64-pc-windows-msvc
    if %errorlevel% neq 0 (
        echo Error: Failed to add Windows target
        pause
        exit /b 1
    ) else (
        echo ✓ Windows target added successfully
    )
) else (
    echo ✓ Windows target is installed
)

echo.
echo ========================================
echo All dependencies are installed and ready!
echo ========================================
echo.
echo You can now run: Scripts\build.bat
echo.
pause 