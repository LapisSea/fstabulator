#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

APP_ID="org.lapissea.FSTabulator"
ICON_NAME="fstabulator"

# Default to the dev build; pass a path to install a different binary,
# e.g. ./scripts/install.sh /usr/local/bin/fstabulator
EXEC="${1:-$ROOT_DIR/target/debug/fstabulator}"

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPLICATIONS_DIR="$DATA_DIR/applications"
HICOLOR_DIR="$DATA_DIR/icons/hicolor"
DARK_THEME_DIR="$DATA_DIR/icons/Adwaita-dark"
ICON_DIR_512="512x512/apps"

mkdir -p "$APPLICATIONS_DIR" "$HICOLOR_DIR/$ICON_DIR_512" "$DARK_THEME_DIR/$ICON_DIR_512"

# Desktop entry
cat > "$APPLICATIONS_DIR/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=FSTabulator
Comment=Edit /etc/fstab
Exec=$EXEC
Icon=$ICON_NAME
Terminal=false
Categories=System;Utility;
EOF

# Light icon
cp "$ROOT_DIR/resources/fstabulator_icon.svg" "$HICOLOR_DIR/$ICON_DIR_512/$ICON_NAME.svg"

# Dark icon
rm -f "$DARK_THEME_DIR/$ICON_DIR_512/$ICON_NAME.png"
cp "$ROOT_DIR/resources/fstabulator_icon_dark.svg" "$DARK_THEME_DIR/$ICON_DIR_512/$ICON_NAME.svg"
if [[ ! -f "$DARK_THEME_DIR/index.theme" ]]; then
	cat > "$DARK_THEME_DIR/index.theme" <<EOF
[Icon Theme]
Name=Adwaita-dark
Inherits=Adwaita,hicolor
Directories=512x512/apps

[512x512/apps]
Size=512
Type=Fixed
Context=Apps
EOF
fi

# Refresh icon caches
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
	gtk-update-icon-cache -f -t "$HICOLOR_DIR" >/dev/null
	gtk-update-icon-cache -f -t "$DARK_THEME_DIR" >/dev/null
fi

# Polkit action: friendly text for the root helper's authentication dialog.
# Needs root because polkit only reads system-wide action directories.
POLKIT_ACTIONS_DIR="/usr/share/polkit-1/actions"
POLKIT_FILE="$POLKIT_ACTIONS_DIR/$APP_ID.root-helper.policy"

if [[ -f "$POLKIT_FILE" ]] && grep -qF "exec.path\">$EXEC<" "$POLKIT_FILE"; then
	POLKIT_STATUS="already up to date"
elif command -v sudo >/dev/null 2>&1; then
	sudo install -d "$POLKIT_ACTIONS_DIR"
	sudo tee "$POLKIT_FILE" >/dev/null <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1.0/policyconfig.dtd">
<policyconfig>
	<vendor>FSTabulator</vendor>
	<vendor_url>https://github.com/LapisSea/fstabulator</vendor_url>
	<icon_name>$APP_ID</icon_name>
	<action id="$APP_ID.root-helper">
		<description>FSTabulator is requesting system access</description>
		<message>FSTabulator needs administrator access to edit /etc/fstab, keep backups of it, and to mount, unmount or swap your drives.</message>
		<icon_name>$APP_ID</icon_name>
		<defaults>
			<allow_any>auth_admin</allow_any>
			<allow_inactive>auth_admin</allow_inactive>
			<allow_active>auth_admin</allow_active>
		</defaults>
		<annotate key="org.freedesktop.policykit.exec.path">$EXEC</annotate>
		<annotate key="org.freedesktop.policykit.exec.argv1">--root-helper</annotate>
	</action>
</policyconfig>
EOF
	sudo chmod 0644 "$POLKIT_FILE"
	POLKIT_STATUS="installed"
else
	POLKIT_STATUS="skipped (no sudo; the auth dialog will show the default message)"
fi

echo "Installed:"
echo "  $APPLICATIONS_DIR/$APP_ID.desktop (Exec=$EXEC)"
echo "  $HICOLOR_DIR/$ICON_DIR_512/$ICON_NAME.svg"
echo "  $DARK_THEME_DIR/$ICON_DIR_512/$ICON_NAME.svg"
echo "  $POLKIT_FILE ($POLKIT_STATUS)"