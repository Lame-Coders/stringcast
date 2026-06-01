#!/bin/sh
set -eu

if [ "${1:-}" = "--help" ]; then
  cat <<'USAGE'
usage: packaging/macos/build_app.sh [release-binary-path] [output-dir]

Builds an unsigned Stringcast.app bundle where the Rust runtime is the app executable.

Defaults:
  release-binary-path: target/release/stringcast
  output-dir: dist/macos
USAGE
  exit 0
fi

BIN_PATH=${1:-target/release/stringcast}
OUT_DIR=${2:-dist/macos}
APP_DIR="$OUT_DIR/Stringcast.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICON_SOURCE=${STRINGCAST_ICON:-packaging/macos/StringcastIcon.png}

if [ ! -f "$BIN_PATH" ]; then
  echo "missing binary: $BIN_PATH" >&2
  echo "run: cargo build --release" >&2
  exit 1
fi

if [ "$(uname -s)" != "Darwin" ]; then
  echo "macOS app bundles can only be built on macOS" >&2
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "missing xcrun; install Xcode command line tools" >&2
  exit 1
fi

if ! xcrun --sdk macosx --find swiftc >/dev/null 2>&1; then
  echo "missing swiftc; install Xcode command line tools" >&2
  exit 1
fi

if ! command -v sips >/dev/null 2>&1; then
  echo "missing sips; cannot generate app icon" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "missing python3; cannot generate app icon" >&2
  exit 1
fi

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp packaging/macos/Info.plist "$CONTENTS_DIR/Info.plist"
cp "$BIN_PATH" "$MACOS_DIR/Stringcast"
xcrun --sdk macosx swiftc \
  -module-cache-path "${TMPDIR:-/tmp}/stringcast-swift-module-cache" \
  packaging/macos/StringcastMenu.swift \
  -framework AppKit \
  -o "$MACOS_DIR/StringcastMenu"

if [ -f "$ICON_SOURCE" ]; then
  ICON_WORK_DIR="$OUT_DIR/StringcastIcon.work"
  rm -rf "$ICON_WORK_DIR"
  mkdir -p "$ICON_WORK_DIR"
  sips -Z 1024 "$ICON_SOURCE" --out "$ICON_WORK_DIR/icon-scaled.png" >/dev/null
  sips --padToHeightWidth 1024 1024 --padColor FFFFFF "$ICON_WORK_DIR/icon-scaled.png" --out "$ICON_WORK_DIR/icon-1024.png" >/dev/null 2>&1
  for size in 16 32 64 128 256 512 1024; do
    sips -z "$size" "$size" "$ICON_WORK_DIR/icon-1024.png" --out "$ICON_WORK_DIR/icon_${size}.png" >/dev/null
  done
  python3 - "$RESOURCES_DIR/StringcastIcon.icns" "$ICON_WORK_DIR" <<'PY'
import struct
import sys
from pathlib import Path

output = Path(sys.argv[1])
source_dir = Path(sys.argv[2])
entries = [
    (b"icp4", "icon_16.png"),
    (b"icp5", "icon_32.png"),
    (b"icp6", "icon_64.png"),
    (b"ic07", "icon_128.png"),
    (b"ic08", "icon_256.png"),
    (b"ic09", "icon_512.png"),
    (b"ic10", "icon_1024.png"),
]

chunks = []
for icon_type, filename in entries:
    data = (source_dir / filename).read_bytes()
    chunks.append(icon_type + struct.pack(">I", len(data) + 8) + data)

payload = b"".join(chunks)
output.write_bytes(b"icns" + struct.pack(">I", len(payload) + 8) + payload)
PY
  rm -rf "$ICON_WORK_DIR"
else
  echo "warning: icon source not found: $ICON_SOURCE" >&2
  echo "         save the app icon PNG there, or set STRINGCAST_ICON=/path/to/icon.png" >&2
fi

chmod +x "$MACOS_DIR/Stringcast" "$MACOS_DIR/StringcastMenu"

echo "Built $APP_DIR"
