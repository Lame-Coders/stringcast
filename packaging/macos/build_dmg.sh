#!/bin/sh
set -eu

if [ "${1:-}" = "--help" ]; then
  cat <<'USAGE'
usage: packaging/macos/build_dmg.sh [app-path] [output-dmg-path] [volume-name]

Builds an unsigned macOS DMG with Stringcast.app and an Applications shortcut.

Defaults:
  app-path: dist/macos/Stringcast.app
  output-dmg-path: dist/stringcast-macos.dmg
  volume-name: Stringcast
USAGE
  exit 0
fi

APP_PATH=${1:-dist/macos/Stringcast.app}
DMG_PATH=${2:-dist/stringcast-macos.dmg}
VOLUME_NAME=${3:-Stringcast}

if [ "$(uname -s)" != "Darwin" ]; then
  echo "macOS DMGs can only be built on macOS" >&2
  exit 1
fi

if [ ! -d "$APP_PATH" ]; then
  echo "missing app bundle: $APP_PATH" >&2
  echo "run: cargo build --release && packaging/macos/build_app.sh" >&2
  exit 1
fi

if ! command -v hdiutil >/dev/null 2>&1; then
  echo "missing hdiutil; cannot generate DMG" >&2
  exit 1
fi

DMG_DIR=$(dirname "$DMG_PATH")
mkdir -p "$DMG_DIR"

STAGING_DIR=$(mktemp -d "${TMPDIR:-/tmp}/stringcast-dmg.XXXXXX")
trap 'rm -rf "$STAGING_DIR"' EXIT INT TERM

cp -R "$APP_PATH" "$STAGING_DIR/Stringcast.app"
ln -s /Applications "$STAGING_DIR/Applications"

hdiutil create \
  -volname "$VOLUME_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

echo "Built $DMG_PATH"
