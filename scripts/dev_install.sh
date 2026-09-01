#!/usr/bin/env bash
set -euo pipefail

# The desktop entry and icons are per-user; only the polkit step below
# self-elevates. Running as root would install under /root and can clobber
# the polkit annotation with the wrong binary path.
if [[ $EUID -eq 0 ]]; then
	echo "error: run as a normal user, not with sudo (the polkit step uses sudo itself)" >&2
	exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

APP_ID="org.lapissea.FSTabulator"
ICON_NAME="fstabulator"

# Default to the dev build; pass a path to install a different binary,
# e.g. ./scripts/dev_install.sh /usr/local/bin/fstabulator
EXEC="${1:-$ROOT_DIR/target/debug/fstabulator}"

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPLICATIONS_DIR="$DATA_DIR/applications"
HICOLOR_DIR="$DATA_DIR/icons/hicolor"
DARK_THEME_DIR="$DATA_DIR/icons/Adwaita-dark"
ICON_DIR="scalable/apps"

mkdir -p "$APPLICATIONS_DIR" "$HICOLOR_DIR/$ICON_DIR" "$DARK_THEME_DIR/$ICON_DIR"

# Desktop entry
sed "s|^Exec=.*|Exec=$EXEC|" "$ROOT_DIR/resources/org.lapissea.FSTabulator.desktop" \
	> "$APPLICATIONS_DIR/$APP_ID.desktop"

# Launcher icon: the light variant in hicolor, which every desktop resolves.
cp "$ROOT_DIR/resources/fstabulator_icon.svg" "$HICOLOR_DIR/$ICON_DIR/$ICON_NAME.svg"

# Dark variant in a local Adwaita-dark theme: DEs that let the user pick the
# icon theme (KDE Plasma, XFCE) can swap to it; GNOME never does, which is fine.
cp "$ROOT_DIR/resources/fstabulator_icon_dark.svg" "$DARK_THEME_DIR/$ICON_DIR/$ICON_NAME.svg"
if [[ ! -f "$DARK_THEME_DIR/index.theme" ]]; then
	cp "$ROOT_DIR/resources/index.theme" "$DARK_THEME_DIR/index.theme"
fi

# Refresh icon and app databases. The user's hicolor dir may have no
# index.theme of its own, in which case building its cache fails; that's
# fine, GTK scans the directory directly.
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
	gtk-update-icon-cache -f -t "$HICOLOR_DIR" >/dev/null 2>&1 || true
	gtk-update-icon-cache -f -t "$DARK_THEME_DIR" >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
	update-desktop-database "$APPLICATIONS_DIR" >/dev/null 2>&1 || true
fi

# Polkit action: friendly text for the root helper's authentication dialog.
# Needs root because polkit only reads system-wide action directories.
POLKIT_ACTIONS_DIR="/usr/share/polkit-1/actions"
POLKIT_FILE="$POLKIT_ACTIONS_DIR/$APP_ID.root-helper.policy"

if [[ -f "$POLKIT_FILE" ]] && grep -qF "exec.path\">$EXEC<" "$POLKIT_FILE"; then
	POLKIT_STATUS="already up to date"
elif command -v sudo >/dev/null 2>&1; then
	sudo install -d "$POLKIT_ACTIONS_DIR"
	sed -e "s|<icon_name>[^<]*</icon_name>|<icon_name>$ICON_NAME</icon_name>|" \
		-e "s|<annotate key=\"org.freedesktop.policykit.exec.path\">[^<]*</annotate>|<annotate key=\"org.freedesktop.policykit.exec.path\">$EXEC</annotate>|" \
		"$ROOT_DIR/resources/org.lapissea.FSTabulator.root-helper.policy" \
		| sudo tee "$POLKIT_FILE" >/dev/null
	sudo chmod 0644 "$POLKIT_FILE"
	POLKIT_STATUS="installed"
else
	POLKIT_STATUS="skipped (no sudo; the auth dialog will show the default message)"
fi

echo "Installed:"
echo "  $APPLICATIONS_DIR/$APP_ID.desktop (Exec=$EXEC)"
echo "  $HICOLOR_DIR/$ICON_DIR/$ICON_NAME.svg"
echo "  $DARK_THEME_DIR/$ICON_DIR/$ICON_NAME.svg"
echo "  $POLKIT_FILE ($POLKIT_STATUS)"