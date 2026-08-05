use anyhow::{Context, Result, bail};
use std::fmt;

#[derive(Clone, PartialEq, Eq)]
pub struct StabEntrySnapshot {
	pub device: String,
	pub mount_point: String,
	pub fs_type: String,
	pub options: Vec<String>,
	pub dump: u8,
	pub pass: u8,
}

pub struct StabEntry {
	pub line: usize,
	pub device: String,
	pub mount_point: String,
	pub fs_type: String,
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
		let fs_type = fields[2].to_string();
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
			self.fs_type,
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

pub enum FsType {
	// Ext4(Ext4Options),
	Btrfs(BtrfsOptions),
	// Xfs(XfsOptions),
	// Vfat(VfatOptions),
	Ntfs3(Ntfs3Options),
	Cifs(CifsOptions), // Samba
	// Nfs(NfsOptions),
	Swap,
	Unknown { fs_type: String, extra_options: Vec<String> },
}

pub struct CifsOptions {
	pub credentials_file: Option<String>,
	pub username: Option<String>,
	pub uid: Option<u32>,
	pub gid: Option<u32>,
	pub file_mode: Option<String>, // e.g. "0755"
	pub dir_mode: Option<String>,
	pub vers: Option<String>,       // SMB protocol version
	pub extra_options: Vec<String>, // anything unrecognized, preserved verbatim
}

impl Default for CifsOptions {
	fn default() -> Self {
		CifsOptions {
			credentials_file: None,
			username: None,
			uid: None,
			gid: None,
			file_mode: Some("0755".into()),
			dir_mode: Some("0755".into()),
			vers: Some("3.0".into()),
			extra_options: Vec::new(),
		}
	}
}

pub struct Ntfs3Options {
	pub uid: Option<u32>,
	pub gid: Option<u32>,
	pub umask: Option<String>,
	pub windows_names: bool,
	pub extra_options: Vec<String>,
}

pub struct BtrfsOptions {
	pub subvol: Option<String>,
	pub compress: Option<String>, // "zstd", "lzo", "none"
	pub extra_options: Vec<String>,
}
