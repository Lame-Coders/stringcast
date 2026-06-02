#!/bin/bash

# --- SETUP SCRIPT FOR UBUNTU WAYLAND ---
# Usage: ./setup_listener.sh <KEY_TYPE> <API_KEY>

API_KEY_TYPE=$1 # e.g., "openai", "gemini"
API_KEY=$2

if [ -z "$API_KEY_TYPE" ] || [ -z "$API_KEY" ]; then
    echo "Usage: $0 <key_type> <api_key>"
    echo "Example: $0 openai sk-your-actual-key-here"
    exit 1
fi

echo "-------------------------------------------------"
echo "🔐 Setting up Stringcast Listener Configuration"
echo "-------------------------------------------------"

# 1. Create the configuration directory
CONFIG_DIR="$HOME/.config/stringcast"
mkdir -p "$CONFIG_DIR"

CONFIG_FILE="$CONFIG_DIR/config.ini"

# 2. Write the key securely to the config file
echo "[API_KEYS]" > "$CONFIG_FILE"
echo "API_TYPE = $API_KEY_TYPE" >> "$CONFIG_FILE"
echo "API_KEY = $API_KEY" >> "$CONFIG_FILE"
echo "CONFIG_FILE=$CONFIG_FILE" >> "$CONFIG_FILE"

echo "✅ Success! API key saved to: $CONFIG_FILE"
echo "================================================="
echo "NEXT STEPS:"
echo "1. Ensure your user is in the 'input' group (sudo usermod -aG input $USER)."
echo "2. Log out and log back in to activate group changes."
echo "3. Run the listener (without sudo): python3 scripts/wayland_listener.py"