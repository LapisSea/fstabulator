# Translating a New Language (po/<lang>.po)

Procedure for producing a complete, valid `po/<lang>.po` for the target
language `<lang>` (2-letter ISO code). Written to be followed with no prior
context: every rule and command is in this file; the helper script is
`scripts/po_tool.py` in the repository.

The target may be **any language and script** — Latin, Cyrillic, Greek,
CJK (zh/ja/ko), Hebrew, Arabic, Thai, Devanagari, … All files are UTF-8,
the tooling is Unicode-safe, and the placeholder/escaping rules are
script-agnostic. Only two things change with the script: which characters
the validator accepts, and how length is interpreted (see step 3/4).

## Background

- Translatable strings are collected with `bash scripts/update-pot.sh`
  (xgettext over `src/*.rs`, keywords `i18n:1`, `i18n_fmt:1`, `opt!:3`)
  into `po/fstabulator.pot`.
- `build.rs` compiles every `po/*.po` with `msgfmt --check -o ...` during
  `cargo build`. A broken .po means a cargo warning and a missing catalog —
  the app silently falls back to English.
- `src/i18n.rs`: `i18n()` is plain gettext; `i18n_fmt()` substitutes
  `{token}` placeholders by **literal string replace** after translation.
  A placeholder that is missing, renamed, or moved breaks the UI. This is
  script-agnostic: `{…}` tokens must survive translation untouched in any
  language.
- The product is a GTK4/libadwaita GUI for editing `/etc/fstab`. Every
  translatable string is **GUI text**: a label, button, dialog text,
  tooltip, or error message. Keep that register in mind when translating.
  RTL languages (ar, he, fa, ur) need nothing special in the .po — GTK
  handles text direction.

## PO format rules (most bugs come from here — read carefully)

### Escaping

- Inside a double-quoted po string, the only escapes are:
  `\"` (quote), `\\` (backslash), `\n` (newline), `\t` (tab).
- A **true string** is a string with escapes interpreted. ALL processing —
  matching, comparing, translating, JSON — happens on true strings.
- Escape **exactly once**, at the moment of writing to a .po file.
  NEVER re-escape a string that is already escaped. An apply step that
  parses an .po and rewrites it must read true strings and escape once on
  output; running that cycle N times over already-written output doubles
  every backslash each cycle (corruption: 2, 4, 8 … backslashes). The
  helper's `apply` is safe to re-run because it always parses true strings.
- If an agent-supplied JSON string still contains a literal 2-char
  `\n`/`\"`/`\\` sequence (escape not interpreted by the agent), `fix_esc`
  interprets it once. A genuine backslash in a UI translation is
  effectively impossible, so this is safe.

### Multiline strings

- A logical string may span multiple quoted lines; continuation lines are
  bare `"...";` the fragments concatenate in order.
- A newline **inside** a string is written as the 2-character sequence
  `\n` inside the quotes — never as an actual line break in the file.
- A string that *starts* with a newline is written as `msgid ""` (or
  `msgstr ""`) followed by continuation lines.
- In agent JSON, keys and values are true strings: real newlines (JSON
  `\n`), never the literal text backslash-n.
- Whitespace is part of the string: a trailing space before `\n` is
  significant (it has broken matches before).

### `{…}` placeholders and markup

- `{name}` tokens are runtime substitutions (`i18n_fmt`). They must appear
  in the translation **exactly once, spelled exactly, in the same
  position**. Never translate, split, reorder, or drop them.
- Strings containing placeholders carry a `#, rust-format` comment in the
  pot; chunks keep those comments, so agents can see which entries are
  live.
- `<b>`/`</b>` markup and the ellipsis character `…` (U+2026) must be
  preserved exactly where present.
- Use the correct orthography of the target language (diacritics where the
  language uses them). Characters from a **different script** are defects —
  e.g. Cyrillic е/а/о in a Latin file, CJK in a Cyrillic file. (Latin
  letters and digits are fine in most languages for technical terms:
  btrfs, SMB, xattr, `ext4`.)
- Agents write the target script's characters directly into the JSON as
  real UTF-8 (or `\u` escapes — both are valid JSON; `json.tool` handles
  both). Never hand-encode strings.

