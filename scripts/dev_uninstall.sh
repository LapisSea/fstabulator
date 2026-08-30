#!/usr/bin/env bash
# Inverse of scripts/dev_install.sh: removes the dev install from the user's XDG
# dirs (desktop entry, both icon variants in current and legacy locations,
# the script-written index.theme, stale caches) and the polkit action when
# dev_install.sh created it. The build tree (target/) is untouched, and files
# owned by an installed package are never removed.
set -euo pipefail

APP_ID="org.lapissea.FSTabulator"
ICON_NAME="fstabulator"

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APPLICATIONS_DIR="$DATA_DIR/applications"
HICOLOR_DIR="$DATA_DIR/icons/hicolor"
DARK_THEME_DIR="$DATA_DIR/icons/Adwaita-dark"

if rpm -q fstabulator >/dev/null 2>&1; then
	echo "note: the fstabulator package is installed; its files live in /usr and go away with: dnf remove fstabulator"
fi

remove_file() {
	if [[ -e $1 ]]; then
		rm -f "$1"
		echo "removed:  $1"
	else
		echo "absent:   $1"
	fi
}

remove_file "$APPLICATIONS_DIR/$APP_ID.desktop"
remove_file "$HICOLOR_DIR/scalable/apps/$ICON_NAME.svg"
remove_file "$HICOLOR_DIR/512x512/apps/$ICON_NAME.svg"
remove_file "$DARK_THEME_DIR/scalable/apps/$ICON_NAME.svg"
remove_file "$DARK_THEME_DIR/512x512/apps/$ICON_NAME.svg"

# Only drop the index.theme this project's install script wrote (current or
# legacy layout); a user-maintained Adwaita-dark theme is left alone.
if [[ -f "$DARK_THEME_DIR/index.theme" ]]; then
	expected="$(cat <<'EOF'
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
EOF
)"
	if [[ "$(cat "$DARK_THEME_DIR/index.theme")" == "$expected" ]] \
		|| { grep -q '^Directories=512x512/apps' "$DARK_THEME_DIR/index.theme" \
			&& grep -q '^Inherits=Adwaita,hicolor' "$DARK_THEME_DIR/index.theme"; }; then
		rm -f "$DARK_THEME_DIR/index.theme"
		echo "removed:  $DARK_THEME_DIR/index.theme"
	else
		echo "kept:     $DARK_THEME_DIR/index.theme (not written by dev_install.sh)"
	fi
fi

# Caches reference the removed icons; drop them, they are regenerated.
rm -f "$HICOLOR_DIR/icon-theme.cache" "$DARK_THEME_DIR/icon-theme.cache"

# Prune directories the install created, if left empty (bottom-up).
for d in "$HICOLOR_DIR/scalable/apps" "$HICOLOR_DIR/scalable" \
	"$HICOLOR_DIR/512x512/apps" "$HICOLOR_DIR/512x512" \
	"$DARK_THEME_DIR/scalable/apps" "$DARK_THEME_DIR/scalable" \
	"$DARK_THEME_DIR/512x512/apps" "$DARK_THEME_DIR/512x512" \
	"$DARK_THEME_DIR"; do
	rmdir "$d" 2>/dev/null || true
done

# Refresh caches of whatever remains.
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
	for d in "$HICOLOR_DIR" "$DARK_THEME_DIR"; do
		[[ -d $d ]] && gtk-update-icon-cache -f -t "$d" >/dev/null 2>&1 || true
	done
fi
if command -v update-desktop-database >/dev/null 2>&1 && [[ -d "$APPLICATIONS_DIR" ]]; then
	update-desktop-database "$APPLICATIONS_DIR" >/dev/null 2>&1 || true
fi

# Polkit action: remove only when dev_install.sh created it (no package owns it).
POLKIT_FILE="/usr/share/polkit-1/actions/$APP_ID.root-helper.policy"
if [[ -e "$POLKIT_FILE" ]]; then
	if pkg="$(rpm -qf "$POLKIT_FILE" 2>/dev/null)"; then
		echo "note:     $POLKIT_FILE is owned by $pkg; remove it with: dnf remove $pkg"
	else
		sudo rm -f "$POLKIT_FILE"
		echo "removed:  $POLKIT_FILE"
	fi
else
	echo "absent:   $POLKIT_FILE"
fi

echo "done. The system no longer knows the dev version; the RPM can be installed with:"
echo "  sudo dnf install <dist/fstabulator-*.rpm>"
