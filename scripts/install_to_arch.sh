#!/usr/bin/env bash
# Builds the release binary with cargo, then packages it (plus desktop entry,
# icons, polkit action and translations) into an Arch package with makepkg.
#
#   ./scripts/install_to_arch.sh             build the package into dist/
#   ./scripts/install_to_arch.sh --install   build it, then `sudo pacman -U` it
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

INSTALL=false
[[ ${1:-} == "--install" ]] && INSTALL=true

for tool in cargo makepkg fakeroot; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "error: $tool not found (on Arch: sudo pacman -S base-devel rust)" >&2
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

LOCALE_SRC="$(ls -1dt "$ROOT_DIR"/target/release/build/fstabulator-*/out/locale 2>/dev/null | head -n1 || true)"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# Write the PKGBUILD.  Variables prefixed with \$ are expanded by makepkg;
# everything else (VERSION, ROOT_DIR, LOCALE_SRC) is expanded now.
cat > "$STAGE/PKGBUILD" <<PKGBUILD
# Maintainer: LapisSea <lapisea@users.noreply.github.com>
pkgname=fstabulator
pkgver=$VERSION
pkgrel=1
pkgdesc='GTK4 GUI for editing /etc/fstab'
arch=('x86_64')
url='https://github.com/LapisSea/fstabulator'
license=('GPL-3.0-or-later')
depends=('gtk4' 'libadwaita' 'glib2' 'polkit' 'util-linux')
optdepends=('btrfs-progs: btrfs filesystem support')
options=('!emptydirs')
source=()
sha256sums=()

package() {
	install -Dm0755 "$ROOT_DIR/target/release/fstabulator" "\$pkgdir/usr/bin/fstabulator"

	install -d "\$pkgdir/usr/share/applications"
	cat > "\$pkgdir/usr/share/applications/org.lapissea.FSTabulator.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=FSTabulator
Comment=Edit /etc/fstab
Exec=fstabulator
Icon=fstabulator
Terminal=false
Categories=System;Utility;
DESKTOP

	install -Dm0644 "$ROOT_DIR/resources/fstabulator_icon.svg" \
		"\$pkgdir/usr/share/icons/hicolor/scalable/apps/fstabulator.svg"

	install -Dm0644 "$ROOT_DIR/resources/fstabulator_icon_dark.svg" \
		"\$pkgdir/usr/share/icons/Adwaita-dark/scalable/apps/fstabulator.svg"
	cat > "\$pkgdir/usr/share/icons/Adwaita-dark/index.theme" <<'THEME'
[Icon Theme]
Name=Adwaita-dark
Inherits=Adwaita,hicolor
Directories=scalable/apps

[scalable/apps]
Context=Applications
Size=128
MinSize=8
MaxSize=512
Type=Scalable
THEME

	install -d "\$pkgdir/usr/share/polkit-1/actions"
	cat > "\$pkgdir/usr/share/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy" <<'POLICY'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1.0/policyconfig.dtd">
<policyconfig>
	<vendor>FSTabulator</vendor>
	<vendor_url>https://github.com/LapisSea/fstabulator</vendor_url>
	<icon_name>org.lapissea.FSTabulator</icon_name>
	<action id="org.lapissea.FSTabulator.root-helper">
		<description>FSTabulator is requesting system access</description>
		<message>FSTabulator needs administrator access to edit /etc/fstab, keep backups of it, and to mount, unmount or swap your drives.</message>
		<icon_name>org.lapissea.FSTabulator</icon_name>
		<defaults>
			<allow_any>auth_admin</allow_any>
			<allow_inactive>auth_admin</allow_inactive>
			<allow_active>auth_admin</allow_active>
		</defaults>
		<annotate key="org.freedesktop.policykit.exec.path">/usr/bin/fstabulator</annotate>
		<annotate key="org.freedesktop.policykit.exec.argv1">--root-helper</annotate>
	</action>
</policyconfig>
POLICY

	install -Dm0644 "$ROOT_DIR/LICENSE" "\$pkgdir/usr/share/licenses/\$pkgname/LICENSE"

	if [[ -n "$LOCALE_SRC" ]]; then
		for lang in "$LOCALE_SRC"/*; do
			[ -d "\$lang/LC_MESSAGES" ] || continue
			install -Dm0644 "\$lang/LC_MESSAGES/fstabulator.mo" \
				"\$pkgdir/usr/share/locale/\$(basename "\$lang")/LC_MESSAGES/fstabulator.mo"
		done
	fi
}
PKGBUILD

echo "==> makepkg (version $VERSION)"
( cd "$STAGE" && makepkg --noconfirm --needed )
PKG="$(find "$STAGE" -maxdepth 1 -name '*.pkg.tar.*' ! -name '*.sig' | head -n1)"
if [[ -z "$PKG" ]]; then
	echo "error: makepkg produced no package" >&2
	exit 1
fi

mkdir -p "$ROOT_DIR/dist"
cp "$PKG" "$ROOT_DIR/dist/"
BUILT="$ROOT_DIR/dist/$(basename "$PKG")"

if $INSTALL; then
	sudo pacman -U --noconfirm "$BUILT"
else
	echo "Package ready: $BUILT"
	echo "Install with: sudo pacman -U '$BUILT'"
fi

# Restore the dev binary's locale dir (the packaging build repointed it).
( cd "$ROOT_DIR" && cargo build --release )