## Step 1 — Collect strings and split into chunks

1. Regenerate the pot and confirm it is current:

   ```
   bash scripts/update-pot.sh
   git diff po/fstabulator.pot
   ```

   If the pot diff contains unexpected changes, ask before continuing.

2. Create the workdir and install the helper:

   ```
   mkdir -p /tmp/opencode/translate_<lang>
   cp scripts/po_tool.py /tmp/opencode/translate_<lang>/po_tool.py
   ```

3. Seed (only if applicable): if an existing `po/<lang>.po` or an old
   translation file exists **and** the user says its entries are
   trustworthy, copy those msgid→msgstr pairs as the starting translations (they must match pot msgids exactly; unmatched ones are discarded).
   Otherwise every msgstr starts empty. Record how many were seeded.

4. Create `po/<lang>.po` with a proper header:

   ```
   msgid ""
   msgstr ""
   "Project-Id-Version: fstabulator <version from Cargo.toml>\n"
   "Report-Msgid-Bugs-To: https://github.com/lapissea/fstabulator/issues\n"
   "POT-Creation-Date: <copied from the pot>\n"
   "PO-Revision-Date: <now, local zone>\n"
   "Last-Translator: Automatically translated (machine-assisted)\n"
   "Language-Team: <Language name> <<lang>@li.org>\n"
   "Language: <lang>\n"
   "MIME-Version: 1.0\n"
   "Content-Type: text/plain; charset=UTF-8\n"
   "Content-Transfer-Encoding: 8bit\n"
   "Plural-Forms: <CLDR rule for the language>\n"
   ```

   Look up the correct CLDR `Plural-Forms` for `<lang>` — it varies a lot (e.g. hr/ru have 3 forms, ar has 6, zh has 1). The current corpus has
   no `msgid_plural` entries, but the field must be correct.

5. Split into **as few chunks as possible, each at most 50 units, all the
   same size** (sizes differ by at most 1). Chunk count is `ceil(N/50)`
   and the units are distributed evenly across it:

   ```
   python3 po_tool.py split po/<lang>.po /tmp/opencode/translate_<lang>/step2 50
   ```

   This yields equal chunks, not a ragged tail — e.g. 160 units → four
   chunks of 40 (NOT 50 50 50 10); 876 units → 18 chunks of 48–49. Note the
   printed chunk manifest. This is the unit of subagent work for step 2.

## Step 2 — Translation pass (best effort)

Goal: a complete first translation. Accuracy and natural GUI delivery are
the only criteria; length is deliberately NOT a criterion here (it is
enforced in step 3).

Dispatch subagents **in groups of 2**: launch 2 in parallel, wait for all
2, apply their results, then launch the next group of 2. Each subagent
translates exactly one chunk. Give it a single focused job — read the chunk,
translate, write the JSON — and nothing else, so it does not wander into
side work and burn its budget. Use this prompt (fill `<lang>`, `<language>`,
`NN`, count):

```
You are translating UI strings for "fstabulator", a GTK4/libadwaita GUI
application for editing /etc/fstab (mounts, filesystems, credentials).
Every string is a piece of GUI text: a label, button, dialog text, tooltip,
or error message shown to the user.

INPUT  <workdir>/step2/group_NN.po
       N PO entries, each with an English msgid and a (possibly empty)
       msgstr.

TASK   Translate every msgid into <language> (<lang>).
       - Accuracy first: the translation must say exactly what the English
         says.
       - Natural, correct <language>: right orthography (diacritics where
         the language uses them) and punctuation.
       - GUI register: concise and neutral; these are short UI texts, not
         prose. Keep terminology consistent (mount, filesystem, credential,
         server, share, backup, ...).
       - Keep EVERY {placeholder} exactly as-is, spelled exactly, in the
         same position. Keep <b></b> markup and the … character where
         present.
       - Write the target script's characters directly (real UTF-8). Never
         mix in characters from a different writing system.

OUTPUT <workdir>/step2/final_NN.json
       ONE JSON object with ALL N entries. Key = the exact msgid as a raw
       string (po escapes interpreted: \" → ", \\ → \, \n → real newline).
       Value = the translation as a raw string (real newlines).
       Validate: `python3 -m json.tool <file>` passes and the object has
       exactly N keys.

Your job is only: read INPUT, translate, write OUTPUT. Do not write helper
scripts or do any other work.
```

