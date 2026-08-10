use crate::build_search_picker;
use crate::clear_list;
use crate::render_list_entry;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup, PreferencesRow};
use gtk::{Align, Box as GtkBox, Entry, ListBox, Orientation};
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
	#[strum(serialize = "bcachefs")]
	Bcachefs,

	#[strum(default)]
	Other(String),
}

impl FsType {
	pub fn description(&self) -> &'static str {
		match self {
			FsType::Ext2 => "original non-journaling ext filesystem, still used for small boot partitions",
			FsType::Ext3 => "ext2 with journaling added for crash-recovery reliability",
			FsType::Ext4 => "default Linux journaling filesystem with extents and very large volume support",
			FsType::Btrfs => "copy-on-write filesystem with snapshots, subvolumes, and RAID",
			FsType::Xfs => "SGI-developed 64-bit journaling filesystem for high performance and large files",
			FsType::F2fs => "flash-friendly filesystem designed for NAND flash storage",
			FsType::Ntfs3 => "Linux kernel driver providing read/write access to Windows NTFS volumes",
			FsType::Vfat => "FAT filesystem variant with long filename support for removable media",
			FsType::Exfat => "Microsoft filesystem for large removable drives and SD cards over 32 GB",
			FsType::Swap => "disk space acting as virtual memory when physical RAM is exhausted",
			FsType::Cifs => "network filesystem using the SMB protocol for Windows and NAS file shares",
			FsType::Smb3 => "modern, secure dialect of SMB used for Windows, Azure, and NAS shares",
			FsType::Nfs => "network filesystem for sharing files across machines over the network",
			FsType::Nfs4 => "newer NFS version adding security, statefulness, and better locking",
			FsType::FuseSshfs => "mounts a remote directory over SSH using SFTP, running in userspace via FUSE",
			FsType::Iso9660 => "standard filesystem format for CD-ROM optical media",
			FsType::Udf => "Universal Disk Format for writable optical discs like DVDs and Blu-rays",
			FsType::Tmpfs => "virtual filesystem storing files in RAM and swap for temporary in-memory data",
			FsType::Proc => "virtual filesystem exposing kernel and process information as files",
			FsType::Sysfs => "virtual filesystem exposing kernel objects, devices, and their attributes to userspace",
			FsType::Devpts => "virtual filesystem providing pseudo-terminal device nodes under /dev/pts",
			FsType::Cgroup2 => "virtual filesystem for the unified cgroup v2 hierarchy, managing resource limits and accounting",
			FsType::Securityfs => "virtual filesystem backing security modules such as SELinux and IMA integrity interfaces",
			FsType::Debugfs => "virtual filesystem exporting arbitrary kernel debugging data with no ABI stability guarantees",
			FsType::Tracefs => "virtual filesystem holding ftrace control and trace-output files under /sys/kernel/tracing",
			FsType::Configfs => "virtual filesystem where userspace creates and configures kernel objects via mkdir and rmdir",
			FsType::Mqueue => "POSIX message queue IPC exposed as files at /dev/mqueue",
			FsType::Hugetlbfs => "interface to huge page memory, mounted at /dev/hugepages",
			FsType::Devtmpfs => "populates /dev with device nodes automatically at boot",
			FsType::P9 => "Plan 9 remote filesystem protocol, used for host-to-VM file sharing",
			FsType::Overlay => "union filesystem stacking upper/lower layers, used by containers",
			FsType::Zfs => "OpenZFS copy-on-write filesystem with integrated volume management",
			FsType::Bcachefs => "copy-on-write, multi-device filesystem with built-in caching, compression, and checksumming",
			FsType::Other(_) => "a custom filesystem type",
		}
	}
}

pub fn add_fs_type_row(
	options: &PreferencesGroup,
	entry: &Rc<RefCell<StabEntry>>,
	action_row: &ActionRow,
	reset_btn: &gtk::Button,
	on_change: impl Fn() + 'static,
) {
	let known: Vec<FsType> = FsType::iter().filter(|e| !matches!(e, FsType::Other(_))).collect();

	let current = entry.borrow().fs_type.clone();

	let displayed: Rc<RefCell<Vec<FsType>>> = Rc::new(RefCell::new(Vec::new()));
	let populate = {
		let known = known.clone();
		let displayed = displayed.clone();
		move |list: &ListBox, query: &str| populate_fs_list(list, &known, query, &displayed)
	};
	let is_other = matches!(&current, FsType::Other(_));
	let menu_label = if is_other { "Other" } else { &current.to_string() };
	let picker = build_search_picker("Search filesystems", menu_label, "Choose the filesystem type", populate);
	picker.menu_btn.set_hexpand(true);
	let menu_btn = picker.menu_btn;
	let popover = picker.popover;
	let list_box = picker.list_box;

	let value_entry = Entry::builder().hexpand(true).build();
	value_entry.set_text(&current.to_string());
	value_entry.set_visible(is_other);

	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		let menu_btn = menu_btn.clone();
		value_entry.connect_changed(move |value_entry| {
			entry.borrow_mut().fs_type = FsType::Other(value_entry.text().to_string());
			menu_btn.set_label("Other");
			render_list_entry(&action_row, &entry.borrow(), Some(&reset_btn));
		});
	}

	let title_label = gtk::Label::new(Some("File system:"));
	title_label.set_xalign(0.0);
	title_label.set_valign(Align::Center);
	title_label.set_margin_start(12);
	title_label.add_css_class("title");

	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(12).hexpand(true).build();
	content.append(&title_label);
	content.append(&menu_btn);
	content.append(&value_entry);

	let row = PreferencesRow::builder().title("File system").child(&content).build();
	options.add(&row);

	{
		let displayed = displayed.clone();
		let entry = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		let popover = popover.clone();
		let menu_btn = menu_btn.clone();
		let value_entry = value_entry.clone();
		list_box.connect_row_activated(move |_, row| {
			let index = row.index() as usize;
			let is_other = index >= displayed.borrow().len();
			if is_other {
				entry.borrow_mut().fs_type = FsType::Other(value_entry.text().to_string());
			} else if let Some(fs_type) = displayed.borrow().get(index).cloned() {
				entry.borrow_mut().fs_type = fs_type;
			}
			let is_other = matches!(entry.borrow().fs_type, FsType::Other(_));
			value_entry.set_visible(is_other);
			if is_other {
				menu_btn.set_label("Other");
			} else {
				menu_btn.set_label(&entry.borrow().fs_type.to_string());
			}
			if is_other {
				value_entry.grab_focus();
			}
			render_list_entry(&action_row, &entry.borrow(), Some(&reset_btn));
			popover.popdown();
			on_change();
		});
	}
}

fn populate_fs_list(list: &ListBox, known: &[FsType], query: &str, displayed: &RefCell<Vec<FsType>>) {
	clear_list(list);

	let query = query.trim().to_lowercase();
	let mut items = Vec::new();
	for fs_type in known {
		if query.is_empty() || fs_type.to_string().to_lowercase().contains(&query) {
			let row = ActionRow::builder().title(fs_type.to_string()).subtitle(fs_type.description()).build();
			row.set_activatable(true);
			list.append(&row);
			items.push(fs_type.clone());
		}
	}
	let other = ActionRow::builder().title("Other…").build();
	other.set_activatable(true);
	list.append(&other);
	*displayed.borrow_mut() = items;
}
