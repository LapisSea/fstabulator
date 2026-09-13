#!/usr/bin/env bash

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

package_init() {
	INSTALL=false
	case "$*" in
		'') ;;
		--install) INSTALL=true ;;
		-h|--help) echo "Usage: $0 [--install]"; exit 0 ;;
		*) echo "Usage: $0 [--install]" >&2; exit 1 ;;
	esac
	umask 022
	VERSION="$(sed -n '/^\[package\]/,/^\[/s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT_DIR/Cargo.toml")"
	if [[ -z "$VERSION" ]]; then
		echo "error: could not read the package version from Cargo.toml" >&2
		exit 1
	fi
	STAGE="$(mktemp -d)"
	trap 'rm -rf "$STAGE"' EXIT
	mkdir -p "$ROOT_DIR/dist"
}

require_tools() {
	local tool
	for tool in "$@"; do
		if ! command -v "$tool" >/dev/null 2>&1; then
			echo "error: required tool '$tool' not found" >&2
			exit 1
		fi
	done
}

build_payload() {
	local destination="$1" license_path="$2" host po lang
	require_tools cargo rustc msgfmt
	host="$(rustc -vV | sed -n 's/^host: //p')"
	echo "==> cargo build --release --locked (embedding LOCALEDIR=/usr/share/locale)"
	( cd "$ROOT_DIR" && LOCALEDIR=/usr/share/locale cargo build --release --locked \
		--target "$host" --target-dir "$ROOT_DIR/target/package" )
	install -Dm0755 "$ROOT_DIR/target/package/$host/release/fstabulator" "$destination/usr/bin/fstabulator"
	install -Dm0644 "$ROOT_DIR/resources/org.lapissea.FSTabulator.desktop" \
		"$destination/usr/share/applications/org.lapissea.FSTabulator.desktop"
	install -Dm0644 "$ROOT_DIR/resources/fstabulator_icon.svg" \
		"$destination/usr/share/icons/hicolor/scalable/apps/fstabulator.svg"
	install -Dm0644 "$ROOT_DIR/resources/fstabulator_icon_dark.svg" \
		"$destination/usr/share/icons/Adwaita-dark/scalable/apps/fstabulator.svg"
	install -Dm0644 "$ROOT_DIR/resources/index.theme" "$destination/usr/share/icons/Adwaita-dark/index.theme"
	install -Dm0644 "$ROOT_DIR/resources/org.lapissea.FSTabulator.root-helper.policy" \
		"$destination/usr/share/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy"
	install -Dm0644 "$ROOT_DIR/LICENSE" "$destination/usr/share/$license_path"
	for po in "$ROOT_DIR"/po/*.po; do
		[[ -f "$po" ]] || continue
		lang="$(basename "$po" .po)"
		mkdir -p "$destination/usr/share/locale/$lang/LC_MESSAGES"
		msgfmt --check -o "$destination/usr/share/locale/$lang/LC_MESSAGES/fstabulator.mo" "$po"
	done
}

collect_package() {
	local output_dir="$1" pattern="$2"
	local -a packages=()
	mapfile -d '' -t packages < <(find "$output_dir" -type f -name "$pattern" -print0)
	if [[ ${#packages[@]} != 1 ]]; then
		echo "error: expected one $pattern package, found ${#packages[@]}" >&2
		exit 1
	fi
	BUILT="$ROOT_DIR/dist/$(basename "${packages[0]}")"
	cp "${packages[0]}" "$BUILT"
	echo "Package ready: $BUILT"
}
