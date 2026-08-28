#!/usr/bin/env python3
"""po_tool.py — helper for the scripts/translate_language_instructions.md procedure.

Subcommands:
  split    <po> <outdir> <max_size> [--len]
           write group_NN.po chunks: as few as possible at most max_size units
           each, all the same size (differ by at most 1); --len adds a
           "# len: en=N hr=N ratio=X.XX" comment per entry (for review chunks);
           lengths are display width (CJK/full-width chars count as 2)
  apply    <po> <final.json>            merge JSON (true msgid -> true msgstr) into <po>, header preserved
  validate <po> <pot> [--lang CODE | --script NAME]
           full validation report; exit 1 on error. The script check only flags
           characters from a DIFFERENT writing system than the target language
           (Latin, digits, punctuation are always allowed). Defaults to LATIN.
  stats    [PO ...] [--pot POT]         per file: total units, empty msgstrs,
           units in pot but not in the file, units in the file but not in
           pot, order ok/wrong (ok = msgid sequence exactly matches pot).
           Defaults: all po/*.po vs po/fstabulator.pot. Exit 1 if any file
           is out of sync.
  missing  [PO ...] [--pot POT]         per file: list the pot msgids missing
           from the file, and the file msgids with an empty msgstr. Same
           defaults as stats. Exit 1 if any are found.
  reorder  [PO ...] [--pot POT]         rewrite each file in pot order: insert
           pot units absent from the file (empty msgstr, pot comments) and
           drop file units absent from the pot (each one printed). Header and
           existing translations are preserved. Run only after reviewing the
           pot diff.

All processing is on "true strings" (po escapes interpreted) and is fully
Unicode-safe (any script). Escaping happens exactly once, at write time.
Never run apply repeatedly on data that was already written by an earlier
apply — that re-escapes and doubles backslashes.
"""
import glob
import json
import re
import sys
import os

def parse_po(path):
	"""Return (header_lines, units). units: list of dict(comments, msgid, msgstr), true strings."""
	lines = open(path, encoding='utf-8').read().split('\n')
	units = []
	header_end = None
	i, n = 0, len(lines)

	def unescape(s):
		out, j = [], 0
		while j < len(s):
			c = s[j]
			if c == '\\' and j + 1 < len(s):
				out.append({'n': '\n', 't': '\t', '"': '"', '\\': '\\'}.get(s[j+1], s[j+1]))
				j += 2
			else:
				out.append(c)
				j += 1
		return ''.join(out)

	def parse_string(i, key):
		m = re.match(r'^%s "((?:[^"\\]|\\.)*)"' % key, lines[i])
		if not m:
			raise ValueError('%s:%d: expected %s, got %r' % (path, i + 1, key, lines[i]))
		parts = [m.group(1)]
		i += 1
		while i < n and re.match(r'^"(?:[^"\\]|\\.)*"$', lines[i]):
			parts.append(re.match(r'^"((?:[^"\\]|\\.)*)"$', lines[i]).group(1))
			i += 1
		return unescape(''.join(parts)), i

	while i < n:
		line = lines[i]
		if line.startswith('#') or line == '':
			i += 1
			continue
		if line.startswith('msgid '):
			comments = []
			k = i - 1
			while k >= 0 and (lines[k].startswith('#') or lines[k] == ''):
				if lines[k].startswith('#'):
					comments.insert(0, lines[k])
				k -= 1
			msgid, i = parse_string(i, 'msgid')
			if msgid == '':
				if header_end is not None:
					raise ValueError('%s: more than one header' % path)
				_, i = parse_string(i, 'msgstr')
				header_end = k + 1 if k >= 0 else i
				continue
			msgstr, i = parse_string(i, 'msgstr')
			units.append({'comments': comments, 'msgid': msgid, 'msgstr': msgstr})
		else:
			i += 1
	header = [l for l in lines[:header_end] if l != ''] if header_end is not None else []
	return header, units

def escape(s):
	return s.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n').replace('\t', '\\t')

def fix_esc(s):
	"""Interpret leftover po-style escapes in an agent-supplied string (defensive)."""
	out, j = [], 0
	while j < len(s):
		c = s[j]
		if c == '\\' and j + 1 < len(s):
			out.append({'n': '\n', 't': '\t', '"': '"', '\\': '\\'}.get(s[j+1], s[j+1]))
			j += 2
		else:
			out.append(c)
			j += 1
	return ''.join(out)

def render_po(header, units):
	out = list(header) + ['']
	for u in units:
		out.extend(u['comments'])
		out.append('msgid "%s"' % escape(u['msgid']))
		out.append('msgstr "%s"' % escape(u['msgstr']))
		out.append('')
	return '\n'.join(out).rstrip('\n') + '\n'

def write_po(path, header, units):
	with open(path, 'w', encoding='utf-8') as f:
		f.write(render_po(header, units))

