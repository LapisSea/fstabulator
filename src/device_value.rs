use crate::fs_value::FsType;
use crate::render_list_entry;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesRow};
use gtk::{Box as GtkBox, DropDown, Entry, Orientation, StringList};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
	Uuid,
	PartUuid,
	Label,
	PartLabel,
	DevicePath,
	Network,
	Other,
}

impl DeviceKind {
	pub const ALL: [DeviceKind; 6] = [
		DeviceKind::Uuid,
		DeviceKind::PartUuid,
		DeviceKind::Label,
		DeviceKind::PartLabel,
		DeviceKind::DevicePath,
		DeviceKind::Network,
	];

	pub const LOCAL: [DeviceKind; 5] = [
		DeviceKind::Uuid,
		DeviceKind::PartUuid,
		DeviceKind::Label,
		DeviceKind::PartLabel,
		DeviceKind::DevicePath,
	];

	pub fn label(self) -> &'static str {
		match self {
			DeviceKind::Uuid => "UUID",
			DeviceKind::PartUuid => "PARTUUID",
			DeviceKind::Label => "LABEL",
			DeviceKind::PartLabel => "PARTLABEL",
			DeviceKind::DevicePath => "Device path",
			DeviceKind::Network => "Network location",
			DeviceKind::Other => "Other",
		}
	}

	pub fn for_fs_type(fs_type: &FsType) -> &'static [DeviceKind] {
		match fs_type {
			FsType::Cifs | FsType::Smb3 | FsType::Nfs | FsType::Nfs4 | FsType::FuseSshfs => &[DeviceKind::Network],
			FsType::Iso9660 | FsType::Udf => &[DeviceKind::DevicePath, DeviceKind::Label, DeviceKind::PartLabel],
			FsType::Tmpfs
			| FsType::Proc
			| FsType::Sysfs
			| FsType::Devpts
			| FsType::Cgroup2
			| FsType::Securityfs
			| FsType::Debugfs
			| FsType::Tracefs
			| FsType::Configfs
			| FsType::Mqueue
			| FsType::Hugetlbfs
			| FsType::Devtmpfs
			| FsType::P9
			| FsType::Overlay
			| FsType::Zfs => &[],
			FsType::Ext2
			| FsType::Ext3
			| FsType::Ext4
			| FsType::Btrfs
			| FsType::Xfs
			| FsType::F2fs
			| FsType::Ntfs3
			| FsType::Vfat
			| FsType::Exfat
			| FsType::Swap => &DeviceKind::LOCAL,
			FsType::Other(_) => &DeviceKind::ALL,
		}
	}

	pub fn classify(device: &str, allowed: &[DeviceKind]) -> (Self, String) {
		for &kind in allowed {
			if let Some(value) = kind.value_of(device) {
				return (kind, value);
			}
		}
		(DeviceKind::Other, device.to_string())
	}

	fn value_of(self, device: &str) -> Option<String> {
		match self {
			DeviceKind::Uuid => device.strip_prefix("UUID=").map(str::to_string),
			DeviceKind::PartUuid => device.strip_prefix("PARTUUID=").map(str::to_string),
			DeviceKind::Label => device.strip_prefix("LABEL=").map(str::to_string),
			DeviceKind::PartLabel => device.strip_prefix("PARTLABEL=").map(str::to_string),
			DeviceKind::DevicePath => device.starts_with("/dev/").then(|| device.to_string()),
			DeviceKind::Network => (device.starts_with("//") || device.contains(":/")).then(|| device.to_string()),
			DeviceKind::Other => Some(device.to_string()),
		}
	}

	pub fn render(self, value: &str) -> String {
		match self {
			DeviceKind::Uuid => format!("UUID={value}"),
			DeviceKind::PartUuid => format!("PARTUUID={value}"),
			DeviceKind::Label => format!("LABEL={value}"),
			DeviceKind::PartLabel => format!("PARTLABEL={value}"),
			DeviceKind::DevicePath | DeviceKind::Network | DeviceKind::Other => value.to_string(),
		}
	}

	fn by_disk_dir(self) -> Option<&'static str> {
		match self {
			DeviceKind::Uuid => Some("/dev/disk/by-uuid"),
			DeviceKind::PartUuid => Some("/dev/disk/by-partuuid"),
			DeviceKind::Label => Some("/dev/disk/by-label"),
			DeviceKind::PartLabel => Some("/dev/disk/by-partlabel"),
			DeviceKind::DevicePath | DeviceKind::Network | DeviceKind::Other => None,
		}
	}

	/// Resolve a local device reference to its real block device node.
	fn resolve_node(self, value: &str) -> Option<PathBuf> {
		if let Some(dir) = self.by_disk_dir() {
			std::fs::canonicalize(Path::new(dir).join(value)).ok()
		} else if self == DeviceKind::DevicePath {
			std::fs::canonicalize(value).ok()
		} else {
			None
		}
	}

	/// Find the identifier of `node` (a real block device) for this kind.
	fn identify_node(self, node: &Path) -> Option<String> {
		if self == DeviceKind::DevicePath {
			return Some(friendly_device_path(node));
		}
		let dir = self.by_disk_dir()?;
		for entry in std::fs::read_dir(dir).ok()? {
			let path = entry.ok()?.path();
			if std::fs::canonicalize(&path).ok().as_deref() == Some(node) {
				return path.file_name()?.to_str().map(str::to_string);
			}
		}
		None
	}

	/// Convert `value` (a reference of kind `self`) into a reference of kind `to`, if possible.
	pub fn transform(self, value: &str, to: DeviceKind) -> Option<String> {
		let node = self.resolve_node(value)?;
		to.identify_node(&node)
	}
}

