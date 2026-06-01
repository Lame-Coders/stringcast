# Release Artifacts

Stringcast can be distributed as downloadable binaries from GitHub Actions. This lets users run the Rust app without installing Cargo.

The current artifacts are raw CLI binaries, not full desktop installers. A macOS app wrapper and richer desktop packaging are planned separately.

## Build Artifacts

The `Release Builds` workflow creates:

```text
stringcast-macos.tar.gz
stringcast-linux-x86_64.tar.gz
stringcast-windows-x86_64.zip
```

Each archive includes:

- `stringcast` or `stringcast.exe`
- `Stringcast.app` in the macOS archive
- `README.md`
- `RUNNING.md`
- `SPEC.md`
- `docs/`

## Run The Workflow

From GitHub:

1. Open the repository.
2. Go to **Actions**.
3. Select **Release Builds**.
4. Click **Run workflow**.
5. Select `main`.
6. Click **Run workflow**.

From the GitHub CLI:

```bash
gh workflow run release-builds.yml --ref main
```

Check the latest run:

```bash
gh run list --workflow release-builds.yml --limit 5
```

Download artifacts:

```bash
gh run download <run-id> -D ./artifacts
```

## macOS Smoke Test

Unpack:

```bash
tar -xzf stringcast-macos.tar.gz
cd stringcast
chmod +x stringcast
```

Run checks:

```bash
./stringcast status
./stringcast check-permissions
./stringcast api-test
```

Run the app:

```bash
./stringcast run
```

Or launch the app wrapper:

```bash
open Stringcast.app
```

macOS may ask for:

- Accessibility permission for keyboard automation.
- Input Monitoring permission for global keyboard events.
- Keychain access when the binary reads a stored API key.

If permissions were already granted to a different terminal or binary path, macOS may still ask again for the downloaded binary.

See [MACOS_APP.md](MACOS_APP.md) for current app-wrapper limitations.

## Linux Smoke Test

Install native dependencies first. Ubuntu/Debian:

```bash
sudo apt update
sudo apt install build-essential pkg-config libdbus-1-dev libxdo-dev libx11-dev libxi-dev libxtst-dev
```

Unpack:

```bash
tar -xzf stringcast-linux-x86_64.tar.gz
cd stringcast
chmod +x stringcast
```

Run checks:

```bash
./stringcast status
./stringcast api-test
```

Run:

```bash
./stringcast run
```

Linux Wayland users should read [WAYLAND_POC.md](WAYLAND_POC.md). The Python Wayland listener is experimental and separate from the Rust release binary.

## Windows Smoke Test

Unpack `stringcast-windows-x86_64.zip`, then open PowerShell in the extracted `stringcast` directory.

Run checks:

```powershell
.\stringcast.exe status
.\stringcast.exe api-test
```

Run:

```powershell
.\stringcast.exe run
```

Windows support still needs real user testing. Some antivirus or SmartScreen warnings are expected for unsigned development binaries.

## API Key Setup

If a machine does not already have a stored Stringcast API key, add one:

macOS/Linux:

```bash
STRINGCAST_API_KEY="your-key-here" ./stringcast add-key gemini main "Gemini"
./stringcast set-provider gemini
./stringcast api-test
```

Windows PowerShell:

```powershell
$env:STRINGCAST_API_KEY="your-key-here"
.\stringcast.exe add-key gemini main "Gemini"
.\stringcast.exe set-provider gemini
.\stringcast.exe api-test
```

## Known Limitations

- Artifacts are unsigned.
- Artifacts are CLI binaries, not app installers.
- macOS permissions are tied to the binary/app identity and may need reapproval.
- Windows support needs broader manual testing.
- Linux Wayland support remains experimental.