def equal_chunk_sizes(n, max_size):
	"""Split n items into the fewest chunks of at most max_size, as equal as
	possible (sizes differ by at most 1). E.g. n=160, max_size=50 -> [40,40,40,40]."""
	if n == 0:
		return []
	k = (n + max_size - 1) // max_size
	base, rem = divmod(n, k)
	return [base + 1] * rem + [base] * (k - rem)

# --- scripts ---------------------------------------------------------------

# Unicode ranges (lo, hi) per writing system. ASCII letters/digits and common
# punctuation are treated as neutral and always allowed (see is_allowed).
SCRIPT_RANGES = {
	'LATIN': [(0x00c0, 0x024f), (0x1e00, 0x1eff), (0x2c60, 0x2c7f), (0xa720, 0xa7ff)],
	'CYRILLIC': [(0x0400, 0x052f)],
	'GREEK': [(0x0370, 0x03ff)],
	'ARMENIAN': [(0x0530, 0x058f)],
	'HEBREW': [(0x0590, 0x05ff)],
	'ARABIC': [(0x0600, 0x06ff), (0x0750, 0x077f)],
	'THAI': [(0x0e00, 0x0e7f)],
	'LAO': [(0x0e80, 0x0eff)],
	'KHMER': [(0x1780, 0x17ff)],
	'GEORGIAN': [(0x10a0, 0x10ff)],
	'DEVANAGARI': [(0x0900, 0x097f)],
	'BENGALI': [(0x0980, 0x09ff)],
	'GURMUKHI': [(0x0a00, 0x0a7f)],
	'GUJARATI': [(0x0a80, 0x0aff)],
	'ORIYA': [(0x0b00, 0x0b7f)],
	'TAMIL': [(0x0b80, 0x0bff)],
	'TELUGU': [(0x0c00, 0x0c7f)],
	'KANNADA': [(0x0c80, 0x0cff)],
	'MALAYALAM': [(0x0d00, 0x0d7f)],
	'CJK': [(0x3000, 0x303f), (0x3040, 0x30ff), (0x3130, 0x318f), (0x3400, 0x4dbf), (0x4e00, 0x9fff),
	       (0xac00, 0xd7a3), (0xf900, 0xfaff), (0xff00, 0xffef), (0x20000, 0x2a6df)],
}

# language code -> script name (for --lang)
LANG_SCRIPT = {
	'en': 'LATIN', 'fr': 'LATIN', 'de': 'LATIN', 'es': 'LATIN', 'it': 'LATIN',
	'nl': 'LATIN', 'pt': 'LATIN', 'hr': 'LATIN', 'sl': 'LATIN', 'cs': 'LATIN',
	'sk': 'LATIN', 'pl': 'LATIN', 'sv': 'LATIN', 'nb': 'LATIN', 'nn': 'LATIN',
	'da': 'LATIN', 'fi': 'LATIN', 'et': 'LATIN', 'lv': 'LATIN', 'lt': 'LATIN',
	'ro': 'LATIN', 'hu': 'LATIN', 'ca': 'LATIN', 'gl': 'LATIN', 'eu': 'LATIN',
	'ru': 'CYRILLIC', 'uk': 'CYRILLIC', 'be': 'CYRILLIC', 'bg': 'CYRILLIC',
	'sr': 'CYRILLIC', 'mk': 'CYRILLIC',
	'el': 'GREEK', 'hy': 'ARMENIAN', 'ka': 'GEORGIAN', 'he': 'HEBREW',
	'ar': 'ARABIC', 'fa': 'ARABIC', 'ur': 'ARABIC',
	'zh': 'CJK', 'ja': 'CJK', 'ko': 'CJK',
	'th': 'THAI', 'lo': 'LAO', 'km': 'KHMER',
	'hi': 'DEVANAGARI', 'bn': 'BENGALI', 'pa': 'GURMUKHI', 'gu': 'GUJARATI',
	'or': 'ORIYA', 'ta': 'TAMIL', 'te': 'TELUGU', 'kn': 'KANNADA', 'ml': 'MALAYALAM',
}

def resolve_script(lang=None, script=None):
	if script:
		s = script.upper()
		if s not in SCRIPT_RANGES:
			raise SystemExit('unknown script %r; choose from %s' % (script, ', '.join(sorted(SCRIPT_RANGES))))
		return s
	if lang:
		s = LANG_SCRIPT.get(lang.lower())
		if s is None:
			raise SystemExit('unknown language %r; use --script to set it manually' % lang)
		return s
	return 'LATIN'

def is_allowed(o, ranges):
	# neutral: printable ASCII, nbsp, general punctuation (…, ‘, ’, “, ”, –, —),
	# a few symbols — allowed in every language
	if (0x20 <= o <= 0x7e) or o == 0xa0 or (0x2000 <= o <= 0x206f) or o in (0xb7, 0xd7, 0x20ac):
		return True
	for lo, hi in ranges:
		if lo <= o <= hi:
			return True
	return False