/// Prefer a stable, human-friendly name for a device node over the raw kernel node.
fn friendly_device_path(node: &Path) -> String {
	if let Ok(entries) = std::fs::read_dir("/dev/mapper") {
		for entry in entries.flatten() {
			let path = entry.path();
			if std::fs::canonicalize(&path).ok().as_deref() == Some(node) {
				return path.to_string_lossy().into_owned();
			}
		}
	}
	if let Ok(entries) = std::fs::read_dir("/dev/disk/by-id") {
		for entry in entries.flatten() {
			let path = entry.path();
			if std::fs::canonicalize(&path).ok().as_deref() == Some(node) {
				return path.to_string_lossy().into_owned();
			}
		}
	}
	node.to_string_lossy().into_owned()
}

pub fn add_device_row(options: &PreferencesGroup, entry: &Rc<RefCell<StabEntry>>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	let mut kinds = DeviceKind::for_fs_type(&entry.borrow().fs_type).to_vec();
	if !kinds.contains(&DeviceKind::Other) {
		kinds.push(DeviceKind::Other);
	}

	let (initial_kind, initial_value) = DeviceKind::classify(&entry.borrow().device, &kinds);
	let selected = kinds.iter().position(|k| *k == initial_kind).unwrap();

	let model = StringList::new(&kinds.iter().map(|k| k.label()).collect::<Vec<_>>());

	let dropdown = DropDown::builder().model(&model).selected(selected as u32).build();

	let value_entry = Entry::builder().text(&initial_value).hexpand(true).build();
	
	let input_row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(12).hexpand(true).build();
	input_row.append(&dropdown);
	input_row.append(&value_entry);
	
	let warning = gtk::Label::new(None);
	warning.set_xalign(0.0);
	warning.set_wrap(true);
	warning.set_visible(false);
	warning.add_css_class("error");
	
	let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).hexpand(true).build();
	content.append(&input_row);
	content.append(&warning);
	
	let row = PreferencesRow::builder().title("Device").child(&content).build();
	
	{
		let kinds_ref = kinds.clone();
		let entry_ref = entry.clone();
		let action_row_ref = action_row.clone();
		let dropdown_ref = dropdown.clone();
		let warning = warning.clone();
		let reset_btn = reset_btn.clone();
		value_entry.connect_changed(move |entry| {
			let kind = kinds_ref[dropdown_ref.selected() as usize];
			entry_ref.borrow_mut().device = kind.render(&entry.text());
			warning.set_visible(false);
			render_list_entry(&action_row_ref, &entry_ref.borrow(), Some(&reset_btn));
		});
	}
	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let value_entry = value_entry.clone();
		let warning = warning.clone();
		let reset_btn = reset_btn.clone();
		dropdown.connect_selected_notify(move |dropdown| {
			let new_kind = kinds[dropdown.selected() as usize];
			let current_device = entry.borrow().device.clone();
			let (current_kind, current_value) = DeviceKind::classify(&current_device, &DeviceKind::ALL);

			if new_kind == DeviceKind::Other {
				value_entry.set_text(&current_device);
				return;
			}
			if new_kind == current_kind {
				value_entry.set_text(&current_value);
				return;
			}

			let both_local = DeviceKind::LOCAL.contains(&current_kind) && DeviceKind::LOCAL.contains(&new_kind);
			match current_kind.transform(&current_value, new_kind) {
				Some(value) => {
					warning.set_visible(false);
					value_entry.set_text(&value);
					entry.borrow_mut().device = new_kind.render(&value);
				}
				None if both_local => {
					let value = value_entry.text().to_string();
					entry.borrow_mut().device = new_kind.render(&value);
					warning.set_label(&format!(
						"Could not resolve a {} for {}. The value was kept as-is.",
						new_kind.label(),
						current_device
					));
					warning.set_visible(true);
				}
				None => {
					let value = value_entry.text().to_string();
					entry.borrow_mut().device = new_kind.render(&value);
				}
			}
			render_list_entry(&action_row, &entry.borrow(), Some(&reset_btn));
		});
	}
	
	options.add(&row);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uuid_to_path_and_back() {
		let dir = match std::fs::read_dir("/dev/disk/by-uuid") {
			Ok(dir) => dir,
			Err(_) => return,
		};
		let Some(entry) = dir.filter_map(Result::ok).next() else { return };
		let uuid = entry.file_name().to_string_lossy().into_owned();

		let Some(path) = DeviceKind::Uuid.transform(&uuid, DeviceKind::DevicePath) else {
			panic!("could not resolve uuid {uuid}");
		};
		assert!(path.starts_with("/dev/"));

		let Some(back) = DeviceKind::DevicePath.transform(&path, DeviceKind::Uuid) else {
			panic!("could not resolve path {path}");
		};
		assert_eq!(back, uuid);
	}
}
