use crate::GC;
use crate::context::EntryContext;
use crate::device_value::DeviceKind;
use crate::i18n::i18n;
use crate::search_picker::SearchPickerBuilder;
use crate::stab_yurself::StabEntry;
use crate::ui_commons::{activatable_row, query_matches};
use adw::prelude::*;
use adw::{PreferencesGroup, PreferencesRow};
use gtk::{Align, Box as GtkBox, Entry, Orientation};
use std::rc::Rc;
use std::str::FromStr;
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
	pub fn is_network(&self) -> bool {
		matches!(DeviceKind::for_fs_type(self), [DeviceKind::Network])
	}

	pub fn description(&self) -> String {
		match self {
			FsType::Ext2 => i18n("original non-journaling ext filesystem, still used for small boot partitions"),
			FsType::Ext3 => i18n("ext2 with journaling added for crash-recovery reliability"),
			FsType::Ext4 => i18n("default Linux journaling filesystem with extents and very large volume support"),
			FsType::Btrfs => i18n("copy-on-write filesystem with snapshots, subvolumes, and RAID"),
			FsType::Xfs => i18n("SGI-developed 64-bit journaling filesystem for high performance and large files"),
			FsType::F2fs => i18n("flash-friendly filesystem designed for NAND flash storage"),
			FsType::Ntfs3 => i18n("Linux kernel driver providing read/write access to Windows NTFS volumes"),
			FsType::Vfat => i18n("FAT filesystem variant with long filename support for removable media"),
			FsType::Exfat => i18n("Microsoft filesystem for large removable drives and SD cards over 32 GB"),
			FsType::Swap => i18n("disk space acting as virtual memory when physical RAM is exhausted"),
			FsType::Cifs => i18n("network filesystem using the SMB protocol for Windows and NAS file shares"),
			FsType::Smb3 => i18n("modern, secure dialect of SMB used for Windows, Azure, and NAS shares"),
			FsType::Nfs => i18n("network filesystem for sharing files across machines over the network"),
			FsType::Nfs4 => i18n("newer NFS version adding security, statefulness, and better locking"),
			FsType::FuseSshfs => i18n("mounts a remote directory over SSH using SFTP, running in userspace via FUSE"),
			FsType::Iso9660 => i18n("standard filesystem format for CD-ROM optical media"),
			FsType::Udf => i18n("Universal Disk Format for writable optical discs like DVDs and Blu-rays"),
			FsType::Tmpfs => i18n("virtual filesystem storing files in RAM and swap for temporary in-memory data"),
			FsType::Proc => i18n("virtual filesystem exposing kernel and process information as files"),
			FsType::Sysfs => i18n("virtual filesystem exposing kernel objects, devices, and their attributes to userspace"),
			FsType::Devpts => i18n("virtual filesystem providing pseudo-terminal device nodes under /dev/pts"),
			FsType::Cgroup2 => i18n("virtual filesystem for the unified cgroup v2 hierarchy, managing resource limits and accounting"),
			FsType::Securityfs => i18n("virtual filesystem backing security modules such as SELinux and IMA integrity interfaces"),
			FsType::Debugfs => i18n("virtual filesystem exporting arbitrary kernel debugging data with no ABI stability guarantees"),
			FsType::Tracefs => i18n("virtual filesystem holding ftrace control and trace-output files under /sys/kernel/tracing"),
			FsType::Configfs => i18n("virtual filesystem where userspace creates and configures kernel objects via mkdir and rmdir"),
			FsType::Mqueue => i18n("POSIX message queue IPC exposed as files at /dev/mqueue"),
			FsType::Hugetlbfs => i18n("interface to huge page memory, mounted at /dev/hugepages"),
			FsType::Devtmpfs => i18n("populates /dev with device nodes automatically at boot"),
			FsType::P9 => i18n("Plan 9 remote filesystem protocol, used for host-to-VM file sharing"),
			FsType::Overlay => i18n("union filesystem stacking upper/lower layers, used by containers"),
			FsType::Zfs => i18n("OpenZFS copy-on-write filesystem with integrated volume management"),
			FsType::Bcachefs => i18n("copy-on-write, multi-device filesystem with built-in caching, compression, and checksumming"),
			FsType::Other(_) => i18n("a custom filesystem type"),
		}
	}
}

#[derive(Clone)]
enum FsChoice {
	Known(FsType),
	Other,
}

fn apply_fs_type(entry: &GC<StabEntry>, fs_type: FsType) {
	entry.borrow_mut().set_fs_type(fs_type);
}

