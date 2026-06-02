#!/usr/bin/env python3
"""
Linux Wayland trigger listener (PoC)
- Reads keyboard events from /dev/input via evdev
- Injects synthetic keys via evdev.UInput
- On trigger (default "?fix") erases typed text and calls AI API in background

Configuration priority:
  1. Environment Variables (e.g., STRINGCAST_API_KEY)
  2. Local config.ini file
  3. Hardcoded safe defaults / placeholders

USAGE:
  - Configure environment variables OR create a config.ini file.
  - Run as root or a user in the `input` group so /dev/input/event* is readable and uinput is writable.
"""

import os
import sys
import time
import threading
import subprocess
import configparser
from dataclasses import dataclass
from typing import Optional

try:
    from evdev import InputDevice, categorize, ecodes, list_devices, UInput
except ImportError:
    print("Missing python-evdev. Install with: pip install evdev")
    sys.exit(1)

try:
    import requests
except ImportError:
    print("Missing requests. Install with: pip install requests")
    sys.exit(1)


# ==========================================
# Configuration Loading
# ==========================================
CONFIG_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.ini")
config_parser = configparser.ConfigParser()
config_parser.read(CONFIG_FILE)

def get_config(env_var: str, ini_section: str, ini_key: str, default: str) -> str:
    """Fetch setting by priority: Env Var -> Config File -> Hardcoded Default"""
    # 1. Environment Variable
    if env_var in os.environ and os.environ[env_var].strip():
        return os.environ[env_var].strip()

    # 2. Local File (config.ini)
    if config_parser.has_option(ini_section, ini_key):
        val = config_parser.get(ini_section, ini_key).strip()
        if val:
            return val

    # 3. Hardcoded Default
    return default

# Global Settings
TRIGGER = get_config("STRINGCAST_TRIGGER", "settings", "trigger", "?fix")
API_URL = get_config("STRINGCAST_API_URL", "settings", "api_url", "")
DEFAULT_PROVIDER = get_config("STRINGCAST_PROVIDER", "settings", "provider", "openai").lower()
DEFAULT_MODEL = get_config("STRINGCAST_MODEL", "settings", "model", "gpt-4o-mini")

# ==========================================
# Provider Classes & Discovery
# ==========================================
@dataclass(frozen=True)
class ProviderConfig:
    provider: str
    api_key: str
    model: str

@dataclass(frozen=True)
class ProviderAttempt:
    provider: str
    api_key: str
    model: str
    key_index: int
    total_keys: int

def _clean_provider(value: str) -> str:
    return value.strip().lower()

def discover_provider_configs() -> list[ProviderConfig]:
    configs: list[ProviderConfig] = []
    seen: set[tuple[str, str]] = set()

    def add(provider: str, api_key: str, model: str) -> None:
        if not api_key:
            return
        provider = _clean_provider(provider)
        key = (provider, api_key)
        if key in seen:
            return
        seen.add(key)
        configs.append(ProviderConfig(provider=provider, api_key=api_key, model=model))

    # Explicit default configuration using the priority system
    primary_key = get_config("STRINGCAST_API_KEY", "credentials", "api_key", "PLACEHOLDER_API_KEY")
    add(DEFAULT_PROVIDER, primary_key, DEFAULT_MODEL)

    # Provider specific fallbacks
    gemini_key = get_config("GEMINI_API_KEY", "credentials", "gemini_api_key", "")
    if gemini_key:
        add("gemini", gemini_key, get_config("STRINGCAST_GEMINI_MODEL", "settings", "gemini_model", "gemini-2.0-flash"))

    openai_key = get_config("OPENAI_API_KEY", "credentials", "openai_api_key", "")
    if openai_key:
        add("openai", openai_key, get_config("STRINGCAST_OPENAI_MODEL", "settings", "openai_model", DEFAULT_MODEL))

    grok_key = get_config("GROK_API_KEY", "credentials", "grok_api_key", "")
    if grok_key:
        add("grok", grok_key, get_config("STRINGCAST_GROK_MODEL", "settings", "grok_model", "grok-2-latest"))

    claude_key = get_config("ANTHROPIC_API_KEY", "credentials", "anthropic_api_key", "")
    if claude_key:
        add("claude", claude_key, get_config("STRINGCAST_CLAUDE_MODEL", "settings", "claude_model", "claude-3-5-sonnet-20240620"))

    return configs

