use crate::GC;
use crate::context::EntryContext;
use crate::fs_value::FsType;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{PreferencesGroup, PreferencesRow};
use gtk::{Box as GtkBox, DropDown, Entry, Orientation, StringList};
use std::path::{Path, PathBuf};

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

	pub fn reclassify_for(&self, fs_type: &FsType) -> DeviceValue {
		let mut reclassified = DeviceKind::classify(&self.render(), &DeviceKind::for_fs_type(fs_type));
		if self.kind == DeviceKind::Other && self.value.is_empty() && fs_type.is_network() {
			reclassified.kind = DeviceKind::Network;
		}
		reclassified
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

#[derive(Clone)]
pub struct DeviceRowController {
	entry: GC<StabEntry>,
	dropdown: DropDown,
	kinds: GC<Vec<DeviceKind>>,
	model: StringList,
}

impl DeviceRowController {
	pub fn refresh_kinds(&self) {
		let device = self.entry.cloned(|e| &e.device);
		let mut new_kinds = DeviceKind::for_fs_type(&self.entry.borrow().fs_type).to_vec();
		if !new_kinds.contains(&DeviceKind::Other) {
			new_kinds.push(DeviceKind::Other);
		}
		let selected = new_kinds.iter().position(|k| *k == device.kind).unwrap_or_else(|| {
			new_kinds
				.iter()
				.position(|k| *k == DeviceKind::Other)
				.expect("Other is always appended to kinds")
		});
		*self.kinds.borrow_mut() = new_kinds;
		self.model.splice(
			0,
			self.model.n_items(),
			&self.kinds.borrow().iter().map(|k| k.label()).collect::<Vec<_>>(),
		);
		self.dropdown.set_selected(selected as u32);
	}
}

pub fn add_device_row(options: &PreferencesGroup, entry_ctx: &EntryContext) -> DeviceRowController {
	let entry = entry_ctx.entry().clone();
	let kinds: GC<Vec<DeviceKind>> = GC::new(DeviceKind::for_fs_type(&entry.borrow().fs_type).to_vec());
	if !kinds.borrow().contains(&DeviceKind::Other) {
		kinds.borrow_mut().push(DeviceKind::Other);
	}

	let initial = &entry.borrow().device;
	let selected = kinds.borrow().iter().position(|k| *k == initial.kind).unwrap_or_else(|| {
		kinds
			.borrow()
			.iter()
			.position(|k| *k == DeviceKind::Other)
			.expect("Other is always appended to kinds")
	});

	let model = StringList::new(&kinds.borrow().iter().map(|k| k.label()).collect::<Vec<_>>());

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
		let entry_ctx = entry_ctx.clone();
		let kinds_ref = kinds.clone();
		let entry_ref = entry.clone();
		let dropdown_ref = dropdown.clone();
		let warning = warning.clone();
		value_entry.connect_changed(move |entry| {
			let Some(&kind) = kinds_ref.borrow().get(dropdown_ref.selected() as usize) else {
				return;
			};
			entry_ref.borrow_mut().device = DeviceValue::from(entry.text(), kind);
			warning.set_visible(false);
			entry_ctx.render();
		});
	}
	{
		let entry_ctx = entry_ctx.clone();
		let entry = entry.clone();
		let value_entry = value_entry.clone();
		let warning = warning.clone();
		let kinds = kinds.clone();
		dropdown.connect_selected_notify(move |dropdown| {
			let Some(&new_kind) = kinds.borrow().get(dropdown.selected() as usize) else {
				return;
			};
			let current = entry.cloned(|e| &e.device);

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
			entry_ctx.render();
		});
	}

	options.add(&row);

	DeviceRowController {
		entry: entry.clone(),
		dropdown,
		kinds,
		model,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reclassify_for_switches_kind_with_fs() {
		let uuid = DeviceValue::from("abc", DeviceKind::Uuid);
		let re = uuid.reclassify_for(&FsType::Tmpfs);
		assert_eq!(re.kind, DeviceKind::Other);
		assert_eq!(re.value, "UUID=abc");

		let share = DeviceValue::from("//server/share", DeviceKind::Other);
		let re = share.reclassify_for(&FsType::Cifs);
		assert_eq!(re.kind, DeviceKind::Network);
		assert_eq!(re.value, "//server/share");
	}

	#[test]
	fn reclassify_for_empty_value_becomes_network_location() {
		let blank = DeviceValue::from("", DeviceKind::Other);
		let re = blank.reclassify_for(&FsType::Cifs);
		assert_eq!(re.kind, DeviceKind::Network);
		assert_eq!(re.value, "");

		let re = blank.reclassify_for(&FsType::Ext4);
		assert_eq!(re.kind, DeviceKind::Other);
	}

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