pub fn add_fs_type_row(options: &PreferencesGroup, entry_ctx: &EntryContext, on_change: impl Fn() + 'static) {
	let entry = entry_ctx.entry().clone();
	let on_change = Rc::new(on_change);
	let choices: Vec<FsChoice> = FsType::iter()
		.filter(|e| !matches!(e, FsType::Other(_)))
		.map(FsChoice::Known)
		.chain([FsChoice::Other])
		.collect();

	let current = entry.cloned(|e| &e.fs_type);
	let is_other = matches!(&current, FsType::Other(_));
	let menu_label = if is_other { i18n("Other") } else { current.to_string() };

	let value_entry = Entry::builder().hexpand(true).build();
	value_entry.set_visible(is_other);

	let menu_btn_holder: GC<Option<gtk::MenuButton>> = GC::new(None);

	let dataset = {
		let choices = choices.clone();
		move || Ok(choices.clone())
	};
	let render_row = |choice: &FsChoice| match choice {
		FsChoice::Known(fs_type) => activatable_row(fs_type.to_string(), fs_type.description()),
		FsChoice::Other => activatable_row(i18n("Other…"), ""),
	};
	let filter = |query: &str, choice: &FsChoice| match choice {
		FsChoice::Other => true,
		FsChoice::Known(fs_type) => query_matches(query, &fs_type.to_string()) || query_matches(query, fs_type.description().as_str()),
	};
	let on_select = {
		let (entry_ctx, entry, value_entry) = (entry_ctx.clone(), entry.clone(), value_entry.clone());
		let (menu_btn_holder, on_change) = (menu_btn_holder.clone(), on_change.clone());
		move |choice: FsChoice, _index: usize| {
			let fs_type = match choice {
				FsChoice::Known(fs_type) => fs_type,
				FsChoice::Other => FsType::Other(value_entry.text().to_string()),
			};
			apply_fs_type(&entry, fs_type);
			let is_other = matches!(entry.borrow().fs_type, FsType::Other(_));
			value_entry.set_visible(is_other);
			if let Some(menu_btn) = menu_btn_holder.borrow().as_ref() {
				if is_other {
					menu_btn.set_label(i18n("Other").as_str());
				} else {
					menu_btn.set_label(&entry.borrow().fs_type.to_string());
				}
			}
			if is_other {
				value_entry.grab_focus();
			}
			entry_ctx.render();
			on_change();
		}
	};

	let menu_btn = SearchPickerBuilder::new(menu_label, dataset, render_row, on_select)
		.search_placeholder(i18n("Search filesystems"))
		.tooltip(i18n("Choose the filesystem type"))
		.error_message(i18n("Error loading filesystem types"))
		.filter(filter)
		.build();
	menu_btn.set_hexpand(true);
	*menu_btn_holder.borrow_mut() = Some(menu_btn.clone());

	{
		let (entry_ctx, entry, menu_btn) = (entry_ctx.clone(), entry.clone(), menu_btn.clone());
		value_entry.connect_changed(move |value_entry| {
			entry.borrow_mut().fs_type = FsType::Other(value_entry.text().to_string());
			menu_btn.set_label(i18n("Other").as_str());
			entry_ctx.render();
		});
	}

	{
		let (entry_ctx, entry, menu_btn) = (entry_ctx.clone(), entry.clone(), menu_btn.clone());
		let (value_entry, on_change) = (value_entry.clone(), on_change.clone());
		let controller = gtk::EventControllerFocus::new();
		value_entry.add_controller(controller.clone());
		controller.connect_leave(move |_| {
			let text = value_entry.text().trim().to_lowercase();
			match FsType::from_str(&text) {
				Ok(FsType::Other(_)) => {}
				Ok(fs_type) => {
					apply_fs_type(&entry, fs_type);
					menu_btn.set_label(&entry.borrow().fs_type.to_string());
					value_entry.set_visible(false);
					entry_ctx.render();
					on_change();
				}
				Err(_) => {}
			}
		});
	}

	let title_text = i18n("File system:");
	let title_label = gtk::Label::new(Some(title_text.as_str()));
	title_label.set_xalign(0.0);
	title_label.set_valign(Align::Center);
	title_label.set_margin_start(12);
	title_label.add_css_class("title");

	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(12).hexpand(true).build();
	content.append(&title_label);
	content.append(&menu_btn);
	content.append(&value_entry);

	let row = PreferencesRow::builder()
		.title(i18n("File system"))
		.activatable(false)
		.child(&content)
		.build();
	options.add(&row);
}