# --- display width ---------------------------------------------------------

WIDE = [(0x1100, 0x115f), (0x2e80, 0x303e), (0x3041, 0x33ff), (0x3400, 0x4dbf),
        (0x4e00, 0x9fff), (0xa000, 0xa4cf), (0xac00, 0xd7a3), (0xf900, 0xfaff),
        (0xfe30, 0xfe4f), (0xff00, 0xff60), (0xffe0, 0xffe6), (0x20000, 0x3fffd)]

def disp_len(s):
	"""Approximate display width: full-width (CJK) chars count as 2, others 1."""
	w = 0
	for ch in s:
		o = ord(ch)
		w += 2 if any(lo <= o <= hi for lo, hi in WIDE) else 1
	return w

# --- commands --------------------------------------------------------------

def cmd_split(po, outdir, max_size, annotate):
	header, units = parse_po(po)
	sizes = equal_chunk_sizes(len(units), max_size)
	os.makedirs(outdir, exist_ok=True)
	manifest = []
	start = 0
	for gi, size in enumerate(sizes, 1):
		chunk = units[start:start + size]
		start += size
		fn = os.path.join(outdir, 'group_%02d.po' % gi)
		with open(fn, 'w', encoding='utf-8') as f:
			for u in chunk:
				f.write('\n'.join(u['comments']) + '\n' if u['comments'] else '')
				if annotate:
					en, hr = disp_len(u['msgid']), disp_len(u['msgstr'])
					f.write('# len: en=%d hr=%d ratio=%.2f\n' % (en, hr, hr / en if en else 0.0))
				f.write('msgid "%s"\n' % escape(u['msgid']))
				f.write('msgstr "%s"\n' % escape(u['msgstr']))
				f.write('\n')
		manifest.append('%02d %d' % (gi, size))
	print('units: %d, chunks: %s' % (len(units), manifest))

def cmd_apply(po, js):
	data = json.load(open(js, encoding='utf-8'))
	lookup = {}
	for k, v in data.items():
		if v and str(v).strip():
			lookup[fix_esc(k)] = fix_esc(v)
	header, units = parse_po(po)
	applied = changed = 0
	missed = []
	for u in units:
		if u['msgid'] in lookup:
			if lookup[u['msgid']] != u['msgstr']:
				changed += 1
			u['msgstr'] = lookup[u['msgid']]
			applied += 1
		else:
			missed.append(u['msgid'])
	write_po(po, header, units)
	print('applied %d, changed %d, not in json %d' % (applied, changed, len(missed)))

def cmd_validate(po, pot, lang=None, script=None):
	script_name = resolve_script(lang, script)
	ranges = SCRIPT_RANGES[script_name]
	_, pot_units = parse_po(pot)
	_, units = parse_po(po)
	errs = 0
	ph = re.compile(r'\{(\w+)\}')
	ratios = []
	if len(pot_units) != len(units):
		print('COUNT MISMATCH: pot %d, po %d' % (len(pot_units), len(units)))
		errs += 1
	for i, (p, u) in enumerate(zip(pot_units, units)):
		mid, ms = p['msgid'], u['msgstr']
		if p['msgid'] != u['msgid']:
			print('unit %d: msgid mismatch\n  pot: %r\n  po : %r' % (i, p['msgid'][:80], u['msgid'][:80]))
			errs += 1
			continue
		if not ms.strip():
			print('unit %d: empty msgstr: %r' % (i, mid[:80]))
			errs += 1
		if '\\' in u['msgid'] or '\\' in ms:
			print('unit %d: leftover backslash: %r' % (i, mid[:80]))
			errs += 1
		if sorted(ph.findall(p['msgid'])) != sorted(ph.findall(ms)):
			print('unit %d: placeholder mismatch %r -> %r' % (i, ph.findall(p['msgid']), ph.findall(ms)))
			errs += 1
		if p['msgid'].count('<b>') + p['msgid'].count('</b>') != ms.count('<b>') + ms.count('</b>'):
			print('unit %d: markup count mismatch: %r' % (i, mid[:80]))
			errs += 1
		if p['msgid'].count('\u2026') != ms.count('\u2026'):
			print('unit %d: ellipsis count mismatch: %r' % (i, mid[:80]))
			errs += 1
		for ch in ms:
			o = ord(ch)
			if o > 0x7f and not is_allowed(o, ranges):
				print('unit %d: foreign-script char U+%04X in %r' % (i, o, ms[:60]))
				errs += 1
				break
		en, hr = disp_len(p['msgid']), disp_len(ms)
		if en:
			ratios.append(hr / en)
	print('units: %d | script: %s | errors: %d' % (len(units), script_name, errs))
	if ratios:
		import statistics
		over = [(r, u['msgid']) for u, r in zip(units, ratios) if r > 1.5 and disp_len(u['msgid']) > 15]
		print('length ratio (display width): median %.2f, mean %.2f, max %.2f | >1.5x (len>15): %d'
		      % (statistics.median(ratios), statistics.mean(ratios), max(ratios), len(over)))
		for r, m in sorted(over, reverse=True)[:10]:
			print('  %4.2f  %r' % (r, m[:70]))
	sys.exit(1 if errs else 0)

