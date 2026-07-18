#!/bin/bash
set -euo pipefail

VERSION="${1#v}"
PACKAGE_NAME="RustyNTRViewer-${VERSION}-macos-universal"
DIST="$PWD/dist"
APP="$DIST/RustyNTRViewer.app"
DMG_ROOT="$DIST/dmg-root"

cargo build --release --locked -p rusty-ntr-viewer --target aarch64-apple-darwin
cargo build --release --locked -p rusty-ntr-viewer --target x86_64-apple-darwin

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$DMG_ROOT"
cp apps/rusty-ntr-viewer/Info.plist "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $VERSION" "$APP/Contents/Info.plist"
lipo -create \
  target/aarch64-apple-darwin/release/rusty-ntr-viewer \
  target/x86_64-apple-darwin/release/rusty-ntr-viewer \
  -output "$APP/Contents/MacOS/rusty-ntr-viewer"
chmod +x "$APP/Contents/MacOS/rusty-ntr-viewer"
codesign --force --deep --sign - "$APP"

cp -R "$APP" "$DMG_ROOT/"
cp README.md LICENSE THIRD_PARTY_NOTICES.md "$DMG_ROOT/"
hdiutil create \
  -volname RustyNTRViewer \
  -srcfolder "$DMG_ROOT" \
  -format UDZO \
  -ov \
  "$DIST/$PACKAGE_NAME.dmg"
