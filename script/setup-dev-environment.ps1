#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Pulsar Engine Development Environment Setup
.DESCRIPTION
    Automatically installs and configures all dependencies for Pulsar Engine development:
    - Rust toolchain (official rustup)
    - Windows SDK (for DirectX development)
    - Build tools and compilers
    - System libraries
.NOTES
    Run this script with administrator privileges on Windows
#>

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Pulsar Engine - Development Setup" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Detect OS
if ($IsWindows -or $env:OS -eq "Windows_NT") {
    $OS = "Windows"
} elseif ($IsLinux) {
    $OS = "Linux"
} elseif ($IsMacOS) {
    $OS = "macOS"
} else {
    Write-Host "❌ Unsupported operating system" -ForegroundColor Red
    exit 1
}

Write-Host "🖥️  Detected OS: $OS" -ForegroundColor Green
Write-Host ""

# ============================================
# RUST INSTALLATION (Official rustup method)
# ============================================

function Install-Rust {
    Write-Host "📦 Installing Rust toolchain..." -ForegroundColor Yellow
    
    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        $rustVersion = rustc --version
        Write-Host "✅ Rust already installed: $rustVersion" -ForegroundColor Green
        
        Write-Host "🔄 Updating Rust..." -ForegroundColor Yellow
        rustup update stable
        rustup default stable
    } else {
        Write-Host "📥 Downloading and installing Rust..." -ForegroundColor Yellow
        
        if ($OS -eq "Windows") {
            # Download rustup-init.exe
            $rustupUrl = "https://win.rustup.rs/x86_64"
            $rustupPath = "$env:TEMP\rustup-init.exe"
            
            Write-Host "   Downloading rustup from $rustupUrl..." -ForegroundColor Gray
            Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath
            
            Write-Host "   Running rustup installer..." -ForegroundColor Gray
            & $rustupPath -y --default-toolchain stable --profile default
            
            # Add to PATH for current session
            $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
            
            Remove-Item $rustupPath -Force
        } else {
            # Unix-like systems
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
            
            # Source cargo env
            if (Test-Path "$HOME/.cargo/env") {
                . "$HOME/.cargo/env"
            }
        }
        
        Write-Host "✅ Rust installation complete!" -ForegroundColor Green
    }
    
    # Install required targets and components
    Write-Host "🔧 Installing required Rust components..." -ForegroundColor Yellow
    rustup component add rustfmt clippy rust-analyzer
    
    Write-Host ""
}

# ============================================
# WINDOWS-SPECIFIC DEPENDENCIES
# ============================================

function Install-WindowsDependencies {
    Write-Host "🪟 Installing Windows dependencies..." -ForegroundColor Yellow
    
    # Check for Visual Studio Build Tools or Visual Studio
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    
    if (Test-Path $vsWhere) {
        $vsInstances = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        
        if ($vsInstances) {
            Write-Host "✅ Visual Studio Build Tools found" -ForegroundColor Green
        } else {
            Write-Host "⚠️  Visual Studio found but C++ tools not installed" -ForegroundColor Yellow
        }
    } else {
        Write-Host "⚠️  Visual Studio Build Tools not found" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "📋 You need to install Visual Studio Build Tools manually:" -ForegroundColor Cyan
        Write-Host "   1. Download from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Gray
        Write-Host "   2. Select 'Build Tools for Visual Studio 2022'" -ForegroundColor Gray
        Write-Host "   3. Install these workloads:" -ForegroundColor Gray
        Write-Host "      - Desktop development with C++" -ForegroundColor Gray
        Write-Host "      - Windows SDK (10.0.22621.0 or later)" -ForegroundColor Gray
        Write-Host ""
        
        $response = Read-Host "Open download page now? (y/n)"
        if ($response -eq 'y') {
            Start-Process "https://visualstudio.microsoft.com/downloads/"
        }
    }
    
    # Check Windows SDK
    $sdkPath = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0"
    if (Test-Path $sdkPath) {
        $sdkVersion = (Get-ItemProperty -Path $sdkPath).ProductVersion
        Write-Host "✅ Windows SDK found: $sdkVersion" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Windows SDK not detected" -ForegroundColor Yellow
        Write-Host "   Required for DirectX development" -ForegroundColor Gray
    }
    
    # Check for winget and offer to install common tools
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host ""
        Write-Host "🔧 Installing additional development tools via winget..." -ForegroundColor Yellow
        
        # Git
        if (!(Get-Command git -ErrorAction SilentlyContinue)) {
            Write-Host "   Installing Git..." -ForegroundColor Gray
            winget install --id Git.Git -e --silent --accept-source-agreements --accept-package-agreements
        }
        
        # CMake (useful for some Rust dependencies)
        if (!(Get-Command cmake -ErrorAction SilentlyContinue)) {
            Write-Host "   Installing CMake..." -ForegroundColor Gray
            winget install --id Kitware.CMake -e --silent --accept-source-agreements --accept-package-agreements
        }
    }
    
    Write-Host ""
}

