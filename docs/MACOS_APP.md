# macOS App Wrapper

Stringcast can be packaged as an unsigned `.app` bundle around the Rust CLI binary. This gives macOS a stable app identity for permissions and Keychain prompts while the runtime still uses the existing CLI internally.

This is not a full menu-bar UI yet. Launching the app starts `stringcast run` in the background.

## Build Locally

From the repo root:

```bash
cargo build --release
packaging/macos/build_app.sh
```

The app is written to:

```text
dist/macos/Stringcast.app
```

Open it:

```bash
open dist/macos/Stringcast.app
```

Stop it:

```bash
pkill -f "Stringcast.app"
```

## Permissions

macOS may ask for:

- Accessibility
- Input Monitoring
- Keychain access

If permissions were granted to a terminal binary before, macOS may ask again because `Stringcast.app` has a different app identity.

## Current Limitations

- The app is unsigned.
- There is no menu-bar UI yet.
- There is no visible quit control yet.
- Logs are not surfaced in an app window.
- Packaging does not create a DMG or installer yet.

## Next Packaging Steps

- Add an icon.
- Add a menu-bar process with Start/Stop/Quit controls.
- Add log/status display.
- Add code signing and notarization.
- Produce a DMG for end users.
