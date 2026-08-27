use crate::GC;
use crate::device_value::{DeviceKind, DeviceValue};
use crate::fs_options;
use crate::fs_value::FsType;
use anyhow::{Context, Result, bail};
use fs_options::FsOption;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Clone)]
pub struct StabEntry {
	pub active: bool,
	pub line: usize,
	pub device: DeviceValue,
	pub mount_point: String,
	pub fs_type: FsType,
	pub options: Vec<FsOption>,
	pub dump: u8,
	pub pass: u8,
	pub original: String,
	pub user_label: Option<String>,
}

impl StabEntry {
	pub fn blank(line: usize) -> Self {
		StabEntry {
			active: true,
			line,
			device: DeviceValue::from("", DeviceKind::Other),
			mount_point: String::new(),
			fs_type: FsType::Other(String::new()),
			options: vec![FsOption::Named("defaults".to_string())],
			dump: 0,
			pass: 0,
			original: String::new(),
			user_label: None,
		}
	}

	pub fn original_normalized(&self) -> String {
		normalize_entry_text(&self.original)
	}

	fn original_or_blank(&self) -> Option<StabEntry> {
		match Self::from(self.line, &self.original) {
			Ok(original) => Some(original),
			Err(_) if self.original.is_empty() => Some(Self::blank(self.line)),
			Err(_) => {
				eprintln!("BUG: Could not parse original entry: {}", self.original);
				None
			}
		}
	}

	pub fn reset(&mut self) {
		let Some(original) = self.original_or_blank() else {
			return;
		};
		self.active = original.active;
		self.device = original.device.clone();
		self.mount_point = original.mount_point.clone();
		self.fs_type = original.fs_type.clone();
		self.options = original.options.clone();
		self.dump = original.dump;
		self.pass = original.pass;
	}

	pub fn has_option(&self, name: &str) -> bool {
		self.options.iter().any(|o| o.name() == name)
	}

	pub fn is_valid(&self) -> bool {
		Self::from(self.line, &self.data_to_string()).is_ok()
	}

	pub fn is_changed(&self) -> bool {
		let Some(original) = self.original_or_blank() else {
			return true;
		};
		self.active != original.active
			|| self.device != original.device
			|| self.mount_point != original.mount_point
			|| self.fs_type != original.fs_type
			|| self.options != original.options
			|| self.dump != original.dump
			|| self.pass != original.pass
	}

	pub fn mount_point_changed(&self) -> bool {
		let Some(original) = self.original_or_blank() else {
			return true;
		};
		original.mount_point != self.mount_point
	}

	pub fn from(line: usize, raw: &str) -> Result<Self> {
		let (fields, active) = split(raw);

		let fields: Vec<String> = fields.iter().map(|field| unescape_field(field)).collect();

		if fields.len() != 6 {
			bail!(
				"line {}: expected 6 fields (device, mount_point, fs_type, options, dump, pass), got {}",
				line,
				fields.len()
			);
		}

		let device = fields[0].clone();
		let mount_point = fields[1].clone();
		let fs_type = FsType::from_str(&fields[2]).context(format!("Cannot parse fs_type: {}", fields[2]))?;
		let options: Vec<FsOption> = fields[3].split(',').map(FsOption::from_raw).collect();

		let dump = fields[4]
			.parse::<u8>()
			.with_context(|| format!("line {line}: dump field is not a valid integer"))?;
		let pass = fields[5]
			.parse::<u8>()
			.with_context(|| format!("line {line}: pass field is not a valid integer"))?;

		let device = DeviceKind::classify(&device, DeviceKind::for_fs_type(&fs_type));

		let entry = StabEntry {
			active,
			line,
			device,
			mount_point: mount_point.clone(),
			fs_type: fs_type.clone(),
			options: options.clone(),
			dump,
			pass,
			original: raw.to_string(),
			user_label: None,
		};

		let produced = entry.to_string();
		let expected = normalize_entry_text(raw);

		if normalize_entry_text(&produced) != expected {
			bail!(
				"line {}: entry did not round-trip cleanly\n  expected: {:?}\n  produced: {:?}",
				line,
				expected,
				produced
			);
		}

		Ok(entry)
	}
	fn data_to_string(&self) -> String {
		let active_str = if self.active { "" } else { "# " };
		let options = self.options.iter().map(|o| o.to_string()).collect::<Vec<_>>().join(",");
		format!(
			"{}{} {} {} {} {} {}",
			active_str,
			escape_field(&self.device.render()),
			escape_field(&self.mount_point),
			escape_field(&self.fs_type.to_string()),
			escape_field(&options),
			self.dump,
			self.pass
		)
	}
}

