#!/bin/sh
# Builds the Debian package into target/debian/sklad_<version>-1_amd64.deb.
# Run it on the oldest Debian the package supports, so its C library suits the newer ones too.
set -eu
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
cargo build --locked --release
root=target/debian/root
rm -rf "$root"
install -Dm755 target/release/sklad "$root/usr/bin/sklad"
install -Dm644 packaging/sklad.desktop "$root/usr/share/applications/sklad.desktop"
install -Dm644 assets/sklad.svg "$root/usr/share/icons/hicolor/scalable/apps/sklad.svg"
install -Dm644 packaging/io.github.pavver.sklad.metainfo.xml "$root/usr/share/metainfo/io.github.pavver.sklad.metainfo.xml"
install -Dm644 LICENSE "$root/usr/share/doc/sklad/copyright"
size=$(du -sk "$root" | cut -f1)
install -d "$root/DEBIAN"
sed -e "s/@VERSION@/$version/" -e "s/@SIZE@/$size/" packaging/debian/control > "$root/DEBIAN/control"
dpkg-deb --root-owner-group --build "$root" "target/debian/sklad_${version}-1_amd64.deb"
