# macOS App Wrapper

Stringcast can be packaged as an unsigned menu-bar `.app` bundle around the Rust CLI binary. This gives macOS a stable app identity for permissions and Keychain prompts while the runtime still uses the existing CLI internally.

Launching the app starts `stringcast run` in the background and adds a `Stringcast` item to the macOS menu bar.

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

Use the menu-bar item to:

- View status
- Start Stringcast
- Stop Stringcast
- Check permissions
- Run an API test
- Open the config file
- Open logs
- Quit

Stop it from Terminal if needed:

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
- There is no custom app icon yet.
- Logs open in Finder rather than an in-app viewer.
- Packaging does not create a DMG or installer yet.

## Next Packaging Steps

- Add an icon.
- Add log/status display.
- Add code signing and notarization.
- Produce a DMG for end users.
