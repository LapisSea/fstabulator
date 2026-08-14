use crate::device_value::resolve_local_device;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subvol {
	pub id: u64,
	pub path: String,
}

fn find_mount_point(device: &str) -> Result<Option<String>> {
	let sources = std::iter::once(device.to_string()).chain(resolve_local_device(device));
	for source in sources {
		let output = Command::new("findmnt")
			.args(["-n", "-o", "TARGET", "-S", &source])
			.output()
			.with_context(|| format!("Could not run 'findmnt' to resolve '{source}'. Is util-linux installed?"))?;
		if !output.status.success() {
			continue;
		}
		let target = String::from_utf8_lossy(&output.stdout);
		let first = target.lines().next().map(str::trim).filter(|s| !s.is_empty());
		if let Some(mount_point) = first {
			return Ok(Some(mount_point.to_string()));
		}
	}
	Ok(None)
}

pub fn list_subvolumes(device: &Path) -> Result<Vec<Subvol>> {
	let Some(mount_point) = find_mount_point(device.to_string_lossy().as_ref())? else {
		bail!(
			"The device '{}' is not currently mounted, so its subvolumes cannot be listed.",
			device.display()
		);
	};

	let direct = Command::new("btrfs")
		.args(["subvolume", "list", &mount_point])
		.output()
		.context("Could not run the 'btrfs' command. Is btrfs-progs installed?")?;
	if direct.status.success() {
		return Ok(parse_subvolumes(&String::from_utf8_lossy(&direct.stdout)));
	}

	let stderr = String::from_utf8_lossy(&direct.stderr);
	if !is_permission_error(&stderr) {
		bail!("btrfs could not list subvolumes on '{mount_point}': {}", stderr.trim());
	}

	crate::privileged_actions::list_subvolumes(&mount_point)
}

fn is_permission_error(stderr: &str) -> bool {
	let lower = stderr.to_ascii_lowercase();
	lower.contains("operation not permitted") || lower.contains("permission denied") || lower.contains("not permitted")
}

pub(crate) fn parse_subvolumes(stdout: &str) -> Vec<Subvol> {
	let mut subvols = Vec::new();
	for line in stdout.lines() {
		let fields: Vec<&str> = line.split_whitespace().collect();
		let Some(id_idx) = fields.iter().position(|f| *f == "ID") else {
			continue;
		};
		let Some(id) = fields.get(id_idx + 1).and_then(|f| f.parse::<u64>().ok()) else {
			continue;
		};
		let Some(path_idx) = fields.iter().position(|f| *f == "path") else {
			continue;
		};
		let path = fields[path_idx + 1..].join(" ");
		if path.is_empty() {
			continue;
		}
		subvols.push(Subvol { id, path });
	}
	subvols.sort_by(|a, b| a.path.cmp(&b.path));
	subvols
}
