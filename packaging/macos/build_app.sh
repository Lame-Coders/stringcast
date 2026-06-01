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

if ! command -v iconutil >/dev/null 2>&1; then
  echo "missing iconutil; cannot generate app icon" >&2
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
  ICONSET_DIR="$OUT_DIR/StringcastIcon.iconset"
  rm -rf "$ICONSET_DIR"
  mkdir -p "$ICONSET_DIR"
  sips -z 16 16 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
  sips -z 32 32 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
  sips -z 64 64 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
  sips -z 256 256 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
  sips -z 512 512 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "$ICON_SOURCE" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null
  iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/StringcastIcon.icns"
  rm -rf "$ICONSET_DIR"
else
  echo "warning: icon source not found: $ICON_SOURCE" >&2
  echo "         save the app icon PNG there, or set STRINGCAST_ICON=/path/to/icon.png" >&2
fi

chmod +x "$MACOS_DIR/Stringcast" "$MACOS_DIR/StringcastMenu"

echo "Built $APP_DIR"
