# Linux Wayland Listener PoC

This is an experimental Linux-only fallback for Wayland desktops where normal global keyboard hooks are blocked. It is separate from the Rust Stringcast runtime.

The listener reads keyboard events from `/dev/input`, detects a trigger such as `?fix`, calls an AI provider, and injects replacement keystrokes through `uinput`.

## Status

- Proof of concept, not production-ready.
- Tested scope is limited to basic ASCII typing.
- It does not use the Rust config or keychain flow.
- It requires sensitive device access.
- It is not intended for macOS or Windows.

For macOS and Windows, the preferred path is packaging the Rust app so users can download a binary without installing Cargo.

## Security Notes

Access to `/dev/input` can read keyboard events, including sensitive typing. Access to `uinput` can inject keystrokes. Only run this on a machine and user account you trust.

Avoid putting API keys directly in systemd unit files. Use an environment file with restricted permissions instead.

## Dependencies

Ubuntu/Debian:

```bash
sudo apt update
sudo apt install python3 python3-pip python3-venv pkg-config libudev-dev
```

Python packages:

```bash
python3 -m venv .venv
. .venv/bin/activate
pip install evdev requests
```

## Environment

Copy the example file and edit it:

```bash
cp scripts/wayland_listener.env.example .env.wayland
chmod 600 .env.wayland
```

For a system install:

```bash
sudo mkdir -p /etc/stringcast
sudo cp scripts/wayland_listener.env.example /etc/stringcast/wayland-listener.env
sudo chmod 600 /etc/stringcast/wayland-listener.env
sudo editor /etc/stringcast/wayland-listener.env
```

## Run Manually

From the repo root:

```bash
sudo env $(grep -v '^#' .env.wayland | xargs) .venv/bin/python scripts/wayland_listener.py
```

Running with `sudo` is the simplest PoC path because the script needs keyboard and `uinput` access.

## Run With systemd

The sample unit assumes the repo is installed at `/opt/stringcast` and reads secrets from `/etc/stringcast/wayland-listener.env`.

```bash
sudo mkdir -p /opt
sudo cp -R . /opt/stringcast
cd /opt/stringcast
sudo python3 -m venv .venv
sudo .venv/bin/pip install evdev requests

sudo cp systemd/stringcast-wayland-listener.service /etc/systemd/system/stringcast-wayland-listener.service
sudo systemctl daemon-reload
sudo systemctl enable --now stringcast-wayland-listener.service
```

Check logs:

```bash
sudo journalctl -u stringcast-wayland-listener.service -f
```

## Lower-Privilege Option

The sample `udev/99-stringcast-input.rules` file grants members of the `input` group access to input devices and `uinput`.

This is still sensitive. Anyone in that group can read keyboard input. Use it only if you understand the tradeoff.

## Known Gaps

- Non-ASCII text and many punctuation/input-method cases are incomplete.
- It does not share Rust command configuration.
- It does not use OS keychain storage.
- It does not support all built-in Stringcast commands.
- It has no robust foreground-app exclusion logic.
- Error reporting is basic.
