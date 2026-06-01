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

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp packaging/macos/Info.plist "$CONTENTS_DIR/Info.plist"
cp "$BIN_PATH" "$MACOS_DIR/Stringcast"
xcrun --sdk macosx swiftc \
  -module-cache-path "${TMPDIR:-/tmp}/stringcast-swift-module-cache" \
  packaging/macos/StringcastMenu.swift \
  -framework AppKit \
  -o "$MACOS_DIR/StringcastMenu"

chmod +x "$MACOS_DIR/Stringcast" "$MACOS_DIR/StringcastMenu"

echo "Built $APP_DIR"
