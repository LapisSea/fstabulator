#!/usr/bin/env bash
# Regenerates po/fstabulator.pot from translatable strings in src/.
# Requires GNU gettext (xgettext). Translators copy the .pot to e.g. po/sv.po,
# translate it, and msgfmt (run by build.rs) compiles it automatically.
set -euo pipefail
cd "$(dirname "$0")/.."

xgettext \
	--language=Rust \
	--from-code=UTF-8 \
	--package-name=fstabulator \
	--package-version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)" \
	--msgid-bugs-address=https://github.com/lapissea/fstabulator/issues \
	--keyword=i18n:1 \
	--keyword='opt!:3' \
	--add-comments=xgettext \
	-o po/fstabulator.pot \
	src/*.rs

echo "wrote po/fstabulator.pot"
