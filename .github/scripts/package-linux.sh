#!/bin/bash
set -euo pipefail

VERSION="${1#v}"
PACKAGE_NAME="RustyNTRViewer-${VERSION}-linux-x86_64"
DIST="$PWD/dist"
STAGE="$DIST/$PACKAGE_NAME"
DEB_ROOT="$DIST/deb-root"

cargo build --release --locked -p rusty-ntr-viewer

mkdir -p "$STAGE"
cp target/release/rusty-ntr-viewer README.md LICENSE THIRD_PARTY_NOTICES.md THIRD_PARTY_LICENSES.html "$STAGE/"
tar -C "$DIST" -czf "$DIST/$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"

mkdir -p "$DEB_ROOT/DEBIAN" "$DEB_ROOT/usr/bin" "$DEB_ROOT/usr/share/applications" "$DEB_ROOT/usr/share/doc/rusty-ntr-viewer"
install -m 755 target/release/rusty-ntr-viewer "$DEB_ROOT/usr/bin/rusty-ntr-viewer"
cp README.md LICENSE THIRD_PARTY_NOTICES.md THIRD_PARTY_LICENSES.html "$DEB_ROOT/usr/share/doc/rusty-ntr-viewer/"

cat > "$DEB_ROOT/DEBIAN/control" <<EOF
Package: rusty-ntr-viewer
Version: $VERSION
Section: video
Priority: optional
Architecture: amd64
Maintainer: Mina Girgis <minagirgis2001@users.noreply.github.com>
Depends: libgl1, libwayland-client0, libx11-6, libxkbcommon0
Description: Nintendo 3DS NTR remote-play viewer
 A memory-safe, cross-platform NTR/NTR-HR video viewer written in Rust.
EOF

cat > "$DEB_ROOT/usr/share/applications/rusty-ntr-viewer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=RustyNTRViewer
Comment=Nintendo 3DS NTR remote-play viewer
Exec=rusty-ntr-viewer
Terminal=false
Categories=AudioVideo;Video;
EOF

dpkg-deb --build --root-owner-group "$DEB_ROOT" "$DIST/rusty-ntr-viewer_${VERSION}_amd64.deb"
