#!/usr/bin/env bash
# Vendors the cargo dependencies, stages the source, builds
# org.lapissea.FSTabulator into a .flatpak bundle and installs it per-user.
#
# Sandbox caveat: the app's privileged operations (saving /etc/fstab,
# mount/remount/unmount, swap, backups, credential files) run inside the
# flatpak sandbox and do not reach the host. For a fully working install,
# use the RPM (scripts/install_to_rpm.sh).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$ROOT_DIR/org.lapissea.FSTabulator.json"
STAGE="$ROOT_DIR/flatpak/source"
APP_ID="org.lapissea.FSTabulator"
SDK_BRANCH="50"

for tool in cargo flatpak; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "error: $tool not found" >&2
		exit 1
	fi
done

missing=()
flatpak info "org.gnome.Platform//$SDK_BRANCH" >/dev/null 2>&1 || missing+=("org.gnome.Platform//$SDK_BRANCH")
flatpak info "org.gnome.Sdk//$SDK_BRANCH" >/dev/null 2>&1 || missing+=("org.gnome.Sdk//$SDK_BRANCH")
flatpak info "org.gnome.Sdk.Extension.rust.cargo//$SDK_BRANCH" >/dev/null 2>&1 || missing+=("org.gnome.Sdk.Extension.rust.cargo//$SDK_BRANCH")
if [[ ${#missing[@]} -gt 0 ]]; then
	echo "error: missing flatpak runtimes:" >&2
	printf '  %s\n' "${missing[@]}" >&2
	echo "Install them with:" >&2
	echo "  sudo flatpak install flathub ${missing[*]}" >&2
	exit 1
fi

VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)"
if [[ -z "$VERSION" ]]; then
	echo "error: could not read the version from Cargo.toml" >&2
	exit 1
fi

# 1. Stage a clean source tree (never includes target/).
echo "==> staging source (version $VERSION)"
rm -rf "$STAGE"
mkdir -p "$STAGE/.cargo"
cp -a Cargo.toml Cargo.lock build.rs src resources po LICENSE README.md "$STAGE/"
cat > "$STAGE/.cargo/config.toml" <<'EOF'
[source.crates-io]
replace-with = "vendored"

[source.vendored]
directory = "vendor"
EOF
cat > "$STAGE/org.lapissea.FSTabulator.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=FSTabulator
Comment=Edit /etc/fstab
Exec=fstabulator
Icon=org.lapissea.FSTabulator
Terminal=false
Categories=System;Utility;
EOF

# 2. Vendor the dependencies into the staging dir: the build sandbox has no
# network access.
echo "==> cargo vendor"
( cd "$ROOT_DIR" && cargo vendor --locked "$STAGE/vendor" )

# 3. Build the bundle.
echo "==> flatpak build"
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT
( cd "$ROOT_DIR" && flatpak build --force-clean "$BUILD_DIR/app" "$MANIFEST" )
flatpak build-bundle "$BUILD_DIR/$APP_ID.flatpak" "$BUILD_DIR/app" "$APP_ID"

# 4. Install per-user.
flatpak install --user -y "$BUILD_DIR/$APP_ID.flatpak"
echo "Installed $APP_ID (per-user). Note: privileged operations stay inside the sandbox."
