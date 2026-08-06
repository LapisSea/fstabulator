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
	pub fn is_changed(&self) -> bool {
		self.device != self.original.device
			|| self.mount_point != self.original.mount_point
			|| self.fs_type != self.original.fs_type
			|| self.options != self.original.options
			|| self.dump != self.original.dump
			|| self.pass != self.original.pass
	}

	pub fn from(line: usize, raw: &str) -> Result<Self> {
		let (content, comment) = match raw.split_once('#') {
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
		write!(
			f,
			"{} {} {} {} {} {}",
			self.device,
			self.mount_point,
			self.fs_type.to_string(),
			self.options.join(","),
			self.dump,
			self.pass
		)?;
		Ok(())
	}
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