# ==========================================
# Keyboard Mapping & Injection
# ==========================================
KEY_TO_CHAR = {
    ecodes.KEY_A: ("a", "A"), ecodes.KEY_B: ("b", "B"), ecodes.KEY_C: ("c", "C"),
    ecodes.KEY_D: ("d", "D"), ecodes.KEY_E: ("e", "E"), ecodes.KEY_F: ("f", "F"),
    ecodes.KEY_G: ("g", "G"), ecodes.KEY_H: ("h", "H"), ecodes.KEY_I: ("i", "I"),
    ecodes.KEY_J: ("j", "J"), ecodes.KEY_K: ("k", "K"), ecodes.KEY_L: ("l", "L"),
    ecodes.KEY_M: ("m", "M"), ecodes.KEY_N: ("n", "N"), ecodes.KEY_O: ("o", "O"),
    ecodes.KEY_P: ("p", "P"), ecodes.KEY_Q: ("q", "Q"), ecodes.KEY_R: ("r", "R"),
    ecodes.KEY_S: ("s", "S"), ecodes.KEY_T: ("t", "T"), ecodes.KEY_U: ("u", "U"),
    ecodes.KEY_V: ("v", "V"), ecodes.KEY_W: ("w", "W"), ecodes.KEY_X: ("x", "X"),
    ecodes.KEY_Y: ("y", "Y"), ecodes.KEY_Z: ("z", "Z"), ecodes.KEY_1: ("1", "!"),
    ecodes.KEY_2: ("2", "@"), ecodes.KEY_3: ("3", "#"), ecodes.KEY_4: ("4", "$"),
    ecodes.KEY_5: ("5", "%"), ecodes.KEY_6: ("6", "^"), ecodes.KEY_7: ("7", "&"),
    ecodes.KEY_8: ("8", "*"), ecodes.KEY_9: ("9", "("), ecodes.KEY_0: ("0", ")"),
    ecodes.KEY_SPACE: (" ", " "), ecodes.KEY_MINUS: ("-", "_"),
    ecodes.KEY_EQUAL: ("=", "+"), ecodes.KEY_COMMA: (",", "<"),
    ecodes.KEY_DOT: (".", ">"), ecodes.KEY_SLASH: ("/", "?"),
    ecodes.KEY_SEMICOLON: (";", ":"), ecodes.KEY_APOSTROPHE: ("'", '"'),
    ecodes.KEY_LEFTBRACE: ("[", "{"), ecodes.KEY_RIGHTBRACE: ("]", "}"),
    ecodes.KEY_BACKSLASH: ("\\", "|"), ecodes.KEY_GRAVE: ("`", "~"),
}

CHAR_TO_KEY = {}
for kc, (normal, shifted) in KEY_TO_CHAR.items():
    CHAR_TO_KEY[normal] = (kc, False)
    CHAR_TO_KEY[shifted] = (kc, True)

UI = UInput()

def notify(summary: str, body: Optional[str] = None):
    try:
        args = ["notify-send", summary]
        if body:
            args.append(body)
        subprocess.run(args, check=False)
    except Exception:
        pass

def find_keyboard_device() -> Optional[InputDevice]:
    devs = [InputDevice(path) for path in list_devices()]
    for d in devs:
        name = (d.name or "").lower()
        if "keyboard" in name or "kbd" in name:
            return d
    for d in devs:
        caps = d.capabilities(verbose=False)
        if ecodes.EV_KEY in caps:
            return d
    return None

def emit_backspaces(n: int):
    for _ in range(n):
        UI.write(ecodes.EV_KEY, ecodes.KEY_BACKSPACE, 1)
        UI.write(ecodes.EV_KEY, ecodes.KEY_BACKSPACE, 0)
    UI.syn()

def type_text(text: str):
    for ch in text:
        entry = CHAR_TO_KEY.get(ch)
        if not entry:
            continue
        keycode, need_shift = entry
        if need_shift:
            UI.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 1)
        UI.write(ecodes.EV_KEY, keycode, 1)
        UI.write(ecodes.EV_KEY, keycode, 0)
        if need_shift:
            UI.write(ecodes.EV_KEY, ecodes.KEY_LEFTSHIFT, 0)
    UI.syn()

# ==========================================
# API Integration
# ==========================================
def _request_gemini(prompt: str, config: ProviderConfig) -> str:
    url = API_URL or f"https://generativelanguage.googleapis.com/v1beta/models/{config.model}:generateContent?key={config.api_key}"
    payload = {"contents": [{"parts": [{"text": prompt}]}]}
    r = requests.post(url, json=payload, timeout=30)
    r.raise_for_status()
    data = r.json()
    candidates = data.get("candidates") or []
    if candidates:
        content = candidates[0].get("content") or {}
        parts = content.get("parts") or []
        if parts:
            return (parts[0].get("text") or "").strip()
    return ""

def _request_openai_compatible(prompt: str, config: ProviderConfig) -> str:
    headers = {
        "Authorization": f"Bearer {config.api_key}",
        "Content-Type": "application/json",
    }
    url = API_URL or "https://api.openai.com/v1/chat/completions"
    if config.provider == "grok":
        url = API_URL or "https://api.x.ai/v1/chat/completions"
    payload = {
        "model": config.model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 2048,
    }
    r = requests.post(url, headers=headers, json=payload, timeout=30)
    r.raise_for_status()
    data = r.json()
    if "choices" in data and len(data["choices"]) > 0:
        msg = data["choices"][0].get("message") or data["choices"][0]
        content = msg.get("content") if isinstance(msg, dict) else str(msg)
        return (content or "").strip()
    return (data.get("text") or "").strip()

