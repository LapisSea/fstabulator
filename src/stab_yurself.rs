use crate::device_value::{DeviceKind, DeviceValue};
use crate::fs_value::FsType;
use anyhow::{Context, Error, Result, bail};
use std::fmt;
use std::iter::Iterator;
use std::str::FromStr;

pub struct StabEntry {
	pub line: usize,
	pub device: DeviceValue,
	pub mount_point: String,
	pub fs_type: FsType,
	pub options: Vec<String>,
	pub dump: u8,
	pub pass: u8,
	pub original: String,
	pub user_label: Option<String>,
}

impl StabEntry {
	pub fn blank(line: usize) -> Self {
		StabEntry {
			line,
			device: DeviceValue::from("", DeviceKind::Other),
			mount_point: String::new(),
			fs_type: FsType::Other(String::new()),
			options: vec!["defaults".to_string()],
			dump: 0,
			pass: 0,
			original: String::new(),
			user_label: None,
		}
	}

	pub fn original_normalized(&self) -> String {
		normalize_whitespace(&self.original)
	}

	pub fn reset(&mut self) {
		let Ok(original) = Self::from(self.line, &self.original) else {
			if self.original.is_empty() {
				*self = Self::blank(self.line);
			} else {
				eprintln!("BUG: Could not parse original entry: {}", self.original);
			}
			return;
		};
		self.device = original.device.clone();
		self.mount_point = original.mount_point.clone();
		self.fs_type = original.fs_type.clone();
		self.options = original.options.clone();
		self.dump = original.dump;
		self.pass = original.pass;
	}

	pub fn has_option(&self, name: &str) -> bool {
		self.options.iter().any(|o| o.split('=').next() == Some(name))
	}

	pub fn is_valid(&self) -> bool {
		Self::from(self.line, &self.data_to_string()).is_ok()
	}

	pub fn is_changed(&self) -> bool {
		let original = match Self::from(self.line, &self.original) {
			Ok(original) => original,
			Err(_) if self.original.is_empty() => Self::blank(self.line),
			Err(_) => {
				eprintln!("BUG: Could not parse original entry: {}", self.original);
				return true;
			}
		};
		self.device != original.device
			|| self.mount_point != original.mount_point
			|| self.fs_type != original.fs_type
			|| self.options != original.options
			|| self.dump != original.dump
			|| self.pass != original.pass
	}

	pub fn from(line: usize, raw: &str) -> Result<Self> {
		let (content, _comment) = match raw.split_once('#') {
			Some((before, after)) => (before, Some(normalize_whitespace(after))),
			None => (raw, None),
		};

		let fields: Vec<&str> = content.split_whitespace().collect();

		if fields.len() < 6 {
			bail!(
				"line {}: expected 6 fields (device, mount_point, fs_type, options, dump, pass), got {}",
				line,
				fields.len()
			);
		}

		let device = fields[0].to_string();
		let mount_point = fields[1].to_string();
		let fs_type = FsType::from_str(fields[2]).context(format!("Cannot parse fs_type: {}", fields[2]))?;
		let options: Vec<String> = fields[3].split(',').map(|opt| opt.to_string()).collect();

		let dump = fields[4]
			.parse::<u8>()
			.with_context(|| format!("line {line}: dump field is not a valid integer"))?;
		let pass = fields[5]
			.parse::<u8>()
			.with_context(|| format!("line {line}: pass field is not a valid integer"))?;

		let device = DeviceKind::classify(&device, DeviceKind::for_fs_type(&fs_type));

		let entry = StabEntry {
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
		let expected = normalize_whitespace(raw);

		if produced != expected {
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
		format!(
			"{} {} {} {} {} {}",
			self.device.render(),
			&self.mount_point,
			&self.fs_type,
			&self.options.join(","),
			self.dump,
			self.pass
		)
	}
}

impl fmt::Display for StabEntry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", &self.data_to_string())
	}
}

pub enum StabLine {
	Blank,
	Comment(String),
	Entry(StabEntry),
	Unparsable(Error, String),
}

fn normalize_whitespace(s: &str) -> String {
	s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_fstab(raw: &str) -> Vec<StabLine> {
	let mut entries = raw
		.lines()
		.enumerate()
		.map(|(line_num, line)| {
			if line.trim().is_empty() {
				return StabLine::Blank;
			}
			if line.starts_with('#') {
				return StabLine::Comment(line.to_string());
			}

			StabEntry::from(line_num, line)
				.map(StabLine::Entry)
				.unwrap_or_else(|e| StabLine::Unparsable(e, line.to_string()))
		})
		.collect::<Vec<_>>();

	merge_comments_into_labels(&mut entries);

	entries
}

pub fn read_fstab() -> Result<Vec<StabLine>> {
	// let path = "./fstab-dummy";
	let path = "/etc/fstab";

	let raw = std::fs::read_to_string(path).context("Could not read /etc/fstab")?;

	Ok(parse_fstab(&raw))
}

fn comment_content(comment: &str) -> Option<&str> {
	comment.strip_prefix('#').map(str::trim).filter(|s| !s.is_empty())
}

fn comment_looks_like_entry(comment: &str) -> bool {
	comment_content(comment).is_some_and(|content| StabEntry::from(0, content).is_ok())
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
			if comment_looks_like_entry(comment) {
				merged.push(line);
				continue;
			}
			match iter.next() {
				Some(StabLine::Entry(mut entry)) => {
					entry.user_label = comment_content(comment).map(str::to_string);
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
	fn all_dummy_entries_parse_as_stab_entry() {
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
		assert_eq!(entries.len(), 44, "unexpected number of entries in fstab-dummy");
	}

	#[test]
	fn comment_before_entry_becomes_user_label() {
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
		assert_eq!(first.user_label.as_deref(), Some("First entry note"));

		let StabLine::Comment(comment) = &lines[1] else {
			panic!("expected commented-out entry to remain a comment");
		};
		assert_eq!(comment, "# /dev/sda1 /mnt ext4 defaults 0 2");

		let StabLine::Entry(second) = &lines[2] else {
			panic!("expected third line to be an entry");
		};
		assert_eq!(second.user_label, None);
	}

	#[test]
	fn comment_not_followed_by_entry_stays_comment() {
		let raw = "# stray note\n";
		let lines = parse_fstab(raw);
		let StabLine::Comment(comment) = &lines[0] else {
			panic!("expected comment to remain");
		};
		assert_eq!(comment, "# stray note");
	}

	#[test]
	fn blank_entry_is_invalid_and_parsed_entry_is_valid() {
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
}
