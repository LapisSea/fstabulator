#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

APP_ID="org.lapissea.FSTabulator"
ICON_NAME="fstabulator"

# Default to the dev build; pass a path to install a different binary,
# e.g. ./install.sh /usr/local/bin/fstabulator
EXEC="${1:-$SCRIPT_DIR/target/debug/fstabulator}"

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
cp "$SCRIPT_DIR/resources/fstabulator_icon.png" "$HICOLOR_DIR/$ICON_DIR_512/$ICON_NAME.png"

# Dark icon
cp "$SCRIPT_DIR/resources/fstabulator_icon_dark.png" "$DARK_THEME_DIR/$ICON_DIR_512/$ICON_NAME.png"
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

echo "Installed:"
echo "  $APPLICATIONS_DIR/$APP_ID.desktop (Exec=$EXEC)"
echo "  $HICOLOR_DIR/$ICON_DIR_512/$ICON_NAME.png"
echo "  $DARK_THEME_DIR/$ICON_DIR_512/$ICON_NAME.png"