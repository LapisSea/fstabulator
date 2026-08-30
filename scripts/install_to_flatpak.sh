#!/usr/bin/env bash
# Packages the prebuilt release binary into a .flatpak bundle and installs
# it per-user. The manifest only installs files (no cargo in the sandbox),
# so no rust extension or vendored dependencies are needed.
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
BRANCH="50"

for tool in cargo flatpak flatpak-builder; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		echo "error: $tool not found (on Fedora: sudo dnf install flatpak-builder)" >&2
		exit 1
	fi
done

missing=()
flatpak info "org.gnome.Platform//$BRANCH" >/dev/null 2>&1 || missing+=("org.gnome.Platform//$BRANCH")
flatpak info "org.gnome.Sdk//$BRANCH" >/dev/null 2>&1 || missing+=("org.gnome.Sdk//$BRANCH")
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

# 1. Build the release binary with the in-sandbox locale dir embedded.
# The binary's glibc requirement must stay <= the runtime's (runtime 50: 2.42),
# and its gtk4/libadwaita versions must exist in the runtime.
echo "==> cargo build --release (embedding LOCALEDIR=/app/share/locale)"
( cd "$ROOT_DIR" && LOCALEDIR=/app/share/locale cargo build --release )

# 2. Stage: binary, icon, desktop entry, compiled translations.
echo "==> staging"
rm -rf "$STAGE"
mkdir -p "$STAGE/resources"
cp -a "$ROOT_DIR/target/release/fstabulator" "$STAGE/fstabulator"
cp -a "$ROOT_DIR/resources/fstabulator_icon.svg" "$STAGE/resources/"
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
LOCALE_SRC="$(ls -1dt "$ROOT_DIR"/target/release/build/fstabulator-*/out/locale 2>/dev/null | head -n1 || true)"
if [[ -n "$LOCALE_SRC" ]]; then
	cp -a "$LOCALE_SRC" "$STAGE/locale"
else
	echo "note: no compiled translations found; bundle will ship without them" >&2
fi

# 3. Build and export into a local repo.
echo "==> flatpak build"
BUILD_DIR="$(mktemp -d "$ROOT_DIR/flatpak/build.XXXXXX")"
trap 'rm -rf "$BUILD_DIR"' EXIT
( cd "$ROOT_DIR" && flatpak-builder --force-clean --repo "$BUILD_DIR/repo" "$BUILD_DIR/app" "$MANIFEST" )
flatpak build-bundle "$BUILD_DIR/repo" "$BUILD_DIR/$APP_ID.flatpak" "$APP_ID" stable

# 4. Install per-user.
flatpak install --user -y "$BUILD_DIR/$APP_ID.flatpak"

# 5. Restore the dev binary's locale dir (the packaging build repointed it).
( cd "$ROOT_DIR" && cargo build --release )
echo "Installed $APP_ID (per-user). Note: privileged operations stay inside the sandbox."