impl fmt::Display for StabEntry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.data_to_string())
	}
}

pub enum StabLine {
	Blank,
	Comment(String),
	Entry(GC<StabEntry>),
	Unparsable(String),
}

fn normalize_entry_text(s: &str) -> String {
	let (fields, active) = split(s);
	let fields = fields.iter().map(|field| unescape_field(field)).collect::<Vec<_>>().join(" ");
	if !active { format!("# {fields}") } else { fields }
}

fn escape_field(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	for c in input.chars() {
		match c {
			' ' => out.push_str("\\040"),
			'\t' => out.push_str("\\011"),
			'\n' => out.push_str("\\012"),
			'\\' => out.push_str("\\134"),
			// unescape_field reads exactly three octal digits (fstab convention),
			// so whitespace above 0o777 cannot be encoded; left verbatim it makes
			// the entry fail to re-parse instead of silently corrupting fields.
			_ if c.is_whitespace() && c <= '\u{01FF}' => out.push_str(&format!("\\{:03o}", c as u32)),
			_ => out.push(c),
		}
	}
	out
}

pub(crate) fn unescape_field(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	let mut chars = input.chars();
	while let Some(c) = chars.next() {
		if c != '\\' {
			out.push(c);
			continue;
		}
		let mut code = String::new();
		let mut interrupted = None;
		for _ in 0..3 {
			match chars.next() {
				Some(d @ '0'..='7') => code.push(d),
				Some(other) => {
					interrupted = Some(other);
					break;
				}
				None => break,
			}
		}
		if code.len() == 3 {
			let decoded = u32::from_str_radix(&code, 8).ok().and_then(char::from_u32).unwrap_or('\u{FFFD}');
			out.push(decoded);
		} else {
			out.push('\\');
			out.push_str(&code);
		}
		if let Some(other) = interrupted {
			out.push(other);
		}
	}
	out
}

fn parse_fstab(raw: &str) -> Vec<StabLine> {
	let mut entries = raw
		.lines()
		.enumerate()
		.map(|(line_num, line)| {
			if line.trim().is_empty() {
				return StabLine::Blank;
			}
			match StabEntry::from(line_num, line) {
				Ok(e) => StabLine::Entry(GC::new(e)),
				Err(_) if line.starts_with('#') => StabLine::Comment(line.to_string()),
				Err(_) => StabLine::Unparsable(line.to_string()),
			}
		})
		.collect::<Vec<_>>();

	merge_comments_into_labels(&mut entries);

	entries
}

pub struct StabFile {
	path: PathBuf,
	pub lines: Vec<StabLine>,
	pub(crate) reference: String,
}

impl StabFile {
	pub fn read<P: Into<PathBuf>>(path: P) -> Result<Self> {
		let path = path.into();
		let raw = std::fs::read_to_string(&path).with_context(|| format!("Could not read {}", path.display()))?;
		let mut file = Self::from_raw(&raw);
		file.path = path;
		Ok(file)
	}
	pub fn from_raw(raw: &str) -> Self {
		let lines = parse_fstab(raw);
		let mut file = Self {
			path: PathBuf::new(),
			lines,
			reference: String::new(),
		};
		file.reference = file.to_string();
		file
	}
	pub fn empty() -> Self {
		Self {
			path: PathBuf::new(),
			lines: Vec::new(),
			reference: String::new(),
		}
	}
	pub fn is_changed(&self) -> bool {
		self.to_string() != self.reference
	}
	pub fn entries(&self) -> impl Iterator<Item = &GC<StabEntry>> {
		self.lines.iter().filter_map(|e| match e {
			StabLine::Entry(e) => Some(e),
			_ => None,
		})
	}
	pub fn entry_at(&self, index: usize) -> Option<&GC<StabEntry>> {
		self.entries().nth(index)
	}

	pub fn remove_entry(&mut self, index: usize) -> Option<GC<StabEntry>> {
		let pos = self
			.lines
			.iter()
			.enumerate()
			.filter(|(_, l)| matches!(l, StabLine::Entry(_)))
			.nth(index)?
			.0;
		let Some(StabLine::Entry(entry)) = self.lines.get(pos) else {
			return None;
		};
		let entry = entry.clone();
		self.lines.remove(pos);
		Some(entry)
	}