After each group of 2 returns, apply and check:

```
python3 po_tool.py apply po/<lang>.po <workdir>/step2/final_NN.json
msgfmt --check po/<lang>.po
```

`apply` prints `applied/changed/not-in-json`; `not-in-json` must equal the
number of units outside that chunk. If a JSON is missing entries, re-run
that one chunk's agent before moving on. Subagents never touch
`po/<lang>.po` directly — JSON in, helper out.

## Step 3 — Review pass (corrections + length reduction)

Goal: fix defects and bring length in line. The new constraint:
translations should be **roughly the same length as the English** —
accuracy still wins; only shorten when a natural shorter phrasing exists.

1. Re-split the current file (so agents see the step-2 results, with
   comments), and **precompute the lengths** so agents never have to measure
   them (a past failure mode: agents repeatedly wrote Python scripts to
   count characters, burning budget):

   ```
   python3 po_tool.py split po/<lang>.po <workdir>/step3 50 --len
   ```

   `--len` adds a `# len: en=N hr=N ratio=X.XX` comment to every entry (ratio = hr/en). Lengths are **display width**: full-width/CJK
   characters count as 2, everything else 1 — i.e. the width the GUI
   actually renders. For Latin languages this equals the character count.

2. Dispatch subagents again **in groups of 3**, one chunk per agent, with a
   single focused job — read the chunk, review, write the JSON. Use this
   prompt (fill `<lang>`, `<language>`, `NN`, count):

   ```
   You are reviewing <language> (<lang>) UI translations for "fstabulator",
   a GTK4/libadwaita GUI for editing /etc/fstab. Every string is GUI text
   (label, button, dialog, tooltip, error).

   INPUT  <workdir>/step3/group_NN.po
          N PO entries with the English msgid and the current <language>
          msgstr. Each entry has a precomputed comment line:
              # len: en=<len> hr=<len> ratio=<hr/en>
          (display widths; CJK/full-width chars count as 2)
          USE that comment for any length judgement. Do NOT write scripts or
          do arithmetic to measure lengths — the numbers are already there.

   TASK   Review every entry.
          a. CORRECTNESS: accurate meaning; correct orthography (diacritics
             where the language uses them), punctuation; consistent IT
             terminology. Fix anything wrong.
          b. LENGTH: the translation should be roughly the same length as
             the English. If ratio > 1.5 and a natural, concise phrasing
             exists that keeps full accuracy, shorten it. Accuracy always
             beats brevity — when in doubt keep the longer version.
             (For CJK languages zh/ja/ko a ratio below 1 is NORMAL — the
             translation is simply compact; never pad to match the English
             length. Only act on a CJK entry if it is markedly LONGER than
             the source, ratio > 1.5.)
          c. INTEGRITY: every {placeholder} exactly as-is, same position;
             <b></b> markup and the … character preserved.
          d. SCRIPT: replace any character from a different writing system
             (e.g. Cyrillic е/а/о in a Latin file; CJK in a Cyrillic file)
             with the correct <language> letter. Latin letters/digits are
             fine for technical terms (btrfs, SMB, xattr).

   OUTPUT <workdir>/step3/final_NN.json
          ONE JSON object with ALL N entries. Key = the exact msgid as a raw
          string (po escapes interpreted). Value = the FINAL translation:
          unchanged original if already good, else your fixed/shortened
          version (raw string, real newlines).
          Validate: `python3 -m json.tool <file>` passes and the object has
          exactly N keys.

   Your job is only: read INPUT, review, write OUTPUT. Do not write helper
   scripts or do any other work.
   ```

3. Apply each chunk's JSON with `po_tool.py apply` and run
   `msgfmt --check` after each group of 2, as in step 2.

## Step 4 — Final validation