# ============================================
# LINUX-SPECIFIC DEPENDENCIES
# ============================================

function Install-LinuxDependencies {
    Write-Host "🐧 Installing Linux dependencies..." -ForegroundColor Yellow
    
    # Detect distribution
    if (Test-Path /etc/os-release) {
        $osRelease = Get-Content /etc/os-release | ConvertFrom-StringData
        $distro = $osRelease.ID
        $version = $osRelease.VERSION_ID
        
        Write-Host "   Detected: $distro $version" -ForegroundColor Gray
        
        switch ($distro) {
            "ubuntu" {
                Write-Host "   Installing Ubuntu/Debian packages..." -ForegroundColor Gray
                sudo apt-get update
                sudo apt-get install -y `
                    build-essential `
                    gcc g++ clang `
                    cmake pkg-config `
                    libssl-dev `
                    libfontconfig-dev libfontconfig1-dev `
                    libfreetype6-dev `
                    libexpat1-dev `
                    libxcb-composite0-dev `
                    libx11-dev libx11-xcb-dev libxcb1-dev `
                    libxcb-xfixes0-dev libxcb-render0-dev libxcb-shape0-dev libxcb-shm0-dev `
                    libxkbcommon-dev libxkbcommon-x11-dev `
                    libwayland-dev `
                    libasound2-dev `
                    libvulkan-dev vulkan-validationlayers vulkan-tools `
                    libwebkit2gtk-4.1-dev `
                    libgtk-3-dev `
                    libzstd-dev `
                    curl git
            }
            "debian" {
                Write-Host "   Installing Debian packages..." -ForegroundColor Gray
                sudo apt-get update
                sudo apt-get install -y `
                    build-essential `
                    gcc g++ clang `
                    cmake pkg-config `
                    libssl-dev `
                    libfontconfig-dev libfontconfig1-dev `
                    libfreetype6-dev `
                    libexpat1-dev `
                    libxcb-composite0-dev `
                    libx11-dev libx11-xcb-dev libxcb1-dev `
                    libxcb-xfixes0-dev libxcb-render0-dev libxcb-shape0-dev libxcb-shm0-dev `
                    libxkbcommon-dev libxkbcommon-x11-dev `
                    libwayland-dev `
                    libasound2-dev `
                    libvulkan-dev vulkan-validationlayers vulkan-tools `
                    libwebkit2gtk-4.1-dev `
                    libgtk-3-dev `
                    libzstd-dev `
                    curl git
            }
            "fedora" {
                Write-Host "   Installing Fedora packages..." -ForegroundColor Gray
                sudo dnf groupinstall -y "Development Tools"
                sudo dnf install -y `
                    gcc gcc-c++ clang `
                    cmake pkg-config `
                    openssl-devel `
                    fontconfig-devel freetype-devel `
                    expat-devel `
                    libxcb-devel libX11-devel `
                    libxkbcommon-devel libxkbcommon-x11-devel `
                    wayland-devel `
                    alsa-lib-devel `
                    vulkan-devel vulkan-validation-layers vulkan-tools `
                    webkit2gtk4.1-devel `
                    gtk3-devel `
                    libzstd-devel `
                    curl git
            }
            "arch" {
                Write-Host "   Installing Arch Linux packages..." -ForegroundColor Gray
                sudo pacman -Syu --noconfirm
                sudo pacman -S --noconfirm `
                    base-devel `
                    gcc clang `
                    cmake pkg-config `
                    openssl `
                    fontconfig freetype2 `
                    expat `
                    libxcb libx11 `
                    libxkbcommon libxkbcommon-x11 `
                    wayland `
                    alsa-lib `
                    vulkan-icd-loader vulkan-validation-layers vulkan-tools `
                    webkit2gtk `
                    gtk3 `
                    zstd `
                    curl git
            }
            default {
                Write-Host "⚠️  Unsupported Linux distribution: $distro" -ForegroundColor Yellow
                Write-Host "   Please manually install build tools and development libraries" -ForegroundColor Gray
            }
        }
        
        Write-Host "✅ Linux dependencies installed" -ForegroundColor Green
    }
    
    Write-Host ""
}

# ============================================
# MACOS-SPECIFIC DEPENDENCIES
# ============================================

function Install-MacOSDependencies {
    Write-Host "🍎 Installing macOS dependencies..." -ForegroundColor Yellow
    
    # Check for Xcode Command Line Tools
    if (!(xcode-select -p 2>&1)) {
        Write-Host "   Installing Xcode Command Line Tools..." -ForegroundColor Gray
        xcode-select --install
        Write-Host "   Please complete the Xcode installation and re-run this script" -ForegroundColor Yellow
        exit 0
    } else {
        Write-Host "✅ Xcode Command Line Tools found" -ForegroundColor Green
    }
    
    # Check for Homebrew
    if (!(Get-Command brew -ErrorAction SilentlyContinue)) {
        Write-Host "   Installing Homebrew..." -ForegroundColor Gray
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    } else {
        Write-Host "✅ Homebrew found" -ForegroundColor Green
    }
    
    # Install dependencies via Homebrew
    Write-Host "   Installing dependencies via Homebrew..." -ForegroundColor Gray
    brew install `
        cmake `
        pkg-config `
        openssl `
        zstd
    
    Write-Host "✅ macOS dependencies installed" -ForegroundColor Green
    Write-Host ""
}

# ============================================
# VERIFY INSTALLATION
# ============================================

function Test-Installation {
    Write-Host "🔍 Verifying installation..." -ForegroundColor Yellow
    Write-Host ""
    
    $allGood = $true
    
    # Check Rust
    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        $rustVersion = rustc --version
        $cargoVersion = cargo --version
        Write-Host "✅ Rust: $rustVersion" -ForegroundColor Green
        Write-Host "✅ Cargo: $cargoVersion" -ForegroundColor Green
    } else {
        Write-Host "❌ Rust not found in PATH" -ForegroundColor Red
        $allGood = $false
    }
    
    # Check rustfmt
    if (Get-Command rustfmt -ErrorAction SilentlyContinue) {
        Write-Host "✅ rustfmt installed" -ForegroundColor Green
    } else {
        Write-Host "⚠️  rustfmt not found" -ForegroundColor Yellow
    }
    
    # Check clippy
    if (Get-Command cargo-clippy -ErrorAction SilentlyContinue) {
        Write-Host "✅ clippy installed" -ForegroundColor Green
    } else {
        Write-Host "⚠️  clippy not found" -ForegroundColor Yellow
    }
    
    Write-Host ""
    
    if ($allGood) {
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "  ✅ Installation Complete!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "Next steps:" -ForegroundColor Cyan
        Write-Host "  1. Close and reopen your terminal" -ForegroundColor Gray
        Write-Host "  2. Navigate to the Pulsar-Native directory" -ForegroundColor Gray
        Write-Host "  3. Run: cargo build --release" -ForegroundColor Gray
        Write-Host ""
    } else {
        Write-Host "⚠️  Some components failed to install" -ForegroundColor Yellow
        Write-Host "   Please check the errors above and retry" -ForegroundColor Gray
    }
}

# ============================================
# MAIN EXECUTION
# ============================================

Install-Rust

switch ($OS) {
    "Windows" { Install-WindowsDependencies }
    "Linux"   { Install-LinuxDependencies }
    "macOS"   { Install-MacOSDependencies }
}

Test-Installation
