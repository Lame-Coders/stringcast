#!/bin/sh
set -eu

if [ "${1:-}" = "--help" ]; then
  cat <<'USAGE'
usage: packaging/macos/build_app.sh [release-binary-path] [output-dir]

Builds an unsigned Stringcast.app bundle around the Rust CLI binary.

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

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp packaging/macos/Info.plist "$CONTENTS_DIR/Info.plist"
cp packaging/macos/Stringcast "$MACOS_DIR/Stringcast"
cp "$BIN_PATH" "$RESOURCES_DIR/stringcast"

chmod +x "$MACOS_DIR/Stringcast" "$RESOURCES_DIR/stringcast"

echo "Built $APP_DIR"
