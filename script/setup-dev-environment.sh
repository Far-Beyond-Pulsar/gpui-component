#!/usr/bin/env bash
# Pulsar Engine Development Environment Setup
# Automatically installs and configures all dependencies

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

echo -e "${CYAN}========================================"
echo -e "  Pulsar Engine - Development Setup"
echo -e "========================================${NC}"
echo ""

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="Linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macOS"
else
    echo -e "${RED}❌ Unsupported operating system: $OSTYPE${NC}"
    exit 1
fi

echo -e "${GREEN}🖥️  Detected OS: $OS${NC}"
echo ""

# ============================================
# RUST INSTALLATION (Official rustup method)
# ============================================

install_rust() {
    echo -e "${YELLOW}📦 Installing Rust toolchain...${NC}"
    
    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        echo -e "${GREEN}✅ Rust already installed: $RUST_VERSION${NC}"
        
        echo -e "${YELLOW}🔄 Updating Rust...${NC}"
        rustup update stable
        rustup default stable
    else
        echo -e "${YELLOW}📥 Downloading and installing Rust...${NC}"
        
        echo -e "${GRAY}   Downloading rustup from https://sh.rustup.rs...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
        
        # Source cargo env
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi
        
        echo -e "${GREEN}✅ Rust installation complete!${NC}"
    fi
    
    # Install required components
    echo -e "${YELLOW}🔧 Installing required Rust components...${NC}"
    rustup component add rustfmt clippy rust-analyzer
    
    echo ""
}

# ============================================
# LINUX-SPECIFIC DEPENDENCIES
# ============================================

install_linux_dependencies() {
    echo -e "${YELLOW}🐧 Installing Linux dependencies...${NC}"
    
    # Detect distribution
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO=$ID
        VERSION=$VERSION_ID
        
        echo -e "${GRAY}   Detected: $DISTRO $VERSION${NC}"
        
        case $DISTRO in
            ubuntu|debian|pop|linuxmint)
                echo -e "${GRAY}   Installing Ubuntu/Debian packages...${NC}"
                sudo apt-get update
                sudo apt-get install -y \
                    build-essential \
                    gcc g++ clang \
                    cmake pkg-config \
                    libssl-dev \
                    libfontconfig-dev libfontconfig1-dev \
                    libfreetype6-dev \
                    libexpat1-dev \
                    libxcb-composite0-dev \
                    libx11-dev libx11-xcb-dev libxcb1-dev \
                    libxcb-xfixes0-dev libxcb-render0-dev libxcb-shape0-dev libxcb-shm0-dev \
                    libxkbcommon-dev libxkbcommon-x11-dev \
                    libwayland-dev \
                    libasound2-dev \
                    libvulkan-dev vulkan-validationlayers vulkan-tools \
                    libwebkit2gtk-4.1-dev \
                    libgtk-3-dev \
                    libzstd-dev \
                    curl git
                ;;
            fedora|rhel|centos)
                echo -e "${GRAY}   Installing Fedora/RHEL packages...${NC}"
                sudo dnf groupinstall -y "Development Tools"
                sudo dnf install -y \
                    gcc gcc-c++ clang \
                    cmake pkg-config \
                    openssl-devel \
                    fontconfig-devel freetype-devel \
                    expat-devel \
                    libxcb-devel libX11-devel \
                    libxkbcommon-devel libxkbcommon-x11-devel \
                    wayland-devel \
                    alsa-lib-devel \
                    vulkan-devel vulkan-validation-layers vulkan-tools \
                    webkit2gtk4.1-devel \
                    gtk3-devel \
                    libzstd-devel \
                    curl git
                ;;
            arch|manjaro)
                echo -e "${GRAY}   Installing Arch Linux packages...${NC}"
                sudo pacman -Syu --noconfirm
                sudo pacman -S --noconfirm \
                    base-devel \
                    gcc clang \
                    cmake pkg-config \
                    openssl \
                    fontconfig freetype2 \
                    expat \
                    libxcb libx11 \
                    libxkbcommon libxkbcommon-x11 \
                    wayland \
                    alsa-lib \
                    vulkan-icd-loader vulkan-validation-layers vulkan-tools \
                    webkit2gtk \
                    gtk3 \
                    zstd \
                    curl git
                ;;
            opensuse*|sles)
                echo -e "${GRAY}   Installing openSUSE packages...${NC}"
                sudo zypper install -y \
                    gcc gcc-c++ clang \
                    cmake pkg-config \
                    libopenssl-devel \
                    fontconfig-devel freetype2-devel \
                    libexpat-devel \
                    libxcb-devel libX11-devel \
                    libxkbcommon-devel libxkbcommon-x11-devel \
                    wayland-devel \
                    alsa-devel \
                    vulkan-devel vulkan-validationlayers vulkan-tools \
                    webkit2gtk3-devel \
                    gtk3-devel \
                    libzstd-devel \
                    curl git
                ;;
            *)
                echo -e "${YELLOW}⚠️  Unsupported Linux distribution: $DISTRO${NC}"
                echo -e "${GRAY}   Please manually install build tools and development libraries${NC}"
                return 1
                ;;
        esac
        
        echo -e "${GREEN}✅ Linux dependencies installed${NC}"
    else
        echo -e "${RED}❌ Cannot detect Linux distribution${NC}"
        return 1
    fi
    
    echo ""
}

