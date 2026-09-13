#!/usr/bin/env bash
# Build into dist/; pass --install to also install the package.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/package_common.sh"

package_init "$@"
require_tools makepkg fakeroot
if (( EUID == 0 )); then
	echo "error: makepkg must run as a regular user, not root" >&2
	exit 1
fi
build_payload "$STAGE/payload" licenses/fstabulator/LICENSE

{
	printf 'pkgver=%q\n' "${VERSION//-/_}"
	printf '_payload=%q\n' "$STAGE/payload"
	cat <<'PKGBUILD'
pkgname=fstabulator
pkgrel=1
pkgdesc='GTK4 GUI for editing /etc/fstab'
arch=('x86_64' 'aarch64')
url='https://github.com/LapisSea/fstabulator'
license=('GPL-3.0-or-later')
depends=('gtk4>=4.12' 'libadwaita>=1.9' 'glib2' 'polkit' 'util-linux')
optdepends=('btrfs-progs: btrfs filesystem support')
options=('!emptydirs' '!debug')
source=()
sha256sums=()

package() {
	cp -a "$_payload/." "$pkgdir/"
}
PKGBUILD
} > "$STAGE/PKGBUILD"

echo "==> makepkg (version $VERSION)"
( cd "$STAGE" && PKGDEST="$STAGE" PKGEXT=.pkg.tar.zst makepkg --noconfirm --needed )
collect_package "$STAGE" '*.pkg.tar.zst'

if $INSTALL; then
	sudo pacman -U --noconfirm "$BUILT"
else
	echo "Install with: sudo pacman -U '$BUILT'"
fi