	pub fn push_entry(&mut self, entry: StabEntry) {
		self.lines.push(StabLine::Entry(GC::new(entry)));
	}

	/// Combine this file's entries with backup, keeps note of what changed between 2 files
	pub fn overlay_backup(&self, backup: &StabFile) -> Vec<StabLine> {
		let baseline: Vec<StabEntry> = self.entries().map(|entry| entry.borrow().clone()).collect();
		backup
			.lines
			.iter()
			.map(|line| match line {
				StabLine::Entry(backup_entry) => {
					let backup_entry = backup_entry.borrow();
					let mut restored = backup_entry.clone();
					restored.original = match_baseline(&restored, &baseline).map(|entry| entry.original).unwrap_or_default();
					StabLine::Entry(GC::new(restored))
				}
				StabLine::Blank => StabLine::Blank,
				StabLine::Comment(comment) => StabLine::Comment(comment.clone()),
				StabLine::Unparsable(raw) => StabLine::Unparsable(raw.clone()),
			})
			.collect()
	}
}

impl fmt::Display for StabFile {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let rendered = self
			.lines
			.iter()
			.map(|e| match e {
				StabLine::Entry(e) => {
					let e = e.borrow();
					let body = if e.is_changed() { e.data_to_string() } else { e.original.clone() };
					match &e.user_label {
						Some(label) => format!("# {}\n{}", label, body),
						None => body,
					}
				}
				StabLine::Blank => String::new(),
				StabLine::Comment(val) => val.clone(),
				StabLine::Unparsable(val) => val.clone(),
			})
			.collect::<Vec<_>>()
			.join("\n");
		f.write_str(&rendered)
	}
}

pub fn scan_for_backups() -> Result<Vec<(PathBuf, SystemTime)>> {
	let folder = "/etc/";
	let file_base = "fstab.bak_";

	let res: Vec<_> = std::fs::read_dir(folder)?
		.filter_map(|entry| entry.ok())
		.filter_map(|entry| {
			let name = entry.file_name().to_string_lossy().to_string();
			let time_str = name.strip_prefix(file_base)?;
			let time = parse_time(time_str)?;
			Some((entry.path(), time))
		})
		.collect();
	Ok(res)
}

fn parse_time(time: &str) -> Option<SystemTime> {
	let parts: Vec<&str> = time.split('T').collect();
	let standard_rfc3339 = format!("{}T{}", parts.first()?, parts.get(1)?.replace('-', ":"));
	humantime::parse_rfc3339(&standard_rfc3339).ok()
}

fn split(raw: &str) -> (Vec<&str>, bool) {
	let mut fields: Vec<&str> = raw.split_whitespace().collect();

	let active = if let Some(rest) = fields.first().and_then(|e| e.strip_prefix("#")) {
		if rest.trim().is_empty() {
			fields.remove(0);
		} else {
			fields[0] = rest.trim();
		}
		false
	} else {
		true
	};
	(fields, active)
}

fn comment_content(comment: &str) -> Option<&str> {
	comment.strip_prefix('#').map(str::trim).filter(|s| !s.is_empty())
}

fn match_baseline(entry: &StabEntry, baseline: &[StabEntry]) -> Option<StabEntry> {
	let mut candidates = baseline.iter().filter(|candidate| candidate.mount_point == entry.mount_point);
	let first = candidates.next();
	let preferred = candidates.find(|candidate| candidate.device == entry.device);
	preferred.or(first).cloned()
}

