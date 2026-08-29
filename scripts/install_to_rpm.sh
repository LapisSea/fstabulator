#!/usr/bin/env bash
# Builds the release binary with cargo, then packages it (plus desktop entry,
# icons, polkit action and translations) into an RPM with rpmbuild.
#
#   ./scripts/install_to_rpm.sh             build the RPM into dist/
#   ./scripts/install_to_rpm.sh --install   build it, then `sudo dnf install` it
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

INSTALL=false
[[ ${1:-} == "--install" ]] && INSTALL=true

for tool in cargo rpmbuild; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "error: $tool not found (on Fedora: sudo dnf install rpm-build)" >&2
		exit 1
	fi
done

VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
if [[ -z "$VERSION" ]]; then
	echo "error: could not read the version from Cargo.toml" >&2
	exit 1
fi

TOPDIR="$(mktemp -d)"
trap 'rm -rf "$TOPDIR"' EXIT
mkdir -p "$TOPDIR"/SPECS "$TOPDIR"/BUILD "$TOPDIR"/RPMS "$TOPDIR"/SRPMS "$TOPDIR"/BUILDROOT

echo "==> cargo build --release (embedding LOCALEDIR=/usr/share/locale)"
( cd "$ROOT_DIR" && LOCALEDIR=/usr/share/locale cargo build --release )

# build.rs compiles the .mo files into OUT_DIR; stage the newest copy.
LOCALE_SRC="$(ls -1dt "$ROOT_DIR"/target/release/build/fstabulator-*/out/locale 2>/dev/null | head -n1 || true)"
mkdir -p "$TOPDIR/locales"
if [[ -n "$LOCALE_SRC" ]]; then
	cp -a "$LOCALE_SRC"/. "$TOPDIR/locales/"
fi

echo "==> rpmbuild (version $VERSION)"
rpmbuild --bb "$ROOT_DIR/fstabulator.spec" \
	--define "_topdir $TOPDIR" \
	--define "cargo_version $VERSION" \
	--define "fstab_srcdir $ROOT_DIR" \
	--define "fstab_locales $TOPDIR/locales"

RPM="$(find "$TOPDIR/RPMS" -name '*.rpm' | head -n1)"
if [[ -z "$RPM" ]]; then
	echo "error: rpmbuild produced no rpm" >&2
	exit 1
fi

mkdir -p "$ROOT_DIR/dist"
cp "$RPM" "$ROOT_DIR/dist/"
BUILT="$ROOT_DIR/dist/$(basename "$RPM")"

if $INSTALL; then
	sudo dnf install -y "$BUILT"
else
	echo "RPM ready: $BUILT"
	echo "Install with: sudo dnf install '$BUILT'"
fi