1. Structural checks (the helper does all of these):

   ```
   msgfmt --check po/<lang>.po
   msgfmt --stat po/<lang>.po            # expect: all messages translated
   python3 po_tool.py validate po/<lang>.po po/fstabulator.pot --lang <lang>
   ```

   `validate` fails (exit 1) on: count mismatch vs pot, msgid order or
   content mismatch, empty msgstr, leftover backslashes in true strings,
   `{placeholder}` set mismatch per unit, `<b>`/`</b>` count mismatch,
   `…` count mismatch, or **foreign-script characters**. It also prints
   the length-ratio stats (display width) and the >1.5× longest entries.

   The script check is language-aware: allowed characters are ASCII (letters, digits, punctuation — this is why technical terms like
   `btrfs` never get flagged), common Unicode punctuation (…, quotes,
   dashes), Latin letters (technical terms), plus the target language's
   script. Anything else is flagged, e.g. `--lang hr` catches a Cyrillic
   е, `--lang ru` catches a CJK character, `--lang ja` catches Cyrillic.
   `--script NAME` overrides the `--lang` lookup for languages not in the
   table.

2. Inspect the reported >1.5× entries: they are acceptable when they are
   single words (the language's word is simply longer) or genuinely
   unshortenable phrasing; otherwise send them back through step 3. For
   CJK languages this list is expected to be empty (translations are
   naturally shorter) — a non-empty list there means bloat.

3. Spot-check: print 8 random units (msgid + msgstr side by side) and
   read them as a user would.

4. Confirm the build picks up the catalog:

   ```
   cargo build 2>&1 | grep -i 'msgfmt\|locale' || true
   ```

   No new warnings for `po/<lang>.po`.

5. Report: units total, seeded, step-2 changes, step-3 changes,
   validation outcome, and the length-ratio median.

## Subagent dispatch rules

- Always dispatch in **groups of 2** (2 parallel agents, then wait, then
  apply, then the next group of 2).
- One chunk per agent; never let an agent work on two chunks.
- Give each agent one focused job — read its chunk, do the task, write its
  one JSON. Keep the prompt complete and detailed, but do not ask it for
  side work (no scripts, no measuring, no research); that is how agents
  wander off-task and exhaust their budget.
- Precompute anything an agent would otherwise have to calculate (step 3
  uses `split --len`).
- Agents only read their chunk file and write their one JSON file.
- Never apply a chunk's JSON more than once, and never re-run `apply`
  against a file that an earlier `apply` already produced from the same
  JSON unless you want to confirm idempotency (it is idempotent, but the
  real danger is hand-editing an .po with an editor that re-escapes).
- When handling existing languages with existing entries, NEVER send all
  entries to workers. Only the missing ones. Also, multiple languages can
  be done in parallel in a single batch if there is only one chunk per
  language.

## Syncing existing languages (missing entries)

When the code adds or removes translatable strings, bring every existing
`po/<lang>.po` in line with the new pot:

1. Regenerate the pot (step 1.1) and confirm the diff is expected.
2. Check state: `python3 scripts/po_tool.py stats` — per file: total units,
   empty msgstrs, units in pot but not in the file, units in the file but
   not in pot, order ok/wrong (ok = msgid sequence exactly matches the pot).
   Exit 1 while any file is out of sync.
3. See what needs translating: `python3 scripts/po_tool.py missing` — lists
   the missing and empty msgids per file.
4. `python3 scripts/po_tool.py reorder` — rewrites every po in pot order:
   inserts pot units absent from the file (empty msgstr, pot comments) and
   drops (printing each) file units that left the pot. It is the destructive
   step — run it only after you have reviewed the pot diff.
5. Translate **only the missing entries** (step 2, one chunk per language;
   see the dispatch rules for batching languages) and review them (step 3).
   Never send already-translated entries to workers.
6. Final validation (step 4) for every language. `stats` must come back
   clean and exit 0.

## Helper script

The helper is `scripts/po_tool.py` in the repository (copy it into the workdir
in step 1). Subcommands: `split`, `apply`, `validate`, `stats`, `missing`,
`reorder` — run it with no arguments for usage; its docstring documents each
one.