fn merge_comments_into_labels(entries: &mut Vec<StabLine>) {
	let mut merged = Vec::with_capacity(entries.len());
	{
		let mut iter = entries.drain(..).peekable();
		while let Some(line) = iter.next() {
			let StabLine::Comment(comment) = &line else {
				merged.push(line);
				continue;
			};
			match iter.next() {
				Some(StabLine::Entry(entry)) => {
					{
						let mut entry = entry.borrow_mut();
						entry.user_label = comment_content(comment).map(str::to_string);
					}
					merged.push(StabLine::Entry(entry));
				}
				other => {
					merged.push(line);
					if let Some(other) = other {
						merged.push(other);
					}
				}
			}
		}
	}
	*entries = merged;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn escape_field_encodes_specials() {
		assert_eq!(escape_field("a b"), "a\\040b");
		assert_eq!(escape_field("a\tb"), "a\\011b");
		assert_eq!(escape_field("a\nb"), "a\\012b");
		assert_eq!(escape_field("a\\b"), "a\\134b");
		assert_eq!(escape_field("plain/path"), "plain/path");

		assert_eq!(escape_field("a\rb"), "a\\015b");
		assert_eq!(escape_field("a\u{000b}b"), "a\\013b");
		assert_eq!(escape_field("a\u{000c}b"), "a\\014b");
		assert_eq!(escape_field("a\u{0085}b"), "a\\205b");
		assert_eq!(escape_field("a\u{00a0}b"), "a\\240b");
		assert!(!escape_field("a b\tc\nd\r\u{00a0}e").chars().any(char::is_whitespace));

		assert_eq!(unescape_field("a\\040b"), "a b");
		assert_eq!(unescape_field("a\\011b"), "a\tb");
		assert_eq!(unescape_field("a\\012b"), "a\nb");
		assert_eq!(unescape_field("a\\134b"), "a\\b");
		assert_eq!(unescape_field("/plain"), "/plain");

		assert_eq!(unescape_field("a\\04"), "a\\04");
		assert_eq!(unescape_field("a\\1x"), "a\\1x");
		assert_eq!(unescape_field("trailing\\"), "trailing\\");
		assert_eq!(unescape_field("\\040"), " ");

		assert_eq!(escape_field("a\u{1680}b"), "a\u{1680}b");
	}

	#[test]
	fn escape_unescape_round_trip() {
		for value in [
			"/mnt/my docs",
			"tab\tinside",
			"new\nline",
			"back\\slash",
			"\\leading",
			"trailing ",
			"",
			"a\\040b",
			" ",
			"cr\rmid",
			"nbsp\u{00a0}mid",
			"vt\u{000b}ff\u{000c}",
		] {
			assert_eq!(unescape_field(&escape_field(value)), value);
		}
	}

	#[test]
	fn escaped_fields_parse_and_round_trip() {
		let raw = "LABEL=arch\\040boot /mnt/my\\040docs ext4 rw,x-systemd.automount 0 2";
		let entry = StabEntry::from(0, raw).unwrap();
		assert_eq!(entry.device.value, "arch boot");
		assert_eq!(entry.mount_point, "/mnt/my docs");
		assert!(entry.is_valid());
		assert!(!entry.is_changed());
		assert_eq!(entry.to_string(), raw);
	}

	#[test]
	fn non_canonical_escapes_keep_original_bytes() {
		// \101 decodes to 'A'; not one of the four canonical escapes, but must
		// still parse and be written back byte-identically while untouched.
		let raw = "UUID=1 /mnt/a\\101b ext4 defaults 0 2";
		let mut entry = StabEntry::from(0, raw).unwrap();
		assert_eq!(entry.mount_point, "/mnt/aAb");
		assert!(!entry.is_changed());

		entry.mount_point = "/mnt/edited".to_string();
		assert_eq!(
			entry.to_string(),
			"UUID=1 /mnt/edited ext4 defaults 0 2",
			"editing should rewrite the line canonically"
		);
	}

	#[test]
	fn written_fields_are_escaped() {
		let mut entry = StabEntry::blank(0);
		entry.device = DeviceValue::from("my label", DeviceKind::Label);
		entry.fs_type = FsType::Ext4;
		entry.mount_point = "/mnt/a b".to_string();

		let rendered = entry.to_string();
		assert_eq!(rendered, "LABEL=my\\040label /mnt/a\\040b ext4 defaults 0 0");

		let reparsed = StabEntry::from(0, &rendered).unwrap();
		assert_eq!(reparsed.device.value, "my label");
		assert_eq!(reparsed.mount_point, "/mnt/a b");
	}

	#[test]
	fn dummy_entries_parse() {
		let raw = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/fstab-dummy")).expect("could not read fstab-dummy");
		let entries: Vec<(usize, Result<StabEntry>)> = raw
			.lines()
			.enumerate()
			.filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'))
			.map(|(line, text)| (line, StabEntry::from(line, text)))
			.collect();

		assert!(!entries.is_empty(), "fstab-dummy contains no entries");
		for (line, result) in &entries {
			if let Err(err) = result {
				panic!("line {} failed to parse: {err:#}", line + 1);
			}
		}
		assert_eq!(entries.len(), 46, "unexpected number of entries in fstab-dummy");
	}

	#[test]
	fn dummy_round_trip() {
		let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fstab-dummy");
		let original = std::fs::read_to_string(path).expect("could not read fstab-dummy");
		let file = StabFile::read(path).expect("could not parse fstab-dummy");
		assert_eq!(file.to_string(), original);
	}

	#[test]
	fn comment_becomes_label() {
		let raw = "\
# First entry note
UUID=550e8400-e29b-41d4-a716-446655440000 / ext4 rw 0 1
# /dev/sda1 /mnt ext4 defaults 0 2
UUID=11111111-1111-1111-1111-111111111111 /home xfs rw 0 2
";
		let lines = parse_fstab(raw);

		let StabLine::Entry(first) = &lines[0] else {
			panic!("expected first line to be an entry");
		};
		let first = first.borrow();
		assert_eq!(first.user_label.as_deref(), Some("First entry note"));
		assert!(first.active);

		let StabLine::Entry(disabled) = &lines[1] else {
			panic!("expected commented-out entry to become a disabled entry");
		};
		let disabled = disabled.borrow();
		assert!(!disabled.active);

		let StabLine::Entry(second) = &lines[2] else {
			panic!("expected third line to be an entry");
		};
		let second = second.borrow();
		assert_eq!(second.user_label, None);
		assert!(second.active);
	}

	#[test]
	fn stray_comment_kept() {
		let raw = "# stray note\n";
		let lines = parse_fstab(raw);
		let StabLine::Comment(comment) = &lines[0] else {
			panic!("expected comment to remain");
		};
		assert_eq!(comment, "# stray note");
	}

	#[test]
	fn entry_validity() {
		let parsed = StabEntry::from(0, "UUID=1 / ext4 defaults 0 1").unwrap();
		assert!(parsed.is_valid());
		assert!(!parsed.is_changed());

		let mut blank = StabEntry::blank(99);
		assert!(!blank.is_valid());
		assert!(!blank.is_changed());
		blank.device = DeviceValue::from("2", DeviceKind::Uuid);
		assert!(blank.is_changed());
		blank.reset();
		assert_eq!(blank.device.value, "");
		assert!(!blank.is_changed());
	}

	#[test]
	fn mount_point_changed() {
		let mut entry = StabEntry::from(0, "UUID=1 /mnt/data ext4 defaults 0 2").unwrap();
		assert!(!entry.mount_point_changed());
		entry.mount_point = "/mnt/other".to_string();
		assert!(entry.mount_point_changed());
		assert!(entry.is_changed());

		let mut blank = StabEntry::blank(0);
		assert!(!blank.mount_point_changed());
		blank.mount_point = "/mnt/data".to_string();
		assert!(blank.mount_point_changed());
	}

	#[test]
	fn overlay_backup_matches_baseline() {
		let real = "\
UUID=1 / ext4 defaults 0 1
UUID=2 /home xfs defaults 0 2
";
		let backup = "\
UUID=1 / ext4 defaults 0 1
UUID=9 /home xfs defaults 0 2
UUID=3 /mnt/tmp ext4 defaults 0 2
";
		let baseline = StabFile::from_raw(real);
		let backup_file = StabFile::from_raw(backup);
		let restored = baseline.overlay_backup(&backup_file);

		let entries: Vec<StabEntry> = restored
			.iter()
			.filter_map(|line| match line {
				StabLine::Entry(entry) => Some(entry.borrow().clone()),
				_ => None,
			})
			.collect();

		assert_eq!(entries.len(), 3);
		assert!(!entries[0].is_changed(), "identical entry should stay unmodified");
		assert!(entries[1].is_changed(), "changed device should be reported as modified");
		assert_eq!(entries[1].original, "UUID=2 /home xfs defaults 0 2");
		assert_eq!(entries[1].device.value, "9");
		assert!(entries[2].is_changed(), "entry absent from real fstab should be reported as modified");
		assert_eq!(entries[2].original, "");
	}

	#[test]
	fn file_is_changed() {
		let raw = "\
UUID=1 / ext4 defaults 0 1
UUID=2 /home xfs defaults 0 2
";
		let mut file = StabFile::from_raw(raw);
		assert!(!file.is_changed(), "freshly parsed file should be unmodified");

		file.remove_entry(0);
		assert!(file.is_changed(), "removed entry should mark the file as changed");

		let mut file = StabFile::from_raw(raw);
		file.push_entry(StabEntry::from(2, "UUID=3 /mnt ext4 defaults 0 2").unwrap());
		assert!(file.is_changed(), "added entry should mark the file as changed");

		let file = StabFile::from_raw(raw);
		let entry = file.entry_at(0).unwrap();
		entry.borrow_mut().mount_point = "/other".to_string();
		assert!(file.is_changed(), "modified entry should mark the file as changed");
	}
}