# ============================================
# MACOS-SPECIFIC DEPENDENCIES
# ============================================

install_macos_dependencies() {
    echo -e "${YELLOW}🍎 Installing macOS dependencies...${NC}"
    
    # Check for Xcode Command Line Tools
    if ! xcode-select -p &> /dev/null; then
        echo -e "${GRAY}   Installing Xcode Command Line Tools...${NC}"
        xcode-select --install
        echo -e "${YELLOW}   Please complete the Xcode installation and re-run this script${NC}"
        exit 0
    else
        echo -e "${GREEN}✅ Xcode Command Line Tools found${NC}"
    fi
    
    # Check for Homebrew
    if ! command -v brew &> /dev/null; then
        echo -e "${GRAY}   Installing Homebrew...${NC}"
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add Homebrew to PATH
        if [[ $(uname -m) == 'arm64' ]]; then
            eval "$(/opt/homebrew/bin/brew shellenv)"
        else
            eval "$(/usr/local/bin/brew shellenv)"
        fi
    else
        echo -e "${GREEN}✅ Homebrew found${NC}"
    fi
    
    # Install dependencies via Homebrew
    echo -e "${GRAY}   Installing dependencies via Homebrew...${NC}"
    brew install \
        cmake \
        pkg-config \
        openssl \
        zstd
    
    echo -e "${GREEN}✅ macOS dependencies installed${NC}"
    echo ""
}

# ============================================
# VERIFY INSTALLATION
# ============================================

verify_installation() {
    echo -e "${YELLOW}🔍 Verifying installation...${NC}"
    echo ""
    
    ALL_GOOD=true
    
    # Check Rust
    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        CARGO_VERSION=$(cargo --version)
        echo -e "${GREEN}✅ Rust: $RUST_VERSION${NC}"
        echo -e "${GREEN}✅ Cargo: $CARGO_VERSION${NC}"
    else
        echo -e "${RED}❌ Rust not found in PATH${NC}"
        ALL_GOOD=false
    fi
    
    # Check rustfmt
    if command -v rustfmt &> /dev/null; then
        echo -e "${GREEN}✅ rustfmt installed${NC}"
    else
        echo -e "${YELLOW}⚠️  rustfmt not found${NC}"
    fi
    
    # Check clippy
    if command -v cargo-clippy &> /dev/null; then
        echo -e "${GREEN}✅ clippy installed${NC}"
    else
        echo -e "${YELLOW}⚠️  clippy not found${NC}"
    fi
    
    echo ""
    
    if [ "$ALL_GOOD" = true ]; then
        echo -e "${GREEN}========================================"
        echo -e "  ✅ Installation Complete!"
        echo -e "========================================${NC}"
        echo ""
        echo -e "${CYAN}Next steps:${NC}"
        echo -e "${GRAY}  1. Close and reopen your terminal (or run: source ~/.cargo/env)${NC}"
        echo -e "${GRAY}  2. Navigate to the Pulsar-Native directory${NC}"
        echo -e "${GRAY}  3. Run: cargo build --release${NC}"
        echo ""
    else
        echo -e "${YELLOW}⚠️  Some components failed to install${NC}"
        echo -e "${GRAY}   Please check the errors above and retry${NC}"
    fi
}

# ============================================
# MAIN EXECUTION
# ============================================

install_rust

case $OS in
    Linux)
        install_linux_dependencies
        ;;
    macOS)
        install_macos_dependencies
        ;;
esac

verify_installation
