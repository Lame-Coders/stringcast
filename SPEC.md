# Stringcast Desktop — Full Product Specification

**Version:** 1.0.1-draft  
**Target Platforms:** macOS 12+, Windows 10+  
**Language:** Rust (2021 edition)  
**Status:** Pre-development  

---

## Table of Contents

1. [Overview](#1-overview)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [System Architecture](#3-system-architecture)
4. [Core Subsystems](#4-core-subsystems)
   - 4.1 [Global Input Capture](#41-global-input-capture)
   - 4.2 [Keystroke Buffer](#42-keystroke-buffer)
   - 4.3 [Trigger Detection Engine](#43-trigger-detection-engine)
   - 4.4 [Text Extraction](#44-text-extraction)
   - 4.5 [AI API Client](#45-ai-api-client)
   - 4.6 [Inline Replacement Engine](#46-inline-replacement-engine)
   - 4.7 [Undo System](#47-undo-system)
   - 4.8 [Operation State Machine](#48-operation-state-machine)
5. [Command System](#5-command-system)
   - 5.1 [Built-in Commands](#51-built-in-commands)
   - 5.2 [Dynamic Commands](#52-dynamic-commands)
   - 5.3 [Custom Commands](#53-custom-commands)
   - 5.4 [Command Resolution Order](#54-command-resolution-order)
6. [API Provider System](#6-api-provider-system)
   - 6.1 [Supported Providers](#61-supported-providers)
   - 6.2 [Key Rotation](#62-key-rotation)
   - 6.3 [Rate Limit & Error Handling](#63-rate-limit--error-handling)
   - 6.4 [Request Construction](#64-request-construction)
   - 6.5 [Response Parsing](#65-response-parsing)
7. [Secure Key Storage](#7-secure-key-storage)
8. [Settings & Persistence](#8-settings--persistence)
9. [System Tray UI](#9-system-tray-ui)
10. [Settings UI (Tauri Window)](#10-settings-ui-tauri-window)
11. [Visual Feedback (Spinner)](#11-visual-feedback-spinner)
12. [Exclusion List (Per-App Blocking)](#12-exclusion-list-per-app-blocking)
13. [Platform-Specific Details](#13-platform-specific-details)
    - 13.1 [macOS](#131-macos)
    - 13.2 [Windows](#132-windows)
14. [Edge Cases & Failure Modes](#14-edge-cases--failure-modes)
15. [Security Model](#15-security-model)
16. [Performance Constraints](#16-performance-constraints)
17. [Crate & Dependency Reference](#17-crate--dependency-reference)
18. [Project Structure](#18-project-structure)
19. [Build & Release](#19-build--release)
20. [Testing Strategy](#20-testing-strategy)
21. [Future Roadmap](#21-future-roadmap)

---

## 1. Overview

**Stringcast Desktop** is a system-wide, background AI text-transformation tool for macOS and Windows. It runs as a tray application, monitors keystrokes globally, and when a user types a trigger suffix (e.g. `?fix`, `?formal`, `?translate:es`) anywhere — in any application — it replaces the text in-place with an AI-enhanced version.

The core user flow is:

```
User types  →  "i dont no whats hapening ?fix"
                                            ↑ trigger detected
Stringcast  →  selects all text  →  sends to AI  →  pastes result
Result      →  "I don't know what's happening."
```

No copy-pasting. No app switching. Works in every text field: browsers, email clients, terminals, IDEs, chat apps, notes apps.

---

## 2. Goals & Non-Goals

### Goals

- System-wide text transformation triggered by typed suffixes
- Works in **any application** that accepts keyboard input
- Cross-platform: macOS and Windows from a single Rust codebase
- Support for multiple AI providers (Gemini, OpenAI, Claude, any OpenAI-compatible endpoint)
- Encrypted API key storage using OS-native credential vaults
- Custom user-defined trigger → prompt pairs
- Multi-key round-robin rotation with automatic rate-limit handling
- Undo: restore previous text after a replacement
- Lightweight: < 15 MB binary, < 30 MB RAM idle
- Zero telemetry, zero analytics, no intermediary servers
- Predictable failure behavior: never leave a text field blank, selected, or containing a spinner after an error

### Non-Goals

- Reading text from arbitrary text fields directly (not supported without accessibility APIs; clipboard-based approach used instead)
- Image or file transformation
- Voice input
- Mobile platforms (Android/iOS)
- Clipboard manager / history (beyond the single undo slot)
- A full accessibility tree inspector
- Perfect compatibility with every secure, custom-rendered, remote-desktop, game, or terminal input surface

---

### MVP Acceptance Criteria

Stringcast v1.0 is considered shippable only when all of the following are true:

- A first-time user can install the app, grant required permissions, add one valid API key, and run `?fix` in a supported text field without reading external documentation.
- For a normal text field, trigger execution leaves the user's clipboard restored to its pre-operation content.
- If extraction, API, paste, or verification fails, the original field text is restored or left unchanged, and the user receives a clear notification.
- Programmatically generated keystrokes and paste operations never re-trigger Stringcast.
- Excluded apps, password manager apps, and detectable secure input fields do not trigger clipboard reads or API calls.
- API keys are never written to plaintext config files or logs.
- The app can be paused globally from the tray and resumed without restart.
- Manual E2E checklist in §20 passes on at least one current macOS release and one current Windows release before public release.

---

## 3. System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Stringcast Process                      │
│                                                             │
│  ┌──────────────┐    ┌─────────────────┐    ┌───────────┐  │
│  │ Global Input │───▶│ Keystroke Buffer │───▶│  Trigger  │  │
│  │   Capture    │    │  + Cursor Track  │    │ Detector  │  │
│  │  (rdev)      │    └─────────────────┘    └─────┬─────┘  │
│  └──────────────┘                                 │        │
│                                                   ▼        │
│  ┌──────────────┐    ┌─────────────────┐    ┌───────────┐  │
│  │   Inline     │◀───│   AI API Client │◀───│   Text    │  │
│  │  Replacer    │    │  (reqwest/tokio) │    │ Extractor │  │
│  │  (enigo)     │    └─────────────────┘    └───────────┘  │
│  └──────────────┘                                          │
│         │                                                   │
│  ┌──────▼───────┐    ┌─────────────────┐    ┌───────────┐  │
│  │  Undo Store  │    │  Key/Config     │    │ Tray + UI │  │
│  │  (1 slot)    │    │  Store (keyring)│    │  (Tauri)  │  │
│  └──────────────┘    └─────────────────┘    └───────────┘  │
└─────────────────────────────────────────────────────────────┘
```

All subsystems run in the same process. The global input capture runs on a dedicated thread. The AI calls are dispatched via `tokio` async runtime. The Tauri UI runs on the main thread. Inter-thread communication uses `tokio::sync::mpsc` channels.

---

## 4. Core Subsystems

### 4.1 Global Input Capture

**Crate:** `rdev`

Stringcast registers a global keyboard listener that fires on every `KeyPress` and `KeyRelease` event, system-wide. This runs on a dedicated OS thread (required by `rdev`).

#### Event source suppression

Keyboard events generated by Stringcast itself must be ignored by the input pipeline. Before sending any synthetic select/copy/paste/typing event, the replacement or extraction engine sets a process-local `synthetic_input_guard` flag with a monotonically increasing operation ID. The global input listener drops events while the guard is active and for a short grace window after release.

| Parameter | Value |
|---|---|
| Guard scope | Current operation only |
| Grace window after synthetic input | 250 ms |
| On guard leak or panic | Clear guard via `Drop`; watchdog clears after 10 seconds |

This prevents spinner frames, pasted AI output, `Ctrl+A`, `Ctrl+C`, `Ctrl+V`, and character-simulation fallback from being appended to the keystroke buffer or triggering another command.

#### Events captured

| Event | Action |
|---|---|
| `KeyPress(Key::*)` | Feed character to buffer; check for trigger |
| `KeyPress(Backspace)` | Pop last char from buffer |
| `KeyPress(Delete)` | Clear buffer (cursor moved) |
| `KeyPress(Return / Enter)` | Clear buffer |
| `KeyPress(Escape)` | Clear buffer |
| `KeyPress(Arrow*)` | Clear buffer (cursor position unknown) |
| `KeyPress(Home / End / PgUp / PgDn)` | Clear buffer |
| Mouse button press | Clear buffer |
| `KeyPress(Space)` | Append space to buffer; check for trigger |

#### Characters appended to buffer

Only printable characters (Unicode scalar values with display glyphs) are appended. Modifier-only keypresses (Ctrl, Alt, Cmd/Win, Fn) are ignored unless combined with a printable key that produces a character.

#### Modifier state tracking

A bitmask tracks currently held modifiers: `SHIFT`, `CTRL/CMD`, `ALT/OPTION`, `WIN/META`. Combinations like `Ctrl+A` (select all), `Ctrl+C`, `Ctrl+V`, `Ctrl+Z` are detected and cause a **buffer clear** rather than appending a character, because they indicate a non-typing user action.

`AltGr` on international Windows keyboard layouts must be treated as a text-entry modifier, not as `Ctrl+Alt`, when it produces printable characters. Dead-key composition should append only the final composed character where available; intermediate dead-key events clear neither the buffer nor trigger detection unless the platform reports them as cursor/navigation actions.

#### Fast-exit optimization

Before any full trigger scan, check: does the last character of the buffer match the last character of **any** known trigger? If no: skip scan entirely. This is O(1) per keystroke for the common case.

Dynamic commands are an exception: once the buffer tail contains a reserved dynamic prefix such as `?translate:` or `?ask:`, detection continues until the dynamic trigger is finalized or cancelled, because the final parameter character is not known in advance.

---

### 4.2 Keystroke Buffer

The keystroke buffer is a `String` maintained in memory (UTF-8). It represents a best-effort reconstruction of what the user has typed since the last buffer-clearing event.

#### Buffer limits

| Parameter | Value |
|---|---|
| Maximum buffer length | 8192 bytes |
| On overflow | Drop oldest characters (ring behaviour), keep tail |

#### Buffer invalidation events

The buffer is fully cleared on:
- Arrow key press (any direction)
- Home, End, Page Up, Page Down
- Mouse click (any button, detected via `rdev` mouse events)
- `Escape`
- `Enter` / `Return`
- `Tab` (unless part of typed text — Tab clears buffer)
- Application focus change (where detectable; see platform notes)
- Successful trigger execution (buffer is cleared after replacement)
- Manual "undo" trigger execution
- System sleep, screen lock, unlock, or user session switch
- Clipboard-only paste shortcuts (`Ctrl+V` / `Cmd+V`) and cut shortcuts (`Ctrl+X` / `Cmd+X`)

#### Backspace handling

On `Backspace`: if buffer is non-empty, pop the last Unicode character (not byte — handle multi-byte correctly). If buffer is empty, no-op.

Backspace after a combining mark removes the last Unicode scalar value. Full grapheme-cluster deletion is a future enhancement; trigger detection must remain correct for ASCII trigger suffixes even when preceding text contains combining marks.

#### Composition input (IME)

Input Method Editor (IME) input (used for CJK, etc.) produces `KeyPress` events that may not map 1:1 to buffer characters. When `rdev` reports a composed character event, append the composed string. On IME composition start/cancel: clear the buffer. This is a known limitation — IME workflows may produce garbled buffers; the trigger scan will simply not match and no action is taken.

---

### 4.3 Trigger Detection Engine

After every character append to the buffer, run trigger detection:

#### Algorithm

```
1. If static fast-exit check fails and no dynamic prefix is pending → return (no match)
2. For each registered trigger T (sorted by length, longest first):
   a. If buffer ends with T → match found
3. If no static trigger matched, evaluate dynamic trigger prefixes:
   a. Find the last occurrence of ?translate: or ?ask:
   b. Validate the parameter grammar for that command
   c. Require dynamic trigger finalization (see below)
4. On match:
   a. Extract content = buffer[0 .. buffer.len() - T.len()]
   b. Trim trailing whitespace from content
   c. If content is empty → abort (nothing to transform)
   d. Dispatch transformation (content, command)
```

#### Longest-match rule

If the buffer ends with `?translate:es` and also ends with `?translate`, the longer match (`?translate:es`) wins. Triggers are always evaluated longest-first.

#### Case sensitivity

Triggers are **case-sensitive** by default. `?Fix` does not match `?fix`. (This is configurable per-command in the data model; defaults to case-sensitive.)

#### Trigger format

All built-in triggers start with `?`. Custom triggers must also start with `?`. The `?` prefix is enforced at command creation time in the UI. Maximum trigger length: 64 characters. Allowed characters: `?`, `a-z`, `A-Z`, `0-9`, `:`, `-`, `_`.

#### Dynamic trigger finalization

Dynamic triggers have open-ended parameters, so they must not execute immediately when the shortest valid parameter is reached. For example, `?translate:es` might become `?translate:es-MX`, and `?ask:sum` might become a longer instruction.

Dynamic commands execute only when one of these finalization conditions is met:

| Condition | Handling |
|---|---|
| User stops typing for 650 ms after a syntactically valid dynamic trigger | Execute the dynamic command |
| User types a trailing space after a valid `?translate:<lang>` trigger | Trim that trailing space from the trigger and execute |
| User types spaces inside a `?ask:<question>` trigger | Treat spaces as part of the question; execution still waits for debounce |
| User presses `Escape`, arrow keys, mouse, or changes focus before debounce completes | Cancel the pending dynamic trigger and clear the buffer |
| User continues typing before debounce completes | Restart the debounce timer with the updated parameter |

Static built-in and custom triggers execute immediately. Only dynamic triggers use debounce finalization.

#### Trigger boundary rules

The trigger must be the final token in the field snapshot after trimming trailing spaces. The text before the trigger is the transformation input. A literal trigger can be typed without execution by escaping the leading question mark as `\?fix`; the backslash is left in the field and no command runs.

---

### 4.4 Text Extraction

Because Stringcast cannot directly read the contents of an arbitrary text field on desktop (unlike Android's Accessibility Service), it uses a **select-all + clipboard read** strategy.

#### Extraction flow

```
1. Save current clipboard content (to restore after)
2. Send Ctrl+A (Win) / Cmd+A (Mac) — select all in active field
3. Send Ctrl+C (Win) / Cmd+C (Mac) — copy selection
4. Wait up to 300 ms for clipboard to populate
5. Read clipboard text
6. Restore original clipboard content
7. Detect trigger in clipboard text → extract prefix
```

The extractor returns an immutable `OperationSnapshot`:

```rust
struct OperationSnapshot {
    operation_id: Uuid,
    app_id: String,
    window_id: Option<String>,
    extracted_text: String,
    transform_input: String,
    trigger_text: String,
    original_clipboard: ClipboardSnapshot,
    started_at: Instant,
}
```

The snapshot is the only source used by the API client, spinner, replacement engine, and undo system. Later keystroke-buffer changes must not mutate an in-flight operation.

#### Why not use the buffer directly?

The keystroke buffer is an approximation. The user may have pasted text, used autocomplete, or edited with the mouse. The clipboard read gets the **actual** current field contents, which is what we need to send to the AI.

#### Fallback: buffer-only extraction

If the clipboard read fails or returns empty, fall back to the keystroke buffer text (minus the trigger). Log this as a degraded extraction.

#### Multi-line fields

The `Ctrl+A / Cmd+A` approach selects all text in a single-line field. In a multi-line field (textarea, IDE, terminal), it selects all text in the entire field — which may be very large. Mitigations:

- Cap extracted text at **16,384 characters** before sending to AI. Truncate from the front (keep recent context).
- Single-line hint: if extracted text contains no newlines, assume single-line field behavior is correct.
- Multi-line safety: user can configure a max-token-budget per request to avoid large API costs.
- Preserve the trigger removal boundary after truncation. If the field exceeds the extraction cap, first remove the trigger from the full copied text, then truncate the remaining transform input from the front.

#### Existing selections

If the user has an active selection before typing the trigger, the selection is normally replaced by the user's typed trigger before Stringcast sees it. Stringcast does not attempt to preserve historical selections. At trigger time, the operation scope is the full text selected by `Ctrl+A / Cmd+A`.

#### Clipboard ownership and races

Clipboard access is serialized through a single in-process mutex. If another application changes the clipboard while Stringcast is processing:

- During extraction: prefer the text copied from the active field only if clipboard change time occurs after Stringcast's copy shortcut and the content contains the detected trigger.
- During spinner or replacement: restore the original clipboard only if the clipboard still exactly matches the Stringcast-owned spinner frame or AI output for the active operation.
- If ownership is ambiguous: do not overwrite the clipboard; log a warning and notify only on debug builds.

#### Apps that block Ctrl+A / Cmd+A

Some apps (games, kiosk apps, terminals with custom bindings) intercept or ignore the select-all shortcut. In these cases the clipboard will either be unchanged or empty. If clipboard content equals the pre-extraction clipboard (i.e. unchanged), or is empty after the timeout:

- Abort the operation silently (do not flash an error for the user in mid-keystroke)
- Log a warning internally
- Add the app to a "known problematic" list visible in settings

---

### 4.5 AI API Client

**Crate:** `reqwest` (async, TLS enabled), `tokio` (runtime)

#### Request lifecycle

```
1. Select next API key (round-robin, skip rate-limited/invalid keys)
2. Build prompt = system_prompt + user_text
3. POST request to provider endpoint
4. Stream or await response
5. Parse and clean response text
6. Return to replacement engine
```

#### Timeout

- Connection timeout: 5 seconds
- Response timeout: 30 seconds (streaming: 60 seconds)
- On timeout: attempt retry with next key (if available), else show error notification

#### Retry policy

| Condition | Action |
|---|---|
| HTTP 429 (rate limited) | Mark key as rate-limited, rotate to next key, retry immediately |
| HTTP 401 / 403 | Mark key as invalid, rotate to next key, retry once |
| HTTP 5xx | Retry after 1s back-off, up to 2 retries, then fail |
| Network error | Retry once immediately, then fail |
| Timeout | Rotate key, retry once, then fail |

Maximum total retries per trigger invocation: **3** (across all keys). After 3 failures, surface an error notification to the user and restore the original text.

#### Response cleaning

AI responses often include preamble, markdown, or commentary. Apply these cleaning steps in order:

1. Strip leading/trailing whitespace
2. If the entire response is wrapped in triple backticks (code block), remove the fences and language hint
3. Strip common preamble phrases: lines starting with "Sure,", "Here is", "Here's", "Certainly,", "Of course," — remove that opening line
4. Strip trailing meta-commentary: lines starting with "Note:", "Please note", "I have", "The text" — remove those trailing lines
5. Final whitespace trim

This cleaning is applied to all responses unless the command explicitly sets `raw_output: true`.

---

### 4.6 Inline Replacement Engine

**Crate:** `enigo`

After the AI response is received, replace the field content:

#### Replacement flow

```
1. Verify the original app/window is still foreground and not excluded
2. Store `OperationSnapshot.extracted_text` in undo slot (see §4.7)
3. Set clipboard to AI response text
4. Send Ctrl+A (Win) / Cmd+A (Mac) — select all
5. Send Ctrl+V (Win) / Cmd+V (Mac) — paste
6. Wait 100 ms for paste to settle
7. Restore original clipboard content if clipboard ownership is still Stringcast-owned
8. Clear keystroke buffer
```

#### Clipboard restoration timing

The user's original clipboard is saved before extraction and restored after paste. The restoration must happen **after** the paste has settled (at least 100 ms), otherwise the paste may pick up the restored clipboard value. Use `tokio::time::sleep(Duration::from_millis(150))` before restore.

#### Apps that block Ctrl+V / Cmd+V paste

Some apps (certain terminals, games, password fields) block programmatic paste. Detection: after paste, attempt to verify by doing a `Ctrl+A / Cmd+A` + clipboard read and comparing against expected output. If mismatch:

- Try `enigo` character-by-character simulation as fallback (slow but works in most apps)
- Character simulation rate: 5 ms per character (fast enough to be instant for typical responses, avoids triggering hold-key repeat)
- Cap character simulation at 2000 characters; beyond that, show a "paste manually" notification with the text on the clipboard

#### Password fields

Do not transform text in password fields. Detection heuristics (imperfect on desktop):

- Check if the app has "password" in its accessibility label (macOS: via AXRole, Windows: via UIA)
- Maintain default exclusions for known password managers and OS credential tools
- If the active field or app cannot be inspected and the app is known to use secure input, abort before clipboard access
- If unable to determine after all checks, continue only outside the default exclusion list; this limitation must be documented in onboarding

---

### 4.7 Undo System

Stringcast maintains a single-slot undo per session (not persisted across restarts).

#### Undo trigger

The trigger `?undo` is a special built-in command. It does not call the AI API. On detection:

```
1. Check undo slot: if empty → show notification "Nothing to undo"
2. If populated:
   a. Save current clipboard snapshot
   b. Set clipboard to saved original text
   c. Ctrl+A + Ctrl+V to replace
   d. Restore previous clipboard if still Stringcast-owned
   e. Clear undo slot
   f. Clear keystroke buffer
```

#### Undo slot contents

The undo slot stores:
- `original_text: String` — the full text before the last transformation
- `replaced_text: String` — the AI output (for potential re-do, future feature)
- `timestamp: Instant` — for diagnostics

#### Undo slot eviction

The undo slot is cleared:
- After `?undo` is used
- On application exit
- After 10 minutes of inactivity (to avoid stale undo from old context)

Only one level of undo is supported. A second transformation overwrites the undo slot.

---

### 4.8 Operation State Machine

Every trigger invocation is represented by one operation with a stable `operation_id`. Only one operation may actively control the clipboard and active text field at a time.

#### States

```
Idle
  -> PendingDynamicTrigger
  -> Extracting
  -> CallingApi
  -> Replacing
  -> Verifying
  -> Completed
  -> Failed
  -> Cancelled
```

#### State rules

| State | Entry condition | Exit condition |
|---|---|---|
| `PendingDynamicTrigger` | Dynamic prefix matched and debounce started | Debounce completes, user cancels, or input changes |
| `Extracting` | Static trigger matched or dynamic debounce completed | Snapshot produced or extraction fails |
| `CallingApi` | Valid snapshot and command resolved | API response, API failure, timeout, or cancellation |
| `Replacing` | AI output accepted | Paste or character-simulation attempt completes |
| `Verifying` | Replacement was attempted | Field matches expected output, verification unsupported, or mismatch handled |
| `Completed` | Replacement verified or accepted | Clear active operation |
| `Failed` | Recoverable failure occurred | Restore original text where possible, then clear active operation |
| `Cancelled` | Focus/app changed, excluded app became active, or user paused app | Restore original text only when safe, then clear active operation |

#### Concurrency policy

- Default policy: one active operation, one queued operation.
- If a second trigger fires while an operation is in `CallingApi`, queue the latest trigger and show a tray "queued" state.
- If a third trigger fires before the first completes, drop the older queued operation and keep only the newest one.
- If the user disables Stringcast while an operation is active, cancel the active operation and clear the queue.
- Queued operations must re-run extraction when they start; they must not reuse stale text snapshots.

#### Cancellation safety

An operation must cancel without changing the text field when:

- The foreground app or window changes before replacement starts.
- The active app becomes excluded before replacement starts.
- The user presses `Escape` while an operation is pending or processing.
- The system sleeps, locks, or switches user sessions.

If replacement or spinner has already modified the field, cancellation attempts to restore `OperationSnapshot.extracted_text` only when the original app/window is still foreground and the field currently contains the Stringcast-owned spinner or expected intermediate value.

---

## 5. Command System

### 5.1 Built-in Commands

Built-in commands are compiled into the binary. They cannot be deleted but can be **disabled** per-user via settings. All use the placeholder `{text}` in their prompt to represent the extracted user text.

| Trigger | Name | Prompt Sent to AI |
|---|---|---|
| `?fix` | Fix Grammar | `Fix all grammar, spelling, and punctuation errors in the following text. Return ONLY the corrected text, nothing else:\n\n{text}` |
| `?improve` | Improve | `Improve the clarity, flow, and readability of the following text while preserving its meaning and tone. Return ONLY the improved text, nothing else:\n\n{text}` |
| `?shorten` | Shorten | `Shorten the following text to its most essential meaning without losing key information. Return ONLY the shortened text, nothing else:\n\n{text}` |
| `?expand` | Expand | `Expand the following text with relevant detail, context, and supporting explanation. Return ONLY the expanded text, nothing else:\n\n{text}` |
| `?formal` | Make Formal | `Rewrite the following text in a professional, formal tone suitable for business communication. Return ONLY the rewritten text, nothing else:\n\n{text}` |
| `?casual` | Make Casual | `Rewrite the following text in a friendly, casual, conversational tone. Return ONLY the rewritten text, nothing else:\n\n{text}` |
| `?emoji` | Add Emojis | `Add relevant and tasteful emojis to the following text to make it more expressive. Return ONLY the text with emojis added, nothing else:\n\n{text}` |
| `?reply` | Generate Reply | `Generate a natural, contextually appropriate reply to the following message. Return ONLY the reply text, nothing else:\n\n{text}` |
| `?bullets` | Bullet Points | `Convert the following text into a concise, well-structured bullet-point list. Return ONLY the bullet points, nothing else:\n\n{text}` |
| `?summarize` | Summarize | `Write a concise summary of the following text in 1–3 sentences. Return ONLY the summary, nothing else:\n\n{text}` |
| `?undo` | Undo | *(special — no API call; see §4.7)* |

### 5.2 Dynamic Commands

Dynamic commands take a parameter embedded in the trigger itself.

#### `?translate:<lang>`

**Pattern:** `?translate:` followed by a BCP-47 language code (2–8 alphanumeric characters and hyphens)

**Regex:** `^\?translate:[a-zA-Z]{2,8}(-[a-zA-Z0-9]{2,8})*$`

**Prompt:**
```
Translate the following text to {language_name} (language code: {lang_code}). 
Return ONLY the translated text, nothing else:

{text}
```

Language name is resolved from a bundled BCP-47 → display name lookup table (covering the top 100 languages). Unknown codes are passed as-is with a note to the AI.

**Examples:**
- `?translate:es` → Spanish
- `?translate:zh-Hant` → Traditional Chinese
- `?translate:hi` → Hindi

#### `?ask:<question>`

**Pattern:** `?ask:` followed by any text (the question/instruction)

**Prompt:**
```
Given the following text:

{text}

{question}

Return ONLY your response, without any preamble or commentary.
```

**Example:** `quarterly report ?ask:what are the 3 most important metrics here`

**Validation:** The question portion must be at least 3 characters and at most 256 characters. If the question is empty, treat as a malformed trigger and ignore.

#### Future dynamic commands (reserved, not in v1.0)

- `?tone:<adjective>` — rewrite in a specified tone
- `?lang:<code>` — alias for translate

### 5.3 Custom Commands

Users can define their own trigger → prompt pairs in the Settings UI.

#### Custom command schema

```toml
[[commands.custom]]
trigger = "?poem"
name = "Make it Poetic"
prompt = "Rewrite the following text as a short poem. Return ONLY the poem:\n\n{text}"
enabled = true
case_sensitive = true
raw_output = false
created_at = "2024-01-01T00:00:00Z"
```

#### Constraints

| Field | Constraint |
|---|---|
| `trigger` | Must start with `?`; 2–64 chars; pattern `[?][a-zA-Z0-9:_-]+`; may override static built-in triggers only after explicit user confirmation |
| `name` | 1–64 characters |
| `prompt` | Must contain `{text}` placeholder; max 4096 characters |
| `enabled` | Boolean; disabled commands are skipped in detection |

#### Collision handling

If a custom trigger matches a built-in trigger exactly, the custom trigger **overrides** the built-in for that trigger string. A warning is shown in the UI: "This trigger overrides the built-in `?fix` command."

Custom commands cannot override reserved dynamic prefixes (`?translate:` and `?ask:`), `?undo`, or any future reserved prefix listed in this document. The UI must reject custom triggers that start with a reserved dynamic prefix, even if the full string differs.

#### Import/Export

Custom commands can be exported to a `.toml` file and imported from one. Import validates all fields and rejects invalid entries with per-entry error messages.

### 5.4 Command Resolution Order

When a trigger is detected:

1. Check if the active app is in the exclusion list → abort
2. Check if Stringcast is globally enabled → abort if disabled
3. Match against **custom static commands** first (user-defined takes priority)
4. Match against **reserved special commands** (`?undo`)
5. Match against **built-in static commands**
6. Match against **dynamic command patterns** (`?translate:*`, `?ask:*`)
7. If no match → ignore (this shouldn't happen if the fast-exit passed, but guard it)

Dynamic trigger strings are exempt from the 64-character static trigger length limit. Their command prefix is limited to 64 characters, and the parameter limit is defined by each dynamic command.

---

## 6. API Provider System

### 6.1 Supported Providers

#### Google Gemini (default)

- Endpoint: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={api_key}`
- Auth: API key as query parameter
- Default model: `gemini-2.0-flash-lite`
- Available models (user-selectable): `gemini-2.0-flash-lite`, `gemini-2.5-flash`, `gemini-2.5-pro`
- Request format: Gemini `generateContent` JSON body
- Response path: `candidates[0].content.parts[0].text`

#### OpenAI

- Endpoint: `https://api.openai.com/v1/chat/completions`
- Auth: `Authorization: Bearer {api_key}`
- Default model: `gpt-4o-mini`
- Available models: any string (user-configurable)
- Request/response: OpenAI Chat Completions format

#### Anthropic Claude

- Endpoint: `https://api.anthropic.com/v1/messages`
- Auth: `x-api-key: {api_key}`, `anthropic-version: 2023-06-01`
- Default model: `claude-haiku-4-5`
- Request/response: Anthropic Messages API format

#### Custom OpenAI-Compatible

- Endpoint: user-defined base URL + `/chat/completions`
- Auth: `Authorization: Bearer {api_key}`
- Model: user-defined string
- Request/response: OpenAI Chat Completions format
- Use case: local models (Ollama, LM Studio), Azure OpenAI, Groq, Mistral, etc.

### 6.2 Key Rotation

Each provider maintains an independent key pool. Keys are used in round-robin order.

#### State per key

```rust
struct ApiKey {
    id: Uuid,
    provider: Provider,
    value: SecretString,        // decrypted in-memory only
    alias: Option<String>,      // user-friendly label
    status: KeyStatus,
    rate_limit_until: Option<Instant>,
    consecutive_errors: u8,
    last_used: Option<Instant>,
    requests_total: u64,
    requests_success: u64,
}

enum KeyStatus {
    Active,
    RateLimited,   // temporary; expires at rate_limit_until
    Invalid,       // permanent until user re-validates
    Disabled,      // manually disabled by user
}
```

#### Rotation algorithm

```
fn next_available_key(keys: &[ApiKey]) -> Option<&ApiKey> {
    let now = Instant::now();
    keys.iter()
        .filter(|k| k.status == Active || 
                    (k.status == RateLimited && k.rate_limit_until < Some(now)))
        .min_by_key(|k| k.last_used)  // pick least-recently-used
}
```

When a key returns HTTP 429:
- Set `status = RateLimited`
- Parse `Retry-After` header if present; otherwise set cooldown to 60 seconds
- Immediately try next key

### 6.3 Rate Limit & Error Handling

| HTTP Status | Meaning | Action |
|---|---|---|
| 200 | Success | Parse response, return |
| 400 | Bad request | Log provider error metadata only, surface error to user, do not retry |
| 401 | Unauthorized | Mark key invalid, try next key |
| 403 | Forbidden | Mark key invalid, try next key |
| 429 | Rate limited | Mark key rate-limited, try next key |
| 500, 502, 503 | Server error | Retry after 1s, up to 2 times |
| 504 | Gateway timeout | Retry once, then fail |
| Connection refused | Network issue | Retry once after 500 ms |
| DNS failure | No network | Fail immediately, show "No network" notification |

#### All keys exhausted

If all keys are rate-limited or invalid:
- Surface a system notification: "Stringcast: All API keys unavailable. Check Settings."
- Restore the original text (undo the selection)
- Do not paste anything

#### Provider-side content and quota failures

Providers may reject otherwise valid text for safety, quota, billing, context-length, model-retirement, or regional-availability reasons. These errors are not retried with the same key unless the provider explicitly marks them transient.

| Provider failure | Handling |
|---|---|
| Context length exceeded | Retry once with input truncated to `max_extract_chars / 2`; if still rejected, restore original text and notify |
| Safety/content block | Restore original text and notify: "Provider refused this text" |
| Quota or billing exhausted | Mark key unavailable until next app restart or manual revalidation; try next key |
| Model not found or retired | Mark provider config invalid; show Settings CTA to choose another model |
| Region unavailable | Treat as provider config error; do not retry automatically |

### 6.4 Request Construction

#### System prompt strategy

Each command's prompt is split into:
- **System message**: The instruction ("Fix grammar and spelling. Return ONLY the corrected text, nothing else.")
- **User message**: The actual text content

This two-part structure works better across providers and avoids AI adding commentary.

No provider request path may log raw `system_prompt`, `user_text`, API keys, or full response bodies. Debug logs may include provider name, model, trigger name, character counts, latency, status code, and a redacted error code.

#### Model defaults and validation

Default model strings are configuration defaults, not hard dependencies. On startup and when Settings opens, Stringcast should validate that the selected model is usable by sending a lightweight provider-specific validation request. If validation fails because the model no longer exists or the account cannot access it, the app must keep the user's configured value but mark it invalid in the UI and block new transformations until the user chooses a valid model.

#### Gemini request body

```json
{
  "contents": [{
    "role": "user",
    "parts": [{"text": "{user_text}"}]
  }],
  "systemInstruction": {
    "parts": [{"text": "{system_prompt}"}]
  },
  "generationConfig": {
    "temperature": 0.3,
    "maxOutputTokens": 2048,
    "candidateCount": 1
  },
  "safetySettings": [
    {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"},
    {"category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE"},
    {"category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE"},
    {"category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE"}
  ]
}
```

Safety settings are set to `BLOCK_NONE` to avoid blocking legitimate text-editing tasks (e.g. fixing a message about violence in a news article).

#### OpenAI / Compatible request body

```json
{
  "model": "{model}",
  "messages": [
    {"role": "system", "content": "{system_prompt}"},
    {"role": "user", "content": "{user_text}"}
  ],
  "temperature": 0.3,
  "max_tokens": 2048,
  "n": 1
}
```

#### Anthropic request body

```json
{
  "model": "{model}",
  "max_tokens": 2048,
  "system": "{system_prompt}",
  "messages": [
    {"role": "user", "content": "{user_text}"}
  ]
}
```

### 6.5 Response Parsing

Each provider has a dedicated parser:

```rust
trait ProviderResponseParser {
    fn parse(&self, body: &str) -> Result<String, ApiError>;
}
```

Parsers extract the text content and return a `String`. All parsers apply response cleaning (§4.5) after extraction.

On JSON parse failure: return `ApiError::MalformedResponse`; debug builds may log a redacted raw body truncated to 512 chars when explicitly enabled.

The raw body log must pass through the same redaction pipeline as all other logs and must be disabled unless `log_level = "debug"`. In normal builds, store only the provider error type, status code, and body length.

---

## 7. Secure Key Storage

**Crate:** `keyring` (cross-platform OS credential store)

API keys are **never stored in plaintext** on disk. They are stored in the OS-native credential vault:

| Platform | Backend |
|---|---|
| macOS | macOS Keychain (via Security framework) |
| Windows | Windows Credential Manager (DPAPI) |

#### Key naming scheme

Keys are stored under service name `stringcast` with a user label of the form:

```
stringcast::provider::{provider_name}::key::{key_uuid}
```

#### In-memory handling

- Keys are decrypted into `SecretString` (from the `secrecy` crate) for the duration of a request
- Keys are zeroed from memory after use
- Keys are never logged (log sanitization strips any string matching an API key pattern: `[A-Za-z0-9_-]{20,}`)

#### Key metadata (stored in config file, not keychain)

```toml
[[api_keys]]
id = "550e8400-e29b-41d4-a716-446655440000"
provider = "gemini"
alias = "Personal Key"
status = "Active"
created_at = "2024-01-15T10:00:00Z"
```

The keychain stores only the raw key value; all metadata is stored in the config file.

#### Key validation

On key addition:
1. Make a minimal test request to the provider (e.g. "Say hi" → expect 200)
2. Show validation status in UI (✅ Valid / ❌ Invalid / ⏳ Checking...)
3. Only save if valid (or allow saving unvalidated with a warning)

---

## 8. Settings & Persistence

### Config file location

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/Stringcast/config.toml` |
| Windows | `%APPDATA%\Stringcast\config.toml` |

### Config schema

```toml
[general]
enabled = true                    # Global on/off
startup_at_login = true           # Launch at OS login
show_spinner = true               # Show spinner overlay during AI call
spinner_style = "dots"            # "dots" | "braille" | "bar"
max_extract_chars = 16384         # Max chars sent to AI
undo_timeout_minutes = 10         # Minutes before undo slot expires
log_level = "warn"                # "error" | "warn" | "info" | "debug"
collect_local_stats = true        # Local-only counters; no telemetry

[provider]
active = "gemini"                 # "gemini" | "openai" | "anthropic" | "custom"
gemini_model = "gemini-2.0-flash-lite"
openai_model = "gpt-4o-mini"
anthropic_model = "claude-haiku-4-5"
custom_base_url = ""
custom_model = ""

[api]
temperature = 0.3
max_output_tokens = 2048
connection_timeout_ms = 5000
response_timeout_ms = 30000
max_retries = 3

[extraction]
select_all_wait_ms = 100          # Wait after Ctrl+A before Ctrl+C
clipboard_read_wait_ms = 300      # Wait for clipboard to populate after Ctrl+C
clipboard_restore_wait_ms = 150   # Wait after paste before restoring clipboard

[exclusions]
apps = []                         # List of app bundle IDs / exe names to exclude
known_problematic_apps = []        # Auto-populated after repeated extraction/paste failures

[privacy]
confirm_before_first_api_call = true
redact_logs = true
allow_debug_body_logging = false

[commands]
# built-in command overrides
disabled_builtins = []            # e.g. ["?emoji"]

[[commands.custom]]
trigger = "?poem"
name = "Make it Poetic"
prompt = "..."
enabled = true
case_sensitive = true
raw_output = false
created_at = "2024-01-01T00:00:00Z"

[[api_keys]]
id = "uuid"
provider = "gemini"
alias = "Personal Key"
status = "Active"
created_at = "2024-01-15T10:00:00Z"
```

### Config file safety

- Writes are atomic: write to `.config.toml.tmp`, then rename
- On startup, validate TOML schema; fall back to defaults if corrupt
- Config version field (`config_version = 1`) for future migrations

---

## 9. System Tray UI

Stringcast runs primarily as a tray icon. **Crate:** `tray-icon` or `tauri` tray API.

### Tray icon states

| State | Icon |
|---|---|
| Active, ready | Stringcast logo (color) |
| Disabled (paused) | Stringcast logo (grey) |
| Processing (AI call in flight) | Animated spinner icon |
| Error (no keys, no network) | Logo with red dot |

### Tray context menu

```
Stringcast
────────────────────
✅ Enabled            ← toggle; shows ✅ or ⏸
────────────────────
Open Settings
────────────────────
Last Action: ?fix     ← shows last trigger used (greyed)
Undo Last Action      ← enabled only if undo slot is populated
────────────────────
Quit Stringcast
```

### Tray click behavior

- **Left click / single click (macOS):** Toggle the settings window
- **Right click:** Show context menu (both platforms)

---

## 10. Settings UI (Tauri Window)

The settings window is built with **Tauri** (Rust backend + web frontend). It is a single-window app opened from the tray.

### Navigation

Four tabs, accessible via a sidebar:

1. **Dashboard** — status, quick stats, enable/disable toggle
2. **Keys** — API key management per provider
3. **Commands** — built-in list + custom command CRUD
4. **Settings** — all config options

### Dashboard tab

- Global enable/disable toggle (large, prominent)
- Service status: "Ready" / "Processing" / "No API Keys" / "Paused"
- Stats: transformations today / total
- Active provider and model display
- Link to "Open Settings"
- First-run privacy notice before the first API call: clearly states that selected text is sent directly to the configured AI provider, never to Stringcast servers

### Keys tab

- Provider selector (Gemini / OpenAI / Anthropic / Custom)
- Per-provider:
  - Key list with alias, last-used timestamp, status badge
  - Add key: input field + alias field + "Validate & Add" button
  - Delete key (with confirmation)
  - Drag-to-reorder (sets priority for round-robin start)
- Validation status shown inline (spinner → ✅ / ❌)
- Custom provider section: Base URL + Model fields

### Commands tab

- Built-in commands list (read-only, with toggle to disable each)
- Custom commands section:
  - Table: Trigger | Name | Status | Actions
  - "Add Command" button → inline form: Trigger / Name / Prompt / Options
  - Prompt has a character counter (max 4096)
  - `{text}` placeholder highlighted in the prompt editor
  - Edit / Delete per command
  - Export / Import buttons

### Settings tab

Sections:

**General**
- Launch at login (checkbox)
- Show spinner during processing (checkbox)
- Spinner style (dropdown)

**Behavior**
- Max text to extract (slider: 1024 – 16384 chars)
- Undo timeout (slider: 1 – 60 minutes)

**AI Parameters**
- Temperature (slider: 0.0 – 1.0)
- Max output tokens (input: 256 – 4096)
- Request timeout (input: 10 – 120 seconds)

**Privacy**
- Confirm before first API call (checkbox)
- Local stats collection (checkbox; local counters only)
- Debug body logging (checkbox, hidden behind advanced mode and disabled by default)

**Exclusions**
- List of excluded apps with remove buttons
- "Add current app" button (detects frontmost app)
- Manual entry field

**Danger Zone**
- Reset all settings to defaults
- Clear all custom commands
- Clear all API keys
- Export configuration (full config.toml, keys NOT included)

---

## 11. Visual Feedback (Spinner)

While the AI API call is in flight, a spinner is shown **inline** — replacing the text in the field — to signal to the user that processing is occurring.

### Spinner frames

```
Dots style:    ⣾ ⣽ ⣻ ⢿ ⡿ ⣟ ⣯ ⣷
Braille style: ◐ ◓ ◑ ◒
Bar style:     [    ] [=   ] [==  ] [=== ] [====] [ ===] [  ==] [   =]
```

### Spinner behavior

1. After successful extraction: set clipboard to spinner frame 0, paste via Ctrl+A + Ctrl+V
2. Spin: every 120 ms, update clipboard to next frame, paste again (Ctrl+A + Ctrl+V)
3. On AI response received: stop spinning, set clipboard to AI result, paste via Ctrl+A + Ctrl+V
4. On error: stop spinning, set clipboard to original text, paste via Ctrl+A + Ctrl+V (restore)

### Spinner notes

- Spinning via repeated Ctrl+A + Ctrl+V feels natural in most apps but may be jarring in IDEs with syntax highlighting (every paste may re-trigger syntax analysis). Users can disable the spinner in Settings; when disabled, Stringcast leaves the original field text untouched until the AI response is ready.
- Spinner is disabled automatically for apps that do not support programmatic paste (those falling back to character simulation).
- Spinner updates must use the synthetic input guard from §4.1 and the clipboard ownership rules from §4.4.
- If focus changes while the spinner is visible, stop the operation immediately and attempt to restore the original text only if the original app/window is foreground again within 500 ms. Otherwise leave the current field unchanged and notify the user.

---

## 12. Exclusion List (Per-App Blocking)

Stringcast maintains a list of apps where it is operationally inactive: no trigger detection, no clipboard access, no API calls, and no replacement. The low-level OS hook may still receive key events because global hooks cannot be cheaply registered/unregistered per foreground app; events from excluded apps are dropped immediately after the foreground-app check.

### Exclusion detection

On macOS: check the frontmost app's bundle identifier (e.g. `com.apple.Terminal`, `com.1password.1password`).  
On Windows: check the foreground window's process executable name (e.g. `1Password.exe`, `KeePass.exe`).

This check happens on focus change and again before every trigger execution. The cached foreground app ID gates detection so excluded apps do not build meaningful text buffers.

### Default exclusion list (pre-populated)

```
# macOS bundle IDs
com.apple.keychainaccess
com.1password.1password
com.agilebits.onepassword7
com.bitwarden.desktop
com.lastpass.lastpass-mac
com.dashlane.Dashlane

# Windows EXEs
1Password.exe
KeePass.exe
KeePassXC.exe
Bitwarden.exe
LastPass.exe
```

Users can add/remove from this list in Settings → Exclusions.

---

## 13. Platform-Specific Details

### 13.1 macOS

#### Permissions

Stringcast requires **Accessibility permission** (System Settings → Privacy & Security → Accessibility) to:
- Register a global keyboard hook via `rdev`
- Read the frontmost application's bundle ID

On first launch, if permission is not granted:
1. Show an onboarding window explaining why it's needed
2. Open System Settings → Accessibility with a deep link
3. Poll for permission every 2 seconds; proceed automatically when granted
4. Do not show the settings window until permission is granted

**Input Monitoring** permission is also required on macOS 10.15+ for global keyboard capture. Request both at first launch.

#### Secure input mode

macOS Secure Event Input can be enabled by password fields, Terminal, SSH tools, and some security apps. While secure input is active, Stringcast must pause trigger detection, clear the buffer, and show a tray warning state if the user opens Settings. Do not attempt clipboard extraction from secure input contexts.

#### Select-all / copy / paste key codes

- Select all: `Cmd+A` (`enigo::Key::Meta + 'a'`)
- Copy: `Cmd+C`
- Paste: `Cmd+V`

#### App focus detection

Use `NSWorkspace.didActivateApplicationNotification` (via `objc2` or a C FFI shim) to detect app switches and clear the keystroke buffer. This is important for buffer accuracy.

#### Login item (launch at startup)

Use `SMLoginItemSetEnabled` or the newer `ServiceManagement` framework (`SMAppService.mainApp.register()` on macOS 13+) to register as a login item.

#### Code signing & notarization

- Must be signed with an Apple Developer ID certificate
- Must be notarized by Apple for distribution outside the App Store
- `hardened-runtime` entitlements required:
  ```xml
  <key>com.apple.security.automation.apple-events</key><true/>
  <key>com.apple.security.cs.allow-jit</key><false/>
  ```

### 13.2 Windows

#### Permissions

No special OS permission dialog is required for global keyboard hooks on Windows. The hook is registered via `SetWindowsHookEx(WH_KEYBOARD_LL, ...)` (used by `rdev`).

**Important:** Some security software (AV, EDR) may flag low-level keyboard hooks. The binary should be signed with a trusted code signing certificate to mitigate this.

#### Select-all / copy / paste key codes

- Select all: `Ctrl+A`
- Copy: `Ctrl+C`
- Paste: `Ctrl+V`

#### App detection

Use `GetForegroundWindow()` + `GetWindowThreadProcessId()` + `QueryFullProcessImageName()` to get the foreground app EXE name. Use this for the exclusion list check.

#### App focus change detection

Use `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ...)` to detect foreground window changes and clear the keystroke buffer.

#### Startup at login

Write a registry key under:
```
HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
Stringcast = "C:\Path\To\Stringcast.exe"
```

Or create a shortcut in the user's Startup folder.

#### Code signing

The executable must be signed with a trusted EV or OV code signing certificate (DigiCert, Sectigo, etc.) to avoid Windows SmartScreen warnings on first run.

#### Windows Defender / AV

Include a `Stringcast.exe.manifest` with UAC level `asInvoker` (no elevation needed). Submit the binary to Microsoft's malware sample portal to whitelist before release.

#### Integrity levels and elevated apps

A non-elevated Stringcast process may not reliably send input to elevated administrator windows because of Windows integrity-level isolation. Stringcast must detect when the foreground process is elevated where possible, abort the operation before clipboard access, and notify: "Cannot transform text in elevated apps unless Stringcast is also elevated." Running Stringcast elevated is not recommended for normal use.

---

## 14. Edge Cases & Failure Modes

### Input edge cases

| Scenario | Handling |
|---|---|
| Buffer longer than 8192 bytes | Trim oldest chars; keep tail |
| Trigger typed very fast (< 50 ms between chars) | No issue; detection happens synchronously post-append |
| Trigger typed in middle of a word | The buffer-based detection requires trigger to be a suffix; mid-word triggers are not detected |
| Two triggers typed in quick succession | First trigger fires; buffer clears; second trigger starts fresh |
| Trigger typed while previous AI call is still in flight | Queue the second trigger; process after first completes. Show "queued" indicator in tray |
| User edits text after typing trigger but before AI returns | Potential race: the replacement may overwrite user's new edits. Mitigation: after AI returns, check if the field content still matches the pre-extraction snapshot before pasting. If not, abort and notify |
| `?undo` with no previous action | Notification: "Nothing to undo" |
| `?undo` in a field that changed | Undo pastes the saved text regardless; the user is in control |
| Very long extracted text (> 16384 chars) | Truncate to 16384 from the start; notify the user ("Text too long — only last N chars sent") |
| Emoji or Unicode in extracted text | Passed as-is in UTF-8; most AI APIs handle this correctly |
| RTL text (Arabic, Hebrew) | Extracted and returned as UTF-8; visual rendering is handled by the target app |
| Null bytes in clipboard | Sanitize clipboard content; strip null bytes before processing |
| Synthetic paste events observed by hook | Drop via `synthetic_input_guard`; clear buffer after operation |
| Dynamic trigger partially typed | Wait for dynamic debounce; cancel on navigation/focus changes |
| User types escaped trigger like `\?fix` | Do not execute; leave text untouched |
| AltGr or dead-key input | Treat printable result as text input; do not misclassify AltGr as shortcut |
| Secure input mode active | Pause detection and clear buffer |

### API edge cases

| Scenario | Handling |
|---|---|
| API returns empty string | Surface error: "AI returned empty response"; restore original text |
| API returns only whitespace | Treat as empty; same as above |
| API response is 100× longer than input | Warn user (notification), paste anyway (user can undo) |
| API returns non-UTF-8 bytes | Replace invalid byte sequences with replacement character (U+FFFD); log warning |
| API key starts returning 403 mid-session | Mark invalid, rotate; show notification if all keys invalid |
| Custom endpoint returns HTML (e.g. login page) | Detect `Content-Type: text/html` in response; return `ApiError::UnexpectedContentType` |
| Streaming response (if enabled in future) | Parse SSE stream; accumulate delta tokens; update spinner text progressively |
| Network proxy required | Respect `HTTP_PROXY` / `HTTPS_PROXY` env vars (reqwest does this by default) |
| Provider rejects content for safety | Restore original text; notify without retrying the same key |
| Provider model is removed or unavailable | Mark model invalid in settings; block transformations until changed |
| Provider returns 200 with no candidates/content | Treat as malformed/empty response; restore original text |
| Response exceeds paste simulation cap | Put result on clipboard, restore field to original, and show "paste manually" notification |

### Clipboard edge cases

| Scenario | Handling |
|---|---|
| Clipboard contains non-text (image, file) | Save clipboard type info; restore correctly after operation. Use `arboard`'s typed clipboard API |
| Clipboard is locked by another app | Retry clipboard access up to 5 times with 50 ms delay; if still locked, skip clipboard restore (log warning) |
| Clipboard content changes between save and restore | Restore only if current clipboard still matches Stringcast-owned content; otherwise leave the newer external clipboard alone |
| App doesn't update clipboard on Ctrl+C | Fall back to keystroke buffer for extraction |
| Paste produces wrong text (app transforms paste) | Detected on verification check; log as warning; no further action (app's behaviour) |
| Clipboard changes by another app during operation | Restore only if current clipboard still matches Stringcast-owned content; otherwise leave clipboard untouched |
| Clipboard has rich text plus plain text | Preserve all supported formats in `ClipboardSnapshot`; use plain text only for AI input |
| Clipboard restore fails after successful paste | Keep replacement, notify only if user-visible clipboard was not restored after retries |

### System edge cases

| Scenario | Handling |
|---|---|
| OS credential store unavailable | Fall back to AES-256-GCM encrypted file under app data dir, with user-supplied passphrase prompt |
| Config file corrupted | Warn on startup, offer reset to defaults; don't silently overwrite |
| Disk full (can't write config) | In-memory only for session; warn on next startup |
| Multiple instances started | Use a lock file in the platform data/runtime directory (`~/Library/Application Support/Stringcast/stringcast.lock` on macOS, `%LOCALAPPDATA%\Stringcast\stringcast.lock` on Windows); second instance detects lock and brings first instance's window to front, then exits |
| Machine wakes from sleep | Re-register global hook if it was lost; macOS may drop hooks across sleep |
| Accessibility permission revoked mid-session | Detect hook failure on next keystroke; show notification; re-prompt for permission |
| User is in a VM / Remote Desktop | Keyboard hooks may behave differently; document known limitations |
| Foreground app changes during API call | Cancel operation before replacement; do not paste into the new app |
| Foreground app changes while spinner is visible | Restore only if original app/window is foreground again and current text is Stringcast-owned spinner |
| Windows elevated foreground window | Abort before clipboard access unless Stringcast also runs at compatible integrity level |
| macOS Secure Event Input enabled | Pause detection and clear buffer until disabled |
| System time changes | Use `Instant` for operation timeout/undo expiry; wall-clock timestamps are diagnostics only |

---

## 15. Security Model

### Threat model

| Threat | Mitigation |
|---|---|
| Keystroke logging by Stringcast | All captured keys processed in-process only; never written to disk; log sanitization strips any high-entropy strings |
| API key theft from disk | Keys stored in OS keychain, never in plaintext config files |
| API key theft from memory | Use `secrecy::SecretString`; keys zeroed after each use |
| Man-in-the-middle on API calls | TLS enforced for all API endpoints; certificate validation enabled (not bypassed) |
| Malicious custom prompt injection | User-defined prompts only; no external prompt injection surface |
| Clipboard snooping | Clipboard is written only during active transformation; restored immediately after; contents are the user's own text |
| Privilege escalation | Process runs as current user; no elevated privileges requested |
| Binary tampering | Code signing on both platforms; binary hash in release notes |
| Supply chain | `Cargo.lock` pinned; reproducible builds via `cargo build --locked` |
| Prompt injection inside selected text | System prompts instruct the model to transform text, not follow instructions contained in the text; no tool calls or external actions are exposed |
| Accidental sensitive-field submission | Exclusion defaults, password-field heuristics, secure-input detection, and first-run privacy confirmation reduce risk; user remains responsible for provider choice |

### Privacy guarantees

- No data leaves the device except to the configured AI provider API
- No analytics, no telemetry, no crash reporting (opt-in crash dump may be added in future, defaulting to off)
- No user account required
- Text processed in-flight; not stored on disk beyond the undo slot (in-memory)
- Log files (if enabled) do not contain user text; only metadata (trigger name, char count, latency)
- Local stats are counters only and contain no text content; they can be disabled in Settings
- Debug body logging is disabled by default, requires advanced opt-in, and must redact secrets before writing any file

---

## 16. Performance Constraints

| Metric | Target |
|---|---|
| Keystroke handler latency | < 1 ms per keypress (no perceptible typing lag) |
| Trigger detection time | < 0.5 ms |
| Memory usage (idle) | < 30 MB RSS |
| Memory usage (during AI call) | < 60 MB RSS |
| Core binary size (release) | < 15 MB stripped, excluding installer/package overhead and OS WebView runtime |
| Startup time (to tray icon visible) | < 500 ms |
| CPU usage (idle, hook active) | < 0.1% on a modern CPU |

### Performance rules

- The global keyboard hook callback must do minimal work: append char, run fast-exit check, optionally enqueue a detection task
- All AI calls happen in a `tokio::spawn` task, never blocking the hook thread
- Clipboard operations happen on a dedicated `tokio::task` to avoid blocking the hook thread
- The settings UI (Tauri WebView) is only loaded when the window is opened; it does not run on startup

---

## 17. Crate & Dependency Reference

### Core dependencies

| Crate | Version | Purpose |
|---|---|---|
| `rdev` | 0.5+ | Global keyboard/mouse hook |
| `enigo` | 0.2+ | Keyboard input simulation |
| `arboard` | 3+ | Clipboard read/write |
| `reqwest` | 0.12+ | Async HTTP client |
| `tokio` | 1 (full features) | Async runtime |
| `serde` | 1 | Serialization |
| `serde_json` | 1 | JSON parsing |
| `toml` | 0.8+ | Config file format |
| `keyring` | 2+ | OS credential store |
| `secrecy` | 0.8+ | In-memory secret management |
| `zeroize` | 1+ | Explicit memory zeroing for sensitive buffers |
| `uuid` | 1 | Key IDs |
| `chrono` | 0.4 | Timestamps |
| `log` + `env_logger` | 0.4 / 0.11 | Logging |
| `anyhow` | 1 | Error handling |
| `directories` | 5+ | Platform-correct config/cache/data paths |
| `tray-icon` | 0.14+ | System tray icon |
| `tauri` | 2 | Settings UI |

### Optional/platform-specific

| Crate | Platform | Purpose |
|---|---|---|
| `windows` | Windows | Win32 API bindings for foreground app detection |
| `objc2` | macOS | Objective-C bridge for NSWorkspace notifications |
| `core-foundation` | macOS | macOS system integration |
| `aes-gcm` + `argon2` | Fallback only | Encrypted key file when OS credential store is unavailable |
| `notify-rust` | Optional | Cross-platform desktop notifications when Tauri tray notifications are unavailable |

### Dev dependencies

| Crate | Purpose |
|---|---|
| `mockall` | Mocking for unit tests |
| `wiremock` | HTTP mock server for API tests |
| `tokio-test` | Async test utilities |
| `tempfile` | Temp files in tests |

---

## 18. Project Structure

```
stringcast/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── SPEC.md
├── LICENSE
├── build.rs                        # Build script (resource embedding, icons)
│
├── src/
│   ├── main.rs                     # Entry point; tray setup; thread orchestration
│   ├── lib.rs                      # Re-exports for integration tests
│   ├── orchestrator.rs             # Operation state machine; queue; cancellation
│   │
│   ├── input/
│   │   ├── mod.rs
│   │   ├── hook.rs                 # rdev global hook; event dispatch
│   │   ├── buffer.rs               # Keystroke buffer; backspace; clear logic
│   │   └── modifier.rs             # Modifier key state tracking
│   │
│   ├── detection/
│   │   ├── mod.rs
│   │   ├── engine.rs               # Trigger scan; longest-match; fast-exit
│   │   └── dynamic.rs              # ?translate:XX and ?ask:* parsing
│   │
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── builtins.rs             # Built-in command definitions
│   │   ├── custom.rs               # Custom command CRUD; persistence
│   │   └── registry.rs             # Unified command lookup
│   │
│   ├── extraction/
│   │   ├── mod.rs
│   │   └── clipboard.rs            # Select-all + copy + read; save/restore
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── client.rs               # Main API dispatch; retry logic
│   │   ├── key_pool.rs             # Round-robin; status tracking
│   │   ├── providers/
│   │   │   ├── gemini.rs
│   │   │   ├── openai.rs
│   │   │   ├── anthropic.rs
│   │   │   └── custom.rs
│   │   └── response_cleaner.rs     # Preamble stripping; whitespace normalization
│   │
│   ├── replacement/
│   │   ├── mod.rs
│   │   ├── engine.rs               # Paste result; verify; character-sim fallback
│   │   ├── spinner.rs              # Spinner frames; update loop
│   │   └── undo.rs                 # Undo slot management
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── config.rs               # TOML config read/write; schema; migration
│   │   ├── keystore.rs             # keyring wrapper; SecretString handling
│   │   └── stats.rs                # Local-only counters; no text content
│   │
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── tray.rs                 # Tray icon; context menu; state
│   │   └── window.rs               # Tauri window lifecycle
│   │
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── macos.rs                # NSWorkspace; permissions; login item
│   │   ├── secure_input.rs         # Secure input/password-field heuristics
│   │   └── windows.rs              # Win32 foreground; hooks; registry
│   │
│   └── errors.rs                   # Central error types
│
├── ui/                             # Tauri frontend (HTML/CSS/JS or React)
│   ├── src/
│   │   ├── App.tsx
│   │   ├── tabs/
│   │   │   ├── Dashboard.tsx
│   │   │   ├── Keys.tsx
│   │   │   ├── Commands.tsx
│   │   │   └── Settings.tsx
│   │   └── components/
│   └── package.json
│
├── assets/
│   ├── icon.png                    # 1024x1024 app icon
│   ├── tray-active.png
│   ├── tray-disabled.png
│   └── tray-error.png
│
└── tests/
    ├── integration/
    │   ├── trigger_detection.rs
    │   ├── api_client.rs
    │   └── config.rs
    └── e2e/                        # Manual / semi-automated E2E tests
        └── README.md
```

---

## 19. Build & Release

### Build commands

```bash
# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release --locked

# Build for specific platform
cargo build --release --target aarch64-apple-darwin       # macOS Apple Silicon
cargo build --release --target x86_64-apple-darwin        # macOS Intel
cargo build --release --target x86_64-pc-windows-msvc     # Windows x64
cargo build --release --target aarch64-pc-windows-msvc    # Windows ARM64

# Universal macOS binary
lipo -create -output Stringcast \
  target/aarch64-apple-darwin/release/stringcast \
  target/x86_64-apple-darwin/release/stringcast
```

### macOS distribution

1. Build universal binary
2. Create `.app` bundle (`Stringcast.app`)
3. Sign with Developer ID Application certificate:
   ```bash
   codesign --deep --force --verify --verbose \
     --sign "Developer ID Application: Your Name (XXXXXXXXXX)" \
     --options runtime \
     Stringcast.app
   ```
4. Notarize:
   ```bash
   xcrun notarytool submit Stringcast.zip \
     --apple-id your@email.com \
     --team-id XXXXXXXXXX \
     --password @keychain:notarytool-password
   xcrun stapler staple Stringcast.app
   ```
5. Package as `.dmg` with a drag-to-Applications installer

### Windows distribution

1. Build x64 + ARM64 binaries
2. Sign with code signing certificate:
   ```
   signtool sign /fd SHA256 /tr http://timestamp.digicert.com \
     /td SHA256 /f certificate.pfx /p password Stringcast.exe
   ```
3. Package with NSIS or WiX installer, or distribute as portable `.zip`
4. Submit binary to Microsoft for SmartScreen reputation (via Partner Center)

### GitHub Actions CI/CD

```yaml
# .github/workflows/release.yml
on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-14
    steps:
      - build universal binary
      - sign + notarize
      - upload .dmg artifact

  build-windows:
    runs-on: windows-latest
    steps:
      - build x64 binary
      - sign
      - package installer
      - upload artifact
```

---

## 20. Testing Strategy

### Unit tests

Located alongside source files (`src/**/*.rs`). Cover:

- `buffer.rs`: append, backspace, overflow, clear, Unicode correctness
- `engine.rs`: longest-match, fast-exit, no false positives, case sensitivity
- `dynamic.rs`: all valid and invalid `?translate:` codes; `?ask:` parsing
- `orchestrator.rs`: state transitions, cancellation, one-active/one-queued policy
- `hook.rs`: synthetic input guard, AltGr/dead-key handling, exclusion gating
- `response_cleaner.rs`: preamble stripping; code block removal; whitespace edge cases
- `key_pool.rs`: round-robin order; skip rate-limited; all-invalid behaviour
- `config.rs`: TOML parse; invalid schema fallback; atomic write

### Integration tests

Located in `tests/integration/`. Use `wiremock` to mock API endpoints.

- Full trigger → API call → cleaned response pipeline
- Key rotation across 429 responses
- Retry logic on 5xx
- Timeout behaviour
- Dynamic trigger debounce and cancellation
- Clipboard ownership: do not overwrite external clipboard changes
- Focus-change cancellation before replacement

### Manual E2E checklist (pre-release)

```
[ ] Works in Safari / Chrome / Firefox (macOS)
[ ] Works in Mail.app (macOS)
[ ] Works in Terminal.app (macOS)
[ ] Works in VS Code (macOS + Windows)
[ ] Works in Microsoft Word (Windows)
[ ] Works in Notepad (Windows)
[ ] Works in Chrome (Windows)
[ ] Exclusion list blocks trigger in 1Password
[ ] Undo restores text correctly
[ ] ?translate:ja produces Japanese
[ ] ?ask:summarize this in 5 words works
[ ] Dynamic triggers wait for debounce and do not fire mid-typing
[ ] Multi-key rotation works (add 2 keys, throttle first via mock)
[ ] App survives sleep/wake cycle
[ ] Second instance quits gracefully
[ ] Config corruption recovery
[ ] Accessibility permission prompt (macOS)
[ ] Secure input/password manager contexts do not read clipboard or call API
[ ] Clipboard is restored after success and left alone if changed externally during processing
[ ] Programmatic paste/spinner events do not trigger a second transformation
[ ] Focus change during API call does not paste into the new app
[ ] SmartScreen passes (Windows — signed build)
```

### Performance test

Measure keystroke handler latency with a synthetic test: send 10,000 synthetic key events via `rdev::simulate` and measure time per event. Must be < 1 ms p99.

---

## 21. Future Roadmap

Items explicitly **out of scope for v1.0** but tracked for future versions:

| Feature | Priority |
|---|---|
| Streaming AI responses (progressive text replacement) | High |
| Image paste transformations (paste image → AI description) | Medium |
| Per-app command sets (different commands enabled in VS Code vs Mail) | Medium |
| Command history & usage analytics (local only) | Medium |
| Regex-based custom triggers | Medium |
| Multi-level undo | Low |
| Team sharing of custom commands (via file export/sync) | Low |
| Linux support (X11 + Wayland) | Low |
| Local model support via Ollama autodiscovery | Low |
| Voice trigger (say "fix this") | Low |
| Tauri mobile (iOS/Android) | Low |
| Plugin system for community commands | Low |

---

*This document is the authoritative specification for Stringcast Desktop v1.0. All implementation decisions should reference and update this document. When in doubt, favour the simplest approach that satisfies the spec — complexity can be added later.*

---

**End of SPEC.md**
