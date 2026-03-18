#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "Reinstalling svmgov CLI..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Please install Rust first.${NC}"
    echo "Visit https://rustup.rs/ to install Rust."
    exit 1
fi

# Determine installation directory (check where existing binary is or use default)
if [ -f "/usr/local/bin/svmgov" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -f "$HOME/.local/bin/svmgov" ]; then
    INSTALL_DIR="$HOME/.local/bin"
elif [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

# Remove existing svmgov binary
echo -e "${YELLOW}Removing existing svmgov binary...${NC}"
if [ -f "/usr/local/bin/svmgov" ]; then
    rm -f "/usr/local/bin/svmgov" 2>/dev/null || sudo rm -f "/usr/local/bin/svmgov"
    echo "  Removed /usr/local/bin/svmgov"
fi
if [ -f "$HOME/.local/bin/svmgov" ]; then
    rm -f "$HOME/.local/bin/svmgov"
    echo "  Removed $HOME/.local/bin/svmgov"
fi

# Clean target directory
echo -e "${YELLOW}Cleaning target directory...${NC}"
if [ -d "$SCRIPT_DIR/target" ]; then
    rm -rf "$SCRIPT_DIR/target"
    echo "  Removed $SCRIPT_DIR/target"
else
    echo "  No target directory to clean"
fi

# Build release binary
echo -e "${YELLOW}Building release binary...${NC}"
if ! cargo build --release; then
    echo -e "${RED}Error: Failed to build binary${NC}"
    exit 1
fi

BINARY_PATH="$SCRIPT_DIR/target/release/svmgov"

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

# Create install directory if it doesn't exist
if [ ! -d "$INSTALL_DIR" ]; then
    mkdir -p "$INSTALL_DIR"
    echo "Created directory: $INSTALL_DIR"
fi

# Copy binary
echo -e "${YELLOW}Installing binary to $INSTALL_DIR/svmgov...${NC}"
cp "$BINARY_PATH" "$INSTALL_DIR/svmgov"
chmod +x "$INSTALL_DIR/svmgov"

# Detect shell
SHELL_NAME=$(basename "$SHELL" 2>/dev/null || echo "bash")
CONFIG_FILE=""

case "$SHELL_NAME" in
    bash)
        if [ -f "$HOME/.bashrc" ]; then
            CONFIG_FILE="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            CONFIG_FILE="$HOME/.bash_profile"
        else
            CONFIG_FILE="$HOME/.bashrc"
        fi
        EXPORT_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
    zsh)
        CONFIG_FILE="$HOME/.zshrc"
        EXPORT_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
    fish)
        CONFIG_FILE="$HOME/.config/fish/config.fish"
        EXPORT_LINE="set -gx PATH \"$INSTALL_DIR\" \$PATH"
        ;;
    *)
        echo -e "${YELLOW}Warning: Unknown shell '$SHELL_NAME'. Skipping PATH configuration.${NC}"
        echo "Please manually add $INSTALL_DIR to your PATH."
        CONFIG_FILE=""
        ;;
esac

# Add to PATH if config file exists and entry doesn't already exist
if [ -n "$CONFIG_FILE" ]; then
    if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
        # Check if PATH entry already exists
        if grep -q "$INSTALL_DIR" "$CONFIG_FILE" 2>/dev/null; then
            echo -e "${GREEN}PATH entry already exists in $CONFIG_FILE${NC}"
        else
            echo "" >> "$CONFIG_FILE"
            echo "# svmgov CLI" >> "$CONFIG_FILE"
            echo "$EXPORT_LINE" >> "$CONFIG_FILE"
            echo -e "${GREEN}Added $INSTALL_DIR to PATH in $CONFIG_FILE${NC}"
        fi
    else
        # For /usr/local/bin, it's usually already in PATH, but we'll check
        echo -e "${GREEN}Binary installed to $INSTALL_DIR (usually already in PATH)${NC}"
    fi
fi

echo ""
echo -e "${GREEN}✓ Reinstallation complete!${NC}"
echo ""

# Check if config exists
CONFIG_PATH="$HOME/.svmgov/config.toml"
if [ -f "$CONFIG_PATH" ]; then
    echo -e "${GREEN}✓ Existing config preserved at $CONFIG_PATH${NC}"
else
    echo -e "${YELLOW}No existing config found. Run 'svmgov init' to create one.${NC}"
fi

echo ""
echo "Verify installation:"
echo "  svmgov --version"
echo ""