def _request_claude(prompt: str, config: ProviderConfig) -> str:
    url = API_URL or "https://api.anthropic.com/v1/messages"
    headers = {
        "x-api-key": config.api_key,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
    }
    payload = {
        "model": config.model,
        "max_tokens": 2048,
        "messages": [{"role": "user", "content": prompt}],
    }
    r = requests.post(url, headers=headers, json=payload, timeout=30)
    r.raise_for_status()
    data = r.json()
    content = data.get("content") or []
    if content:
        first = content[0] or {}
        if isinstance(first, dict):
            return (first.get("text") or "").strip()
    return ""

def call_provider(prompt: str, config: ProviderConfig) -> str:
    if config.provider == "gemini":
        return _request_gemini(prompt, config)
    if config.provider == "claude":
        return _request_claude(prompt, config)
    return _request_openai_compatible(prompt, config)

def _attempts_from_configs(configs: list[ProviderConfig]) -> list[ProviderAttempt]:
    grouped: dict[tuple[str, str], list[ProviderConfig]] = {}
    order: list[tuple[str, str]] = []
    for config in configs:
        key = (config.provider, config.model)
        grouped.setdefault(key, []).append(config)
        if key not in order:
            order.append(key)

    attempts: list[ProviderAttempt] = []
    for provider, model in order:
        entries = grouped[(provider, model)]
        total_keys = len(entries)
        for index, entry in enumerate(entries, start=1):
            attempts.append(
                ProviderAttempt(
                    provider=entry.provider, api_key=entry.api_key, 
                    model=entry.model, key_index=index, total_keys=total_keys
                )
            )
    return attempts

def call_ai_api_sync(text: str) -> str:
    prompt = f"Fix grammar and polish this text. Return ONLY the fixed text: {text}"
    configs = discover_provider_configs()
    
    if not configs:
        return "[Error: no API keys found in environment or config.ini]"

    last_error = None
    attempts = _attempts_from_configs(configs)
    
    for attempt_number, attempt in enumerate(attempts, start=1):
        if attempt.api_key == "PLACEHOLDER_API_KEY":
            last_error = "Using default placeholder API key. Please update your environment or config.ini."
            continue

        config = ProviderConfig(provider=attempt.provider, api_key=attempt.api_key, model=attempt.model)
        try:
            result = call_provider(prompt, config)
            if result:
                return result
            last_error = f"empty response from {config.provider} key {attempt.key_index}/{attempt.total_keys}"
        except requests.HTTPError as error:
            status = getattr(error.response, "status_code", None)
            last_error = f"{config.provider} HTTP {status} key {attempt.key_index}/{attempt.total_keys}"
            if status not in (401, 403, 429):
                break
        except requests.RequestException as error:
            last_error = f"{config.provider} network error: {error} key {attempt.key_index}/{attempt.total_keys}"
        except Exception as error:
            last_error = f"{config.provider} error: {error} key {attempt.key_index}/{attempt.total_keys}"

        time.sleep(0.2)

    return f"[Error: {last_error or 'all providers failed'}]"

def handle_trigger_async(raw_text: str):
    def worker(text_to_process: str):
        notify("Stringcast: Processing…")
        result = call_ai_api_sync(text_to_process)
        if not result:
            result = "[No output]"
        type_text(result)
        notify("Stringcast: Done")

    t = threading.Thread(target=worker, args=(raw_text,), daemon=True)
    t.start()

# ==========================================
# Main Event Loop
# ==========================================
def main():
    print("Wayland listener PoC starting")
    dev = find_keyboard_device()
    if not dev:
        print("No keyboard device found. Are you running as root or in the input group?")
        sys.exit(1)

    print(f"Listening on: {dev.path} ({dev.name})")
    typed_buffer = ""
    shift_down = False

    for event in dev.read_loop():
        if event.type != ecodes.EV_KEY:
            continue
        ev = categorize(event)
        
        if ev.keystate != 1:
            continue
        code = ev.scancode

        if code in (ecodes.KEY_LEFTSHIFT, ecodes.KEY_RIGHTSHIFT):
            shift_down = True
            continue

        if code == ecodes.KEY_BACKSPACE:
            typed_buffer = typed_buffer[:-1]
            continue

        if code in (ecodes.KEY_ENTER, ecodes.KEY_ESC):
            typed_buffer = ""
            continue

        mapping = KEY_TO_CHAR.get(code)
        if mapping:
            ch = mapping[1] if shift_down else mapping[0]
            typed_buffer += ch
        else:
            if code in (ecodes.KEY_LEFT, ecodes.KEY_RIGHT, ecodes.KEY_UP, ecodes.KEY_DOWN):
                typed_buffer = ""

        shift_down = False

        if typed_buffer.endswith(TRIGGER):
            raw_text = typed_buffer[: -len(TRIGGER)].strip()
            total_backspaces = len(raw_text) + len(TRIGGER)
            emit_backspaces(total_backspaces)
            typed_buffer = ""
            handle_trigger_async(raw_text)

if __name__ == "__main__":
    main()