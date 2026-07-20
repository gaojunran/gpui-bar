#!/usr/bin/env bash
# Build the release binary, assemble a macOS .app bundle, and produce a DMG.
#
# Usage: scripts/build_dmg.sh [output-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${1:-$ROOT/dist}"
APP_NAME="gpui-bar"
BIN_NAME="gpui-dashboard"

if [[ -z "${VERSION:-}" ]]; then
  VERSION=$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$ROOT/Cargo.toml")
fi

cd "$ROOT"

echo "==> building release binary (version $VERSION)"
cargo build --release

BIN_PATH="$ROOT/target/release/$BIN_NAME"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "missing binary at $BIN_PATH" >&2
  exit 1
fi

APP_ROOT="$OUT_DIR/$APP_NAME.app"
echo "==> assembling $APP_ROOT"
rm -rf "$APP_ROOT"
mkdir -p "$APP_ROOT/Contents/MacOS" "$APP_ROOT/Contents/Resources"

cp "$BIN_PATH" "$APP_ROOT/Contents/MacOS/$BIN_NAME"
if [[ -f "$ROOT/assets/AppIcon.icns" ]]; then
  cp "$ROOT/assets/AppIcon.icns" "$APP_ROOT/Contents/Resources/AppIcon.icns"
else
  echo "warning: assets/AppIcon.icns not found" >&2
fi

INFO_PLIST="$APP_ROOT/Contents/Info.plist"
cat > "$INFO_PLIST" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>gpui-bar</string>
  <key>CFBundleDisplayName</key>
  <string>gpui-bar</string>
  <key>CFBundleIdentifier</key>
  <string>com.gaojunran.gpui-bar</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleExecutable</key>
  <string>gpui-dashboard</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

# patch version fields into the templated plist
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$INFO_PLIST" 2>/dev/null || true
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$INFO_PLIST" 2>/dev/null || true

if command -v codesign >/dev/null 2>&1; then
  echo "==> ad-hoc codesign"
  codesign --force --deep --sign - "$APP_ROOT" 2>/dev/null || true
fi

DMG_NAME="${APP_NAME}-v${VERSION}-macos.dmg"
DMG_PATH="$OUT_DIR/$DMG_NAME"
STAGING="$OUT_DIR/dmg-staging"
echo "==> building DMG $DMG_PATH"
rm -rf "$STAGING" "$DMG_PATH"
mkdir -p "$STAGING"
cp -R "$APP_ROOT" "$STAGING/"
ln -s /Applications "$STAGING/Applications"

TMP_DMG="$OUT_DIR/tmp-${DMG_NAME}"
hdiutil create -srcfolder "$STAGING" -volname "$APP_NAME" -fs HFS+ \
  -format UDRW "$TMP_DMG" >/dev/null
hdiutil convert "$TMP_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_PATH" >/dev/null
rm -f "$TMP_DMG"
rm -rf "$STAGING"

echo "==> done"
ls -la "$DMG_PATH"