def default_files_and_pot(args):
	pot_path = 'po/fstabulator.pot'
	if '--pot' in args:
		pot_path = args[args.index('--pot') + 1]
	pos = [a for a in args if a != '--pot' and a != pot_path]
	files = pos if pos else sorted(glob.glob('po/*.po'))
	if not files:
		raise SystemExit('no po files found; pass file paths or run from the repo root')
	return files, pot_path

def cmd_stats(files, pot_path):
	_, pot_units = parse_po(pot_path)
	pot_ids = [u['msgid'] for u in pot_units]
	pot_set = set(pot_ids)
	print('%-16s %6s %6s %8s %6s %8s' % ('file', 'units', 'empty', 'missing', 'extra', 'order'))
	out_of_sync = 0
	for f in files:
		_, units = parse_po(f)
		ids = [u['msgid'] for u in units]
		id_set = set(ids)
		empty = sum(1 for u in units if not u['msgstr'].strip())
		missing = len(pot_set - id_set)
		extra = len(id_set - pot_set)
		order_ok = ids == pot_ids
		if empty or missing or extra or not order_ok:
			out_of_sync += 1
		print('%-16s %6d %6d %8d %6d %8s'
		      % (f, len(ids), empty, missing, extra, 'ok' if order_ok else 'WRONG'))
	sys.exit(1 if out_of_sync else 0)

def cmd_missing(files, pot_path):
	_, pot_units = parse_po(pot_path)
	found = 0
	for f in files:
		_, units = parse_po(f)
		id_set = set(u['msgid'] for u in units)
		missing = [u['msgid'] for u in pot_units if u['msgid'] not in id_set]
		empty = [u['msgid'] for u in units if not u['msgstr'].strip()]
		print('%s' % f)
		print('  missing from po (%d):' % len(missing))
		for m in missing:
			print('    %r' % m)
		print('  empty msgstr (%d):' % len(empty))
		for m in empty:
			print('    %r' % m)
		found += bool(missing or empty)
	sys.exit(1 if found else 0)

def cmd_reorder(files, pot_path):
	_, pot_units = parse_po(pot_path)
	pot_by_id = {u['msgid']: u for u in pot_units}
	for f in files:
		header, units = parse_po(f)
		po_by_id = {u['msgid']: u for u in units}
		removed = [u['msgid'] for u in units if u['msgid'] not in pot_by_id]
		added = 0
		new_units = []
		for pu in pot_units:
			if pu['msgid'] in po_by_id:
				ou = po_by_id[pu['msgid']]
				new_units.append({'comments': ou['comments'], 'msgid': pu['msgid'], 'msgstr': ou['msgstr']})
			else:
				new_units.append({'comments': pu['comments'], 'msgid': pu['msgid'], 'msgstr': ''})
				added += 1
		text = render_po(header, new_units)
		if text == open(f, encoding='utf-8').read():
			print('%s: no change' % f)
		else:
			with open(f, 'w', encoding='utf-8') as fh:
				fh.write(text)
			print('%s: reordered (units=%d added=%d removed=%d)' % (f, len(new_units), added, len(removed)))
		for m in removed:
			print('  removed: %r' % m)

if __name__ == '__main__':
	if len(sys.argv) < 2:
		print(__doc__)
		sys.exit(2)
	cmd = sys.argv[1]
	args = sys.argv[2:]
	if cmd == 'split':
		annotate = '--len' in args
		pos = [a for a in args if a != '--len']
		cmd_split(pos[0], pos[1], int(pos[2]), annotate)
	elif cmd == 'apply':
		cmd_apply(args[0], args[1])
	elif cmd == 'validate':
		lang = script = None
		if '--lang' in args:
			lang = args[args.index('--lang') + 1]
		if '--script' in args:
			script = args[args.index('--script') + 1]
		pos = [a for a in args if a not in ('--lang', '--script')
		       and (a != lang) and (a != script)]
		cmd_validate(pos[0], pos[1], lang, script)
	elif cmd in ('stats', 'missing', 'reorder'):
		files, pot_path = default_files_and_pot(args)
		{'stats': cmd_stats, 'missing': cmd_missing, 'reorder': cmd_reorder}[cmd](files, pot_path)
	else:
		print('unknown command %r' % cmd)
		sys.exit(2)
