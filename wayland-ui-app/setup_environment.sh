#!/bin/bash

# ===============================================
# AI Text Enhancer Setup and Launcher
# Instructions: 
# 1. Make sure you have python3 installed (sudo apt install python3)
# 2. Place this script in the same folder as 'system_enhancer.py'
# 3. Run: ./setup_environment.sh
# =================================================

# Exit immediately if any command fails
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PYTHON_SCRIPT="${SCRIPT_DIR}/system_enhancer.py"

echo "==================================================="
echo "  AI Text Enhancer Setup and Launcher"
echo "========================================================="

# --- Dependency Check ---
if ! command -v python3 &> /dev/null; then
    echo "❌ ERROR: python3 command not found."
    echo "Please install Python 3 first (e.g., sudo apt update && sudo apt install python3)."
    exit 1
fi

# --- 1. Virtual Environment Setup ---
VENV_DIR=".venv"
if [ ! -d "$VENV_DIR" ]; then
    echo "✅ Creating isolated Python virtual environment in '$VENV_DIR'..."
    python3 -m venv "$VENV_DIR"
else
    echo "✅ Virtual environment already exists at '$VENV_DIR'. Skipping creation."
fi

# --- 2. Dependency Installation ---
echo "Activating virtual environment..."
source "$VENV_DIR/bin/activate"

# *** FIX APPLIED HERE: Installing the missing SDK ***
echo "Installing required Python packages (including google-generativeai)..."
pip install google-generativeai
# *** FIX APPLIED HERE: Installing the missing SDKs ***
echo "Installing required Python packages..."
pip install google-generativeai openai python-dotenv
# Add any other packages your code uses (e.g., pip install requests)

# --- 3. Execution ---
echo -e "\n============================================================"
echo "Setup Complete. Running the application now..."
echo "============================================================"

# Execute the main script using the activated environment's Python interpreter
python "$PYTHON_SCRIPT"

# --- 4. Cleanup ---
echo -e "\n=================================================="
echo "Application finished."
deactivate
echo "=========================================================="