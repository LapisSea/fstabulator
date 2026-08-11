use crate::fs_value::FsType;
use crate::stab_yurself::StabEntry;
use crate::{device_value, render_list_entry};
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesRow};
use gtk::{Box as GtkBox, DropDown, Entry, Orientation, StringList};
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeviceValue {
	pub value: String,
	pub kind: DeviceKind,
}

impl DeviceValue {
	pub fn from<T: Into<String>>(value: T, kind: DeviceKind) -> Self {
		Self::new(value.into(), kind)
	}
	pub fn new(value: String, kind: DeviceKind) -> Self {
		DeviceValue { value, kind }
	}

	pub fn resolve_node(&self) -> Option<PathBuf> {
		if let Some(dir) = self.kind.by_disk_dir() {
			std::fs::canonicalize(Path::new(dir).join(&self.value)).ok()
		} else if self.kind == DeviceKind::DevicePath {
			std::fs::canonicalize(&self.value).ok()
		} else {
			None
		}
	}

	/// Attempt to transform the value of a device from the current kind in to a
	/// new one. This way when changing the type of device, it does not become invalid
	pub fn transform(&self, to: DeviceKind) -> Option<DeviceValue> {
		let node = self.resolve_node()?;
		to.identify_node(&node).map(|value| Self::new(value, to))
	}

	pub fn render(&self) -> String {
		match self.kind {
			DeviceKind::Uuid => format!("UUID={}", self.value),
			DeviceKind::PartUuid => format!("PARTUUID={}", self.value),
			DeviceKind::Label => format!("LABEL={}", self.value),
			DeviceKind::PartLabel => format!("PARTLABEL={}", self.value),
			DeviceKind::DevicePath | DeviceKind::Network | DeviceKind::Other => self.value.clone(),
		}
	}
}

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
			DeviceKind::PartUuid => "Partition UUID",
			DeviceKind::Label => "Label",
			DeviceKind::PartLabel => "Partition Label",
			DeviceKind::DevicePath => "Device path",
			DeviceKind::Network => "Network location",
			DeviceKind::Other => "Other",
		}
	}

	pub fn for_fs_type(fs_type: &FsType) -> &'static [DeviceKind] {
		match fs_type {
			FsType::Cifs | FsType::Smb3 | FsType::Nfs | FsType::Nfs4 | FsType::FuseSshfs => &[DeviceKind::Network],
			FsType::Iso9660 | FsType::Udf => &[DeviceKind::DevicePath, DeviceKind::Label, DeviceKind::PartLabel],
			FsType::Tmpfs | FsType::Proc | FsType::Sysfs | FsType::Devpts | FsType::Cgroup2 => &[],
			FsType::Securityfs | FsType::Debugfs | FsType::Tracefs | FsType::Configfs | FsType::Mqueue => &[],
			FsType::Hugetlbfs | FsType::Devtmpfs | FsType::P9 | FsType::Overlay | FsType::Zfs => &[],
			FsType::Ext2 | FsType::Ext3 | FsType::Ext4 | FsType::Btrfs | FsType::Xfs | FsType::F2fs => &DeviceKind::LOCAL,
			FsType::Ntfs3 | FsType::Vfat | FsType::Exfat | FsType::Swap | FsType::Bcachefs => &DeviceKind::LOCAL,
			FsType::Other(_) => &DeviceKind::ALL,
		}
	}

	pub fn classify(device: &str, allowed: &[DeviceKind]) -> DeviceValue {
		for &kind in allowed {
			if let Some(value) = kind.value_of(device) {
				return value;
			}
		}
		DeviceValue {
			kind: DeviceKind::Other,
			value: device.to_owned(),
		}
	}

	fn value_of(self, device: &str) -> Option<DeviceValue> {
		let val = match self {
			DeviceKind::Uuid => device.strip_prefix("UUID="),
			DeviceKind::PartUuid => device.strip_prefix("PARTUUID="),
			DeviceKind::Label => device.strip_prefix("LABEL="),
			DeviceKind::PartLabel => device.strip_prefix("PARTLABEL="),
			DeviceKind::DevicePath => device.starts_with("/dev/").then(|| device),
			DeviceKind::Network => (device.starts_with("//") || device.contains(":/")).then(|| device),
			DeviceKind::Other => Some(device),
		};
		val.map(|val| DeviceValue::from(val, self))
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

	fn identify_node(self, node: &Path) -> Option<String> {
		if self == DeviceKind::DevicePath {
			return Some(friendly_device_path(node));
		}
		let path = find_node_in_dir(self.by_disk_dir()?, node)?;
		path.file_name()?.to_str().map(str::to_string)
	}
}

pub fn resolve_local_device(device: &str) -> Option<String> {
	let value = DeviceKind::classify(device, &DeviceKind::LOCAL);
	match value.kind {
		DeviceKind::Other => None,
		_ => value.resolve_node().map(|p| p.to_string_lossy().into_owned()),
	}
}

fn find_node_in_dir(dir: &str, node: &Path) -> Option<PathBuf> {
	std::fs::read_dir(dir)
		.ok()?
		.flatten()
		.map(|entry| entry.path())
		.find(|path| std::fs::canonicalize(path).ok().as_deref() == Some(node))
}

fn friendly_device_path(node: &Path) -> String {
	["/dev/mapper", "/dev/disk/by-id"]
		.into_iter()
		.find_map(|dir| find_node_in_dir(dir, node).map(|p| p.to_string_lossy().into_owned()))
		.unwrap_or_else(|| node.to_string_lossy().into_owned())
}

pub fn add_device_row(options: &PreferencesGroup, entry: &Rc<RefCell<StabEntry>>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	let mut kinds = DeviceKind::for_fs_type(&entry.borrow().fs_type).to_vec();
	if !kinds.contains(&DeviceKind::Other) {
		kinds.push(DeviceKind::Other);
	}

	let initial = &entry.borrow().device;
	let selected = kinds.iter().position(|k| *k == initial.kind).unwrap();

	let model = StringList::new(&kinds.iter().map(|k| k.label()).collect::<Vec<_>>());

	let dropdown = DropDown::builder().model(&model).selected(selected as u32).build();

	let value_entry = Entry::builder().text(&initial.value).hexpand(true).build();

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
			entry_ref.borrow_mut().device = DeviceValue::from(entry.text(), kind);
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
			let current = &entry.borrow().device;

			if new_kind == DeviceKind::Other {
				value_entry.set_text(&current.value);
				return;
			}
			if new_kind == current.kind {
				value_entry.set_text(&current.value);
				return;
			}

			let both_local = DeviceKind::LOCAL.contains(&current.kind) && DeviceKind::LOCAL.contains(&new_kind);
			match current.transform(new_kind) {
				Some(device) => {
					warning.set_visible(false);
					value_entry.set_text(&device.value);
					entry.borrow_mut().device = device;
				}
				None if both_local => {
					entry.borrow_mut().device = DeviceValue::from(value_entry.text(), new_kind);
					warning.set_label(&format!(
						"Could not resolve a {} for {}. The value was kept as-is.",
						new_kind.label(),
						current.value
					));
					warning.set_visible(true);
				}
				None => {
					let value = value_entry.text().to_string();
					entry.borrow_mut().device = DeviceValue::new(value, new_kind);
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
		let uuid = &entry.file_name().to_string_lossy().into_owned();

		let Some(path) = DeviceValue::from(uuid, DeviceKind::Uuid).transform(DeviceKind::DevicePath) else {
			panic!("could not resolve uuid {uuid}");
		};
		assert_eq!(path.kind, DeviceKind::DevicePath);
		let path = &path.value;
		assert!(path.starts_with("/dev/"));

		let Some(back) = DeviceValue::from(path, DeviceKind::DevicePath).transform(DeviceKind::Uuid) else {
			panic!("could not resolve path {path}");
		};
		assert_eq!(back.value, *uuid);
	}
}
