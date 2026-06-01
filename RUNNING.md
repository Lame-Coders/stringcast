# Running Stringcast (Developer / Local)

Quick guide to build, configure, and run Stringcast locally.

## Prerequisites
- Rust toolchain (rustup + cargo).
- Platform native dependencies for input simulation and X11 (Linux example):

  Debian/Ubuntu:
  ```bash
  sudo apt update
  sudo apt install build-essential libxdo-dev libx11-dev libxi-dev libxtst-dev
  ```

  Fedora:
  ```bash
  sudo dnf install @development-tools libxdo-devel libX11-devel libXi-devel libXtst-devel
  ```

  Arch:
  ```bash
  sudo pacman -S base-devel xdotool libx11 libxi libxtst
  ```

- On macOS/Windows: ensure accessibility/input-monitoring permissions are available to the app when running.

## 1) Build

Debug build:

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

## 2) Initialize config
Create a default config file (if none exists):

```bash
# Using the built binary
./target/debug/stringcast init
# or with cargo
cargo run -- init
```

Show config path:

```bash
./target/debug/stringcast show-config
# or
cargo run -- show-config
```

## 3) Add an API key
Set your provider secret in the environment and add key metadata:

```bash
export STRINGCAST_API_KEY="your-secret-here"
./target/debug/stringcast add-key openai key-1 "Main"
# or
STRINGCAST_API_KEY="your-secret-here" cargo run -- add-key openai key-1 "Main"
```

Supported providers:
- `gemini`
- `openai`
- `anthropic`
- `custom` for any OpenAI-compatible endpoint

Examples:

```bash
STRINGCAST_API_KEY="your-secret-here" cargo run -- add-key gemini key-1 "Gemini"
STRINGCAST_API_KEY="your-secret-here" cargo run -- add-key anthropic key-1 "Claude"
STRINGCAST_API_KEY="your-secret-here" cargo run -- add-key custom key-1 "Custom API"
```

Use the provider that matches the API key you are storing, then select it as the active provider in the next step.

## 4) Select provider

Choose the active provider with one of these commands:

```bash
./target/debug/stringcast set-provider gemini
./target/debug/stringcast set-provider openai
./target/debug/stringcast set-provider anthropic
./target/debug/stringcast set-provider custom
```

If you prefer `cargo run`:

```bash
cargo run -- set-provider gemini
cargo run -- set-provider openai
cargo run -- set-provider anthropic
cargo run -- set-provider custom
```

Use the same provider name here that you used when adding the API key.

## 5) Enable and run

Enable the service in config (persists):

```bash
./target/debug/stringcast enable
```

Run the runtime (foreground; logs printed to terminal):

```bash
./target/debug/stringcast run
# or
cargo run -- run
```

To run the release binary:

```bash
./target/release/stringcast run
```

Ctrl+C stops the process.

## 6) Useful commands

Check permissions (macOS/Windows):

```bash
./target/debug/stringcast check-permissions
```

View status (enabled, provider, keys):

```bash
./target/debug/stringcast status
```

Run tests:

```bash
cargo test
```

## 7) Running as a background service (example: systemd)
Create `/etc/systemd/system/stringcast.service` (example):

```ini
[Unit]
Description=Stringcast background service
After=network.target

[Service]
Type=simple
User=YOUR_USER
WorkingDirectory=/home/YOUR_USER/Documents/stringcast
ExecStart=/home/YOUR_USER/Documents/stringcast/target/release/stringcast run
Restart=on-failure

[Install]
WantedBy=default.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now stringcast.service
```

On macOS you'd use `launchd` or a user agent; on Windows use a scheduled task or NSSM/svc wrapper.

## 8) Ubuntu Wayland listener (Python PoC)

Ubuntu Wayland does not allow a normal desktop app to capture global keyboard input reliably, so this repository includes a Python listener that reads `/dev/input` and injects synthetic keystrokes through `uinput`.

This is a separate Linux-only background service. It is intended for Ubuntu/Wayland users who are willing to grant device access.

### Dependencies

Install system packages:

```bash
sudo apt update
sudo apt install python3 python3-pip python3-venv pkg-config libudev-dev
```

Create a virtual environment and install Python packages:

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install evdev requests python-uinput
```

### Configure the API key

The listener reads the API key from `STRINGCAST_API_KEY`:

```bash
export STRINGCAST_API_KEY="your-secret-here"
```

You can also set the trigger and provider URL:

```bash
export STRINGCAST_TRIGGER="?fix"
export STRINGCAST_API_URL="https://api.openai.com/v1/chat/completions"
```

For multiple keys, use comma-separated lists in the provider-specific variables:

```bash
export GEMINI_API_KEYS="gemini-key-1,gemini-key-2"
export OPENAI_API_KEYS="openai-key-1"
export ANTHROPIC_API_KEYS="claude-key-1,claude-key-2"
export XAI_API_KEYS="grok-key-1"
```

The listener will try keys in order and fall back quickly if one fails.

### Install the service

Copy the service file to systemd and enable it:

```bash
sudo cp systemd/stringcast-wayland-listener.service /etc/systemd/system/stringcast-wayland-listener.service
sudo systemctl daemon-reload
sudo systemctl enable --now stringcast-wayland-listener.service
```

The example service runs as `root` so it can access `/dev/input` and `uinput`. If you prefer a less-privileged setup, use the sample `udev/99-stringcast-input.rules` file and run a service account in the `input` group.

### Notes

- The PoC currently supports basic ASCII and Shift-aware keys.
- It uses a background thread for the AI call so input capture stays responsive.
- If the API fails, the script shows an error string instead of crashing.
- This is intentionally separate from the Rust desktop app.

## 8) Troubleshooting
- Linker errors on Linux often mean missing native dev packages (e.g., `-lxdo`). Install `libxdo-dev` / `xdotool` system packages.
- If replacements re-trigger the app, verify `SyntheticInputGuard` behavior and ensure the app's simulated input is suppressed by the listener.
- If clipboard operations fail in certain apps, those apps may block select/copy; see `SPEC.md` for fallback behavior.

## 9) Notes
- This repo uses select-all + clipboard to read active-field text. The app restores the clipboard after each operation.
- For development, `cargo run -- run` is easiest; for production, prefer `cargo build --release` and run the release binary.

---
For architecture details and behavioral specs, see [SPEC.md](SPEC.md).
