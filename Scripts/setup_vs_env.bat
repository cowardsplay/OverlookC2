@echo off
echo Setting up Visual Studio Build Tools environment...
echo.

REM Check if we're already in a Visual Studio environment
where cl >nul 2>&1
if %errorlevel% equ 0 (
    echo ✓ Visual Studio environment is already set up
    where cl
    echo.
    where link
    goto :end
)

REM Try to find and run the Visual Studio environment setup
echo Looking for Visual Studio Build Tools...
if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    echo Found Visual Studio 2022 Build Tools
    call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
    echo ✓ Visual Studio 2022 environment variables set
) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    echo Found Visual Studio 2019 Build Tools
    call "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
    echo ✓ Visual Studio 2019 environment variables set
) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    echo Found Visual Studio 2017 Build Tools
    call "%ProgramFiles(x86)%\Microsoft Visual Studio\2017\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
    echo ✓ Visual Studio 2017 environment variables set
) else (
    echo Checking for full Visual Studio installation...
    if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
        echo Found Visual Studio 2022 Community
        call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
        echo ✓ Visual Studio 2022 Community environment variables set
    ) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" (
        echo Found Visual Studio 2022 Professional
        call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
        echo ✓ Visual Studio 2022 Professional environment variables set
    ) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" (
        echo Found Visual Studio 2022 Enterprise
        call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
        echo ✓ Visual Studio 2022 Enterprise environment variables set
    ) else (
        echo Error: Could not find Visual Studio Build Tools or full Visual Studio
        echo.
        echo Please install Visual Studio Build Tools with C++ workload:
        echo 1. Run: install\install.bat (recommended)
        echo 2. Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
        echo 3. Or run: winget install Microsoft.VisualStudio.2022.BuildTools
        echo.
        echo IMPORTANT: Make sure to select "C++ build tools" workload during installation.
        echo After installation, restart your terminal and run this script again.
        pause
        exit /b 1
    )
)

REM Verify the environment is set up correctly
echo.
echo Verifying environment setup...
where cl >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: cl.exe not found after environment setup
    pause
    exit /b 1
)

where link >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: link.exe not found after environment setup
    pause
    exit /b 1
)

echo ✓ Environment setup complete!
echo.
echo Available tools:
where cl
where link

:end
echo.
echo You can now run: Scripts\build.bat
echo.
pause
exit /b 0 