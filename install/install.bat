@echo off
setlocal enabledelayedexpansion

echo ========================================
echo C2 Framework - Dependency Installer
echo ========================================
echo.

REM Check if this is a restart after Rust installation
if exist "temp\rust_installed.flag" (
    echo Detected restart after Rust installation...
    del "temp\rust_installed.flag"
    echo ✓ Continuing with setup...
    echo.
    goto :continue_setup
)

echo Installing all required dependencies...
echo.

REM Create temp directory for downloads
if not exist temp mkdir temp
cd temp

:continue_setup

REM 1. Install Rust if not present
echo [1/3] Installing Rust...
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo Downloading Rust installer...
    powershell -Command "Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile 'rustup-init.exe'"
    
    if exist rustup-init.exe (
        echo Installing Rust...
        rustup-init.exe -y --quiet
        if %errorlevel% equ 0 (
            echo ✓ Rust installed successfully
            echo. > "rust_installed.flag"
            echo Rust installation complete. Restarting script to refresh environment...
            timeout /t 3 /nobreak >nul
            start /wait cmd /c "%~f0"
            exit /b 0
        ) else (
            echo Error: Rust installation failed
            echo Press any key to exit...
            pause >nul
            exit /b 1
        )
    ) else (
        echo Error: Failed to download Rust installer
        echo Press any key to exit...
        pause >nul
        exit /b 1
    )
) else (
    echo ✓ Rust is already installed
    rustc --version
)

echo.

REM 2. Add Windows target for Rust
echo [2/3] Adding Windows target for Rust...
rustup target add x86_64-pc-windows-msvc
if %errorlevel% neq 0 (
    echo Error: Failed to add Windows target
    echo Press any key to exit...
    pause >nul
    exit /b 1
) else (
    echo ✓ Windows target added successfully
)

echo.

REM 3. Update Rust toolchain and setup environment
echo [3/3] Finalizing setup...
rustup update
if %errorlevel% neq 0 (
    echo Warning: Failed to update Rust toolchain
) else (
    echo ✓ Rust toolchain updated
)

echo.

REM Clean up temp directory
if exist temp (
    echo Cleaning up temporary files...
    del "temp\*.flag" 2>nul
    rmdir /s /q temp
)

REM --- Silent Visual Studio Build Tools Installer ---
echo Installing Visual Studio Build Tools (silent)...
powershell -Command "Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_buildtools.exe' -OutFile 'vs_buildtools.exe'"

vs_buildtools.exe --quiet --wait --norestart --nocache ^
  --installPath "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools" ^
  --add Microsoft.VisualStudio.Workload.VCTools ^
  --add Microsoft.VisualStudio.Component.Windows10SDK.19041

del vs_buildtools.exe

echo.
echo =====================================================
echo  IMPORTANT: Manual Visual Studio Component Selection
 echo -----------------------------------------------------
echo  In order to build the C2 framework, you MUST:
echo   1. Open the Visual Studio Installer (search in Start menu)
echo   2. Click "Modify" on your Build Tools installation
echo   3. In the Individual components tab, search for C++
echo   4. Add:
echo      - C++ Universal Windows Platform runtime for v142 build tools
echo      - C++ Universal Windows Platform runtime for v143 build tools
echo   5. Click Modify to install these components
echo.
echo  These components are required for successful compilation.
echo =====================================================
echo.

echo ========================================
echo Installation complete!
echo ========================================
echo.
echo Building the project...
echo.

REM Change back to the project directory
cd ..

echo Building Rust C2 Framework...
cargo build --release

if %errorlevel% neq 0 (
    echo.
    echo Error: Build failed
    echo.
    echo Troubleshooting:
    echo 1. Make sure you have the required C++ build tools (Desktop development with C++)
    echo 2. Install Visual Studio Build Tools 2022 with C++ components manually if needed
    echo 3. Or run: Scripts\troubleshoot_build.bat
    echo.
    echo Press any key to exit...
    pause >nul
    exit /b 1
)

echo.
echo ========================================
echo Build successful!
echo ========================================
echo.
echo Binaries created:
echo   target\release\teamserver.exe
echo   target\release\agent.exe
echo   target\release\client.exe
echo.
echo To run the server:
echo   target\release\teamserver.exe
echo.
echo To run an agent:
echo   target\release\client.exe
echo.
echo The C2 framework is now ready to use!
echo.
echo Installation completed successfully!
echo.
timeout /t 5 /nobreak >nul 