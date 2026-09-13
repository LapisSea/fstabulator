#!/usr/bin/env bash
# Build into dist/; pass --install to also install the package.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/package_common.sh"

package_init "$@"
require_tools rpmbuild
build_payload "$STAGE/payload" licenses/fstabulator/LICENSE
mkdir -p "$STAGE"/{SPECS,BUILD,RPMS,SRPMS,BUILDROOT}

echo "==> rpmbuild (version $VERSION)"
rpmbuild -bb "$ROOT_DIR/fstabulator.spec" \
	--define "_topdir $STAGE" \
	--define "cargo_version ${VERSION//-/\~}" \
	--define "fstab_payload $STAGE/payload"
collect_package "$STAGE/RPMS" '*.rpm'

if $INSTALL; then
	sudo dnf install -y "$BUILT"
else
	echo "Install with: sudo dnf install '$BUILT'"
fi
