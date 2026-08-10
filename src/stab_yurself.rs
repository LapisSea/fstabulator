use crate::fs_value::FsType;
use anyhow::{Context, Result, bail};
use std::fmt;
use std::iter::Iterator;
use std::str::FromStr;

#[derive(Clone, PartialEq, Eq)]
pub struct StabEntrySnapshot {
	pub device: String,
	pub mount_point: String,
	pub fs_type: FsType,
	pub options: Vec<String>,
	pub dump: u8,
	pub pass: u8,
}
impl fmt::Display for StabEntrySnapshot {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write_entry(f, &self.device, &self.mount_point, &self.fs_type, &self.options, self.dump, self.pass)
	}
}

pub struct StabEntry {
	pub line: usize,
	pub device: String,
	pub mount_point: String,
	pub fs_type: FsType,
	pub options: Vec<String>,
	pub dump: u8,
	pub pass: u8,
	original: StabEntrySnapshot,
}

impl StabEntry {
	pub fn original(&self) -> &StabEntrySnapshot {
		&self.original
	}

	pub fn reset(&mut self) {
		self.device = self.original.device.clone();
		self.mount_point = self.original.mount_point.clone();
		self.fs_type = self.original.fs_type.clone();
		self.options = self.original.options.clone();
		self.dump = self.original.dump;
		self.pass = self.original.pass;
	}

	pub fn has_option(&self, name: &str) -> bool {
		self.options.iter().any(|o| o.split('=').next() == Some(name))
	}

	pub fn is_changed(&self) -> bool {
		self.device != self.original.device
			|| self.mount_point != self.original.mount_point
			|| self.fs_type != self.original.fs_type
			|| self.options != self.original.options
			|| self.dump != self.original.dump
			|| self.pass != self.original.pass
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

		let entry = StabEntry {
			line,
			device: device.clone(),
			mount_point: mount_point.clone(),
			fs_type: fs_type.clone(),
			options: options.clone(),
			dump,
			pass,
			original: StabEntrySnapshot {
				device,
				mount_point,
				fs_type,
				options,
				dump,
				pass,
			},
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
}

impl fmt::Display for StabEntry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write_entry(f, &self.device, &self.mount_point, &self.fs_type, &self.options, self.dump, self.pass)
	}
}

fn write_entry(f: &mut fmt::Formatter<'_>, device: &str, mount_point: &str, fs_type: &FsType, options: &[String], dump: u8, pass: u8) -> fmt::Result {
	write!(f, "{} {} {} {} {} {}", device, mount_point, fs_type, options.join(","), dump, pass)
}

fn normalize_whitespace(s: &str) -> String {
	s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn read_fstab() -> Result<Vec<Result<StabEntry>>> {
	let path = "./fstab-dummy";
	// let path="/etc/fstab";

	let raw = std::fs::read_to_string(path).context("Could not read /etc/fstab")?;

	let entries = raw
		.lines()
		.enumerate()
		.filter(|(_, line)| !line.trim().is_empty() && !line.starts_with("#"))
		.map(|(line, str)| StabEntry::from(line, str))
		.collect::<Vec<_>>();

	Ok(entries)
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
}
