#!/usr/bin/env bash
# Regenerates po/fstabulator.pot from translatable strings in src/.
# Requires GNU gettext (xgettext). Translators copy the .pot to e.g. po/sv.po,
# translate it, and msgfmt (run by build.rs) compiles it automatically.
set -euo pipefail
cd "$(dirname "$0")/.."

xgettext \
	--language=Rust \
	--from-code=UTF-8 \
	--no-location \
	--package-name=fstabulator \
	--package-version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)" \
	--msgid-bugs-address=https://github.com/lapissea/fstabulator/issues \
	--keyword=i18n:1 \
	--keyword=i18n_fmt:1 \
	--keyword='opt!:3' \
	--add-comments=xgettext \
	-o po/fstabulator.pot \
	src/*.rs

echo "wrote po/fstabulator.pot"

# Non-destructive sync check: report when po files diverged from the new pot.
# Deliberately NOT auto-fixed here — review the pot diff first, then run
# `python3 scripts/po_tool.py reorder` (it drops entries that left the pot).
if ! python3 scripts/po_tool.py stats >/dev/null 2>&1; then
	echo "note: po files are out of sync with the pot (missing/empty entries or wrong order)."
	echo "      inspect: python3 scripts/po_tool.py missing"
	echo "      fix:     python3 scripts/po_tool.py reorder"
fi
