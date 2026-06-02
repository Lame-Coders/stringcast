# 📝 Stringcast Wayland Listener

A Proof-of-Concept (PoC) keyboard listener that intercepts typed text, sends it to configured Large Language Model (LLM) APIs for grammar correction and polishing, and simulates pasting the improved text back into the foreground application.

This tool acts as an automated text refinement layer that hooks into your system's input stream.

## ✨ Features

*   **Global Listener:** Captures key events system-wide (requires necessary permissions).
*   **Contextual Correction:** Sends text typed up to a trigger sequence to external AI models.
*   **Multi-Provider Support:** Designed to seamlessly switch between OpenAI, Gemini, Claude, and Grok endpoints using environment variables.
*   **Failover Logic:** Attempts to use multiple API keys/endpoints sequentially if one fails (e.g., rate limiting or authentication errors).
*   **Synthetic Input:** Restores the corrected text into the active input field.

## 🚀 Getting Started

### Prerequisites

1.  **Python 3:** Ensure you have Python 3 installed.
2.  **System Dependencies:** You must have `evdev` capabilities, requiring specific libraries.
3.  **Permissions:** **Crucially, this script must be run with root privileges (`sudo`) or by a user added to the `input` group** so it can read raw input events (`/dev/input/event*`) and write simulated events (`UInput`).

### Installation

Install Python dependencies:

```bash
pip install evdev requests
```

### Configuration (Environment Variables)

The script relies heavily on environment variables to configure API access. You must set at least one valid API key for a provider you wish to use.

| Variable | Purpose | Example |
| :--- | :--- | :--- |
| `STRINGCAST_API_KEY` | A single key to test first, or a comma-separated list for fallback. | `sk-xxxx, sk-yyyy` |
| `STRINGCAST_TRIGGER` | The exact sequence that triggers the AI process. | `?fix` |
| `STRINGCAST_API_URL` | Optional. Overrides the default endpoint URL for all providers. | |
| `STRINGCAST_MODEL` | The default model name to use if specific provider models are not set. | `gpt-4o-mini` |

**Provider-Specific Keys:** The script automatically checks for standard environment variable patterns for various LLM services (e.g., `OPENAI_API_KEYS`, `GEMINI_API_KEYS`, etc.).

### Usage

1.  **Set Environment Variables:** Configure your keys and desired trigger sequence.
2.  **Execute Script:** Run the script using `sudo` or elevated permissions.

```bash
# Example: Set API keys and run (Replace with your actual keys)
export STRINGCAST_API_KEY="YOUR_MAIN_API_KEY"
export STRINGCAST_TRIGGER=";done"

# Run the listener
sudo python3 scripts/wayland_listener.py
```

> **⚠️ Warning:** Running this script gives it the ability to type anything anywhere on your system. Use it responsibly.

## 📚 Deep Dive & Advanced Topics

### Input Handling

*   **Mapping:** The script uses `KEY_TO_CHAR` to map physical keycodes (`ecodes`) to characters. It supports basic ASCII letters, numbers, and common punctuation.
*   **Shift State:** It tracks `shift_down` to correctly emit both lowercase and uppercase characters.
*   **Buffer Management:** When the trigger is hit, it accurately simulates backspacing to clear the buffer and resets the internal state.

### AI Processing Pipeline

1.  **Provider Discovery:** `discover_provider_configs()` scans all defined environment variables to build a ranked list of available API access points.
2.  **Execution Order:** The script iterates through the providers in a defined fallback order (OpenAI -> Gemini -> etc.).
3.  **Prompting:** The system prompt is hardcoded to: *"Fix grammar and polish this text. Return ONLY the fixed text: {raw_text}"*
4.  **Error Handling:** It is robust against common API errors (401 Unauthorized, 429 Rate Limit) and network timeouts, ensuring it moves to the next provider rather than crashing.

## 🛠️ Known Limitations & Future Improvements

*   **Scope:** This is a PoC. Current character mapping is minimal (ASCII focused).
*   **UI Integration:** The text injection works via simulating key presses, which is generally reliable but can sometimes be interrupted by OS-level focus changes.
*   **Performance:** Due to the network calls, processing time depends entirely on API latency.
*   **Feature Gaps:** (List any planned features here, e.g., "Support for markdown formatting," or "GUI integration.")