use crate::device_value::{DeviceKind, DeviceValue};
use crate::fs_value::FsType;
use crate::stab_yurself::{StabEntry, unescape_field};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MountStatus {
	Mounted,
	Unmounted,
	Missing,
}

impl MountStatus {
	pub fn label(self) -> &'static str {
		match self {
			MountStatus::Mounted => "Mounted",
			MountStatus::Unmounted => "Unmounted",
			MountStatus::Missing => "Mount point missing",
		}
	}

	pub fn css_class(self) -> &'static str {
		match self {
			MountStatus::Mounted => "mount-status-mounted",
			MountStatus::Unmounted => "mount-status-unmounted",
			MountStatus::Missing => "mount-status-missing",
		}
	}

	pub fn tooltip(self) -> &'static str {
		match self {
			MountStatus::Mounted => "The device is currently mounted at this mount point.",
			MountStatus::Unmounted => "The device is not currently mounted at this mount point.",
			MountStatus::Missing => "This mount point does not exist on the system.",
		}
	}
}

pub fn detect(entry: &StabEntry) -> MountStatus {
	let mount_point = entry.mount_point.trim();
	if mount_point.is_empty() {
		return MountStatus::Missing;
	}

	if entry.fs_type == FsType::Swap {
		return if is_swap_active(&entry.device) {
			MountStatus::Mounted
		} else {
			MountStatus::Unmounted
		};
	}

	if !entry.fs_type.is_network() && !Path::new(mount_point).exists() {
		return MountStatus::Missing;
	}

	if is_mounted_at(entry, mount_point) {
		MountStatus::Mounted
	} else {
		MountStatus::Unmounted
	}
}

fn is_mounted_at(entry: &StabEntry, mount_point: &str) -> bool {
	let Some(mounts) = read_mounts() else {
		return false;
	};
	mounts
		.iter()
		.any(|(source, target, _fstype)| target == mount_point && device_matches(&entry.device, source))
}

fn device_matches(device: &DeviceValue, mount_source: &str) -> bool {
	let rendered = device.render();
	if rendered == mount_source {
		return true;
	}
	match (resolve_to_node(&rendered), resolve_to_node(mount_source)) {
		(Some(a), Some(b)) => a == b,
		_ => false,
	}
}

fn resolve_to_node(device: &str) -> Option<PathBuf> {
	let classified = DeviceKind::classify(device, &DeviceKind::ALL);
	if classified.kind == DeviceKind::Other {
		if device.starts_with('/') {
			std::fs::canonicalize(device).ok()
		} else {
			None
		}
	} else {
		classified.resolve_node()
	}
}

fn read_mounts() -> Option<Vec<(String, String, String)>> {
	let content = std::fs::read_to_string("/proc/self/mounts").ok()?;
	let mut mounts = Vec::new();
	for line in content.lines() {
		let mut fields = line.split_whitespace();
		let (Some(source), Some(target), Some(fstype)) = (fields.next(), fields.next(), fields.next()) else {
			continue;
		};
		mounts.push((unescape_field(source), unescape_field(target), fstype.to_string()));
	}
	Some(mounts)
}

fn is_swap_active(device: &DeviceValue) -> bool {
	let rendered = device.render();
	let node = resolve_to_node(&rendered);
	let Ok(content) = std::fs::read_to_string("/proc/swaps") else {
		return false;
	};
	content.lines().skip(1).any(|line| {
		let Some(source) = line.split_whitespace().next() else {
			return false;
		};
		source == rendered || node.as_deref().is_some_and(|node| Path::new(source) == node)
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn network_fs_never_reports_missing() {
		let mut entry = StabEntry::blank(0);
		entry.fs_type = FsType::Cifs;
		entry.device = DeviceValue::from("//server/share", DeviceKind::Network);
		entry.mount_point = "/mnt/does_not_exist_fstabulator_test".to_string();
		assert_ne!(detect(&entry), MountStatus::Missing);
	}

	#[test]
	fn local_missing_path() {
		let mut entry = StabEntry::blank(0);
		entry.fs_type = FsType::Ext4;
		entry.device = DeviceValue::from("UUID=deadbeef", DeviceKind::Uuid);
		entry.mount_point = "/mnt/does_not_exist_fstabulator_test".to_string();
		assert_eq!(detect(&entry), MountStatus::Missing);
	}

	#[test]
	fn empty_mount_point() {
		let entry = StabEntry::blank(0);
		assert_eq!(detect(&entry), MountStatus::Missing);
	}
}
