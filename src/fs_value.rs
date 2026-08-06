use crate::render_list_entry;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesRow};
use gtk::{Box as GtkBox, DropDown, Entry, Orientation, StringList};
use std::cell::RefCell;
use std::rc::Rc;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Clone, PartialEq, Eq, Debug, Display, EnumString, EnumIter)]
pub enum FsType {
	#[strum(serialize = "ext2")]
	Ext2,
	#[strum(serialize = "ext3")]
	Ext3,
	#[strum(serialize = "ext4")]
	Ext4,
	#[strum(serialize = "btrfs")]
	Btrfs,
	#[strum(serialize = "xfs")]
	Xfs,
	#[strum(serialize = "f2fs")]
	F2fs,
	#[strum(serialize = "ntfs3")]
	Ntfs3,
	#[strum(serialize = "vfat")]
	Vfat,
	#[strum(serialize = "exfat")]
	Exfat,
	#[strum(serialize = "swap")]
	Swap,
	#[strum(serialize = "cifs")]
	Cifs,
	#[strum(serialize = "smb3")]
	Smb3,
	#[strum(serialize = "nfs")]
	Nfs,
	#[strum(serialize = "nfs4")]
	Nfs4,
	#[strum(serialize = "fuse.sshfs")]
	FuseSshfs,
	#[strum(serialize = "iso9660")]
	Iso9660,
	#[strum(serialize = "udf")]
	Udf,
	#[strum(serialize = "tmpfs")]
	Tmpfs,
	#[strum(serialize = "proc")]
	Proc,
	#[strum(serialize = "sysfs")]
	Sysfs,
	#[strum(serialize = "devpts")]
	Devpts,
	#[strum(serialize = "cgroup2")]
	Cgroup2,
	#[strum(serialize = "securityfs")]
	Securityfs,
	#[strum(serialize = "debugfs")]
	Debugfs,
	#[strum(serialize = "tracefs")]
	Tracefs,
	#[strum(serialize = "configfs")]
	Configfs,
	#[strum(serialize = "mqueue")]
	Mqueue,
	#[strum(serialize = "hugetlbfs")]
	Hugetlbfs,
	#[strum(serialize = "devtmpfs")]
	Devtmpfs,
	#[strum(serialize = "9p")]
	P9,
	#[strum(serialize = "overlay")]
	Overlay,
	#[strum(serialize = "zfs")]
	Zfs,

	#[strum(default)]
	Other(String),
}

pub fn add_fs_type_row(options: &PreferencesGroup, entry: &Rc<RefCell<StabEntry>>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	let known: Vec<FsType> = FsType::iter().filter(|e| !matches!(e, FsType::Other(_))).collect();
	let mut labels: Vec<String> = known.iter().map(|e| e.to_string()).collect();
	labels.push("Other".to_string());
	let other_index = known.len();

	let model = StringList::new(&labels.iter().map(String::as_str).collect::<Vec<_>>());

	let current = entry.borrow().fs_type.clone();

	let dropdown = DropDown::builder().model(&model).build();
	let value_entry = Entry::builder().hexpand(true).build();

	value_entry.set_text(&current.to_string());

	{
		let entry_ref = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		value_entry.connect_changed(move |entry| {
			entry_ref.borrow_mut().fs_type = FsType::Other(entry.text().to_string());
			render_list_entry(&action_row, &entry_ref.borrow(), Some(&reset_btn));
		});
	}
	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let value_entry = value_entry.clone();
		let known = known.clone();
		let reset_btn = reset_btn.clone();
		dropdown.connect_selected_notify(move |dropdown| {
			let selected = dropdown.selected() as usize;
			let mut entry = entry.borrow_mut();
			let is_other = selected == other_index;
			if is_other {
				entry.fs_type = FsType::Other(value_entry.text().to_string());
			} else if let Some(fs_type) = known.get(selected) {
				entry.fs_type = fs_type.clone();
			}
			value_entry.set_visible(is_other);
			dropdown.set_hexpand(!is_other);
			render_list_entry(&action_row, &entry, Some(&reset_btn));
		});
	}

	dropdown.set_selected(match current {
		FsType::Other(_) => other_index,
		_ => known.iter().position(|t| t == &current).unwrap_or(0),
	} as u32);

	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(12).hexpand(true).build();
	content.append(&dropdown);
	content.append(&value_entry);

	let row = PreferencesRow::builder().title("File system").child(&content).build();

	options.add(&row);
}
