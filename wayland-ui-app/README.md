# Stringcast (Wayland UI Edition)

## Introduction

Stringcast is a lightweight, floating desktop utility that uses AI to instantly transform your text.

Whether you need to fix grammar, adopt a professional tone, or translate a paragraph, Stringcast acts as your persistent, "always-on-top" AI assistant right on your desktop.

This project is specifically designed as a secure, root-free solution for modern Linux desktops running Wayland.

## 🛡️ Why a UI App instead of a Keystroke Listener?

If you are using a modern Linux desktop environment (like GNOME or KDE on Wayland), you might wonder why this isn't a background script that listens to your keyboard.

### The Wayland Security Sandbox: By design, Wayland strictly isolates applications. It intentionally blocks global key-logging and synthetic keystroke injection to protect users from malicious software.

### The Root Privilege Problem: While workarounds exist (like reading raw hardware events via /dev/input), they absolutely require running scripts as root (using sudo).

### The Environment Conflict: Running desktop scripts as root is highly insecure, cumbersome for daily use, and breaks standard Python virtual environments.

### The UI Solution: This application uses a native PyQt6 interface configured to hover "always on top" of your other windows. It provides a frictionless workflow that fully respects Wayland's security sandbox—meaning no sudo is required!

## 🚀 Installation & Usage (For Users)

You do not need to install Python, run pip, or touch terminal commands to use Stringcast.

### Download the App: Go to the Releases page of this repository and download the latest standalone executable (e.g., stringcast_ui or system_enhancer).

### Setup your API Key: In the exact same folder where you downloaded the app, create a new text file named .env. Open it and add your Gemini API key like this:
GEMINI_API_KEY="your_actual_key_here"

### Make it Executable: Linux requires you to give downloaded apps permission to run. Right-click the downloaded file -> Properties -> Permissions -> check "Allow executing file as program". (Alternatively, run chmod +x filename in your terminal).

### Run Stringcast: Double-click the file! Type or paste your text into the floating window, select a transformation mode from the dropdown, and hit "Transform Text".

## 🛠️ Cloning & Tweaking (For Developers)

Want to add your own custom AI prompts (like "Rewrite as a Pirate" or "Translate to Python code")? It is incredibly easy to modify.

### Clone the repository:
git clone https://github.com/mclovin22117/stringcast-wayland.git
cd stringcast-wayland

### Set up a virtual environment:
python3 -m venv venv
source venv/bin/activate

### Install dependencies:
pip install PyQt6 google-generativeai openai python-dotenv pyinstaller

### Make your changes:
Open your main Python UI script and simply modify the dropdown list items 
(e.g., self.option_dropdown.addItems([...])) to add new features. 
 The backend AI automatically understands natural language commands!

### Rebuild the standalone app:
pyinstaller --noconsole --onefile your_script_name.py

Your new custom app will be compiled and waiting for you inside the dist/ folder.

## 🤝 Contributing

Contributions are always welcome! If you have an idea to improve the UI, add new AI providers, or optimize the codebase:

Fork the Project

Create your Feature Branch (git checkout -b feature/AmazingFeature)

Commit your Changes (git commit -m 'Add some AmazingFeature')

Push to the Branch (git push origin feature/AmazingFeature)

Open a Pull Request