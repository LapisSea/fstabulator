#!/usr/bin/env bash
# Builds the release binary with cargo, then packages it (plus desktop entry,
# icons, polkit action and translations) into a .deb with dpkg-deb.
#
#   ./scripts/install_to_deb.sh             build the .deb into dist/
#   ./scripts/install_to_deb.sh --install   build it, then `sudo apt install` it
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

INSTALL=false
[[ ${1:-} == "--install" ]] && INSTALL=true

for tool in cargo dpkg-deb; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "error: $tool not found (on Debian/Ubuntu: sudo apt install dpkg-dev)" >&2
		exit 1
	fi
done

VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
if [[ -z "$VERSION" ]]; then
	echo "error: could not read the version from Cargo.toml" >&2
	exit 1
fi

echo "==> cargo build --release (embedding LOCALEDIR=/usr/share/locale)"
( cd "$ROOT_DIR" && LOCALEDIR=/usr/share/locale cargo build --release )

# build.rs compiles the .mo files into OUT_DIR; stage the newest copy.
LOCALE_SRC="$(ls -1dt "$ROOT_DIR"/target/release/build/fstabulator-*/out/locale 2>/dev/null | head -n1 || true)"

PACKAGE="fstabulator_${VERSION}-1_amd64"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
DEB_DIR="$STAGE/$PACKAGE"
mkdir -p "$DEB_DIR/DEBIAN" \
	"$DEB_DIR/usr/bin" \
	"$DEB_DIR/usr/share/applications" \
	"$DEB_DIR/usr/share/icons/hicolor/scalable/apps" \
	"$DEB_DIR/usr/share/icons/Adwaita-dark/scalable/apps" \
	"$DEB_DIR/usr/share/polkit-1/actions" \
	"$DEB_DIR/usr/share/doc/fstabulator"

install -m 0755 "$ROOT_DIR/target/release/fstabulator" "$DEB_DIR/usr/bin/fstabulator"

install -m 0644 "$ROOT_DIR/resources/org.lapissea.FSTabulator.desktop" \
	"$DEB_DIR/usr/share/applications/org.lapissea.FSTabulator.desktop"

install -m 0644 "$ROOT_DIR/resources/fstabulator_icon.svg" \
	"$DEB_DIR/usr/share/icons/hicolor/scalable/apps/fstabulator.svg"
install -m 0644 "$ROOT_DIR/resources/fstabulator_icon_dark.svg" \
	"$DEB_DIR/usr/share/icons/Adwaita-dark/scalable/apps/fstabulator.svg"
install -m 0644 "$ROOT_DIR/resources/index.theme" \
	"$DEB_DIR/usr/share/icons/Adwaita-dark/index.theme"

install -m 0644 "$ROOT_DIR/resources/org.lapissea.FSTabulator.root-helper.policy" \
	"$DEB_DIR/usr/share/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy"

install -m 0644 "$ROOT_DIR/LICENSE" "$DEB_DIR/usr/share/doc/fstabulator/copyright"

if [[ -n "$LOCALE_SRC" ]]; then
	for lang in "$LOCALE_SRC"/*; do
		[ -d "$lang/LC_MESSAGES" ] || continue
		install -d "$DEB_DIR/usr/share/locale/$(basename "$lang")/LC_MESSAGES"
		install -m 0644 "$lang/LC_MESSAGES/fstabulator.mo" \
			"$DEB_DIR/usr/share/locale/$(basename "$lang")/LC_MESSAGES/fstabulator.mo"
	done
else
	echo "note: no compiled translations found; package will ship without them" >&2
fi

SIZE="$(du -sk "$DEB_DIR/usr" 2>/dev/null | cut -f1)"
cat > "$DEB_DIR/DEBIAN/control" <<EOF
Package: fstabulator
Version: $VERSION-1
Section: utils
Priority: optional
Architecture: amd64
Installed-Size: ${SIZE:-0}
Maintainer: LapisSea <lapisea@users.noreply.github.com>
Depends: libgtk-4-1 (>= 4.12), libadwaita-1-0 (>= 1.9), libglib2.0-0, polkitd, pkexec, util-linux
Suggests: btrfs-progs
Description: GTK4 GUI for editing /etc/fstab
 FSTabulator is a GTK4/libadwaita front end for /etc/fstab. It lists the
 block devices and filesystems on the system, edits mount entries, keeps
 timestamped backups, and applies changes (mount, remount, unmount, swap)
 through a polkit-authenticated root helper.
EOF

echo "==> dpkg-deb (version $VERSION)"
DEB="$(cd "$STAGE" && dpkg-deb --build --root-owner-group "$PACKAGE")"
DEB="$STAGE/$PACKAGE.deb"

mkdir -p "$ROOT_DIR/dist"
cp "$DEB" "$ROOT_DIR/dist/"
BUILT="$ROOT_DIR/dist/$(basename "$DEB")"

if $INSTALL; then
	sudo apt install -y "$BUILT"
else
	echo "Deb ready: $BUILT"
	echo "Install with: sudo apt install '$BUILT'"
fi

# Restore the dev binary's locale dir (the packaging build repointed it).
( cd "$ROOT_DIR" && cargo build --release )
