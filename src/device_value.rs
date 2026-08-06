use crate::fs_value::FsType;
use crate::render_list_entry;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesRow};
use gtk::{Box as GtkBox, DropDown, Entry, Orientation, StringList};
use std::cell::RefCell;
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

	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(12).hexpand(true).build();
	content.append(&dropdown);
	content.append(&value_entry);

	let row = PreferencesRow::builder().title("Device").child(&content).build();

	{
		let kinds_ref = kinds.clone();
		let entry_ref = entry.clone();
		let action_row_ref = action_row.clone();
		let dropdown_ref = dropdown.clone();
		let reset_btn = reset_btn.clone();
		value_entry.connect_changed(move |entry| {
			let kind = kinds_ref[dropdown_ref.selected() as usize];
			entry_ref.borrow_mut().device = kind.render(&entry.text());
			render_list_entry(&action_row_ref, &entry_ref.borrow(), Some(&reset_btn));
		});
	}
	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let value_entry = value_entry.clone();
		let reset_btn = reset_btn.clone();
		dropdown.connect_selected_notify(move |dropdown| {
			let kind = kinds[dropdown.selected() as usize];
			if kind == DeviceKind::Other {
				let full = entry.borrow().device.clone();
				value_entry.set_text(&full);
			} else {
				let value = value_entry.text().to_string();
				entry.borrow_mut().device = kind.render(&value);
				render_list_entry(&action_row, &entry.borrow(), Some(&reset_btn));
			}
		});
	}

	options.add(&row);
}
