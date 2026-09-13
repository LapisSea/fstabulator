#!/usr/bin/env bash
# Build into dist/; pass --install to also install the package.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/package_common.sh"

package_init "$@"
require_tools dpkg dpkg-deb
ARCH="$(dpkg --print-architecture)"
PACKAGE="fstabulator_${VERSION//-/\~}-1_${ARCH}"
DEB_DIR="$STAGE/$PACKAGE"
build_payload "$DEB_DIR" doc/fstabulator/copyright
mkdir -p "$DEB_DIR/DEBIAN"

SIZE="$(du -sk "$DEB_DIR/usr" 2>/dev/null | cut -f1)"
cat > "$DEB_DIR/DEBIAN/control" <<EOF
Package: fstabulator
Version: ${VERSION//-/\~}-1
Section: utils
Priority: optional
Architecture: $ARCH
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
dpkg-deb --build --root-owner-group "$DEB_DIR"
collect_package "$STAGE" '*.deb'

if $INSTALL; then
	sudo apt install -y "$BUILT"
else
	echo "Install with: sudo apt install '$BUILT'"
fi
