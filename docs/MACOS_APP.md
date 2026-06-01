# macOS App Wrapper

Stringcast can be packaged as an unsigned `.app` bundle where the Rust runtime is the app executable. This keeps the permission-sensitive keyboard listening and replacement work under `Stringcast.app` instead of a separate helper binary.

Launching the app starts the runtime and adds a `Stringcast` companion item to the macOS menu bar.

## Build Locally

From the repo root:

```bash
cargo build --release
packaging/macos/build_app.sh
```

To include the app icon, save a square PNG at:

```text
packaging/macos/StringcastIcon.png
```

The packaging script converts it to `Contents/Resources/StringcastIcon.icns`. You can also use a different source path for local builds:

```bash
STRINGCAST_ICON=/path/to/icon.png packaging/macos/build_app.sh
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
- Request permissions
- Run an API test
- Open the config file
- Open logs
- Reveal the app executable
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

Grant permissions to `Stringcast.app`. The runtime now runs as the app executable:

- `dist/macos/Stringcast.app/Contents/MacOS/Stringcast`

If macOS still shows a stale `Stringcast` entry from an older local build, remove the old entry and add the rebuilt `Stringcast.app` again. To locate the current executable manually, use `Reveal App Executable` from the menu, or open Finder's Go to Folder dialog and paste:

```text
dist/macos/Stringcast.app/Contents/MacOS/
```

The app does not block startup on this permission check. If permissions are missing, Stringcast can still show as running, but keyboard listening or text replacement may not work until macOS grants the required permissions.

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
