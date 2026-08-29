use crate::block_devices::{BlockDeviceInfo, list_block_devices};
use crate::context::EntryContext;
use crate::fs_value::FsType;
use crate::i18n::{i18n, i18n_fmt};
use crate::stab_yurself::StabEntry;
use crate::{GC, problem_reports, ui_commons};
use adw::prelude::*;
use adw::{Dialog, EntryRow, PreferencesGroup, PreferencesRow};
use glib::subclass::prelude::*;
use gtk::{
	Align, Box as GtkBox, Button, ColumnView, ColumnViewColumn, CustomFilter, DropDown, Entry, FilterListModel, Label, ListScrollFlags, Orientation,
	ScrolledWindow, SearchEntry, SignalListItemFactory, SingleSelection, StringList,
};
use std::cell::RefCell;
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
		} else if self.kind == DeviceKind::DevicePath || self.kind == DeviceKind::FilePath {
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
		let mut reclassified = DeviceKind::classify(&self.render(), DeviceKind::for_fs_type(fs_type));
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
			DeviceKind::DevicePath | DeviceKind::FilePath | DeviceKind::Network | DeviceKind::Other => self.value.clone(),
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetworkStyle {
	/// A `//server/share` location (CIFS/SMB)
	Smb,
	/// A `server:/export` location (NFS, sshfs)
	HostPath,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NetworkLocation {
	pub style: NetworkStyle,
	pub user: Option<String>,
	pub host: String,
	pub port: Option<String>,
	/// Share or export path after the host. HostPath locations keep the leading `/`.
	pub path: Option<String>,
}

impl NetworkLocation {
	pub fn parse(raw: &str) -> Option<Self> {
		let raw = raw.trim();
		if raw.is_empty() {
			return None;
		}
		if let Some(after) = raw.strip_prefix("//") {
			let (authority, path) = match after.split_once('/') {
				Some((authority, path)) => (authority, Some(path.to_string())),
				None => (after, None),
			};
			let (user, host, port) = parse_authority(authority)?;
			return Some(NetworkLocation {
				style: NetworkStyle::Smb,
				user,
				host,
				port,
				path,
			});
		}
		let colon = find_colon_outside_brackets(raw)?;
		let (authority, path) = (&raw[..colon], &raw[colon + 1..]);
		let (user, host, port) = parse_authority(authority)?;
		Some(NetworkLocation {
			style: NetworkStyle::HostPath,
			user,
			host,
			port,
			path: Some(path.to_string()),
		})
	}

	pub fn render(&self) -> String {
		let host = if self.host.contains(':') {
			format!("[{}]", self.host)
		} else {
			self.host.clone()
		};
		let authority = match (&self.user, &self.port) {
			(Some(user), Some(port)) => format!("{user}@{host}:{port}"),
			(Some(user), None) => format!("{user}@{host}"),
			(None, Some(port)) => format!("{host}:{port}"),
			(None, None) => host,
		};
		match self.style {
			NetworkStyle::Smb => match &self.path {
				Some(path) => format!("//{authority}/{path}"),
				None => format!("//{authority}"),
			},
			NetworkStyle::HostPath => format!("{authority}:{}", self.path.as_deref().unwrap_or_default()),
		}
	}
}

fn parse_authority(authority: &str) -> Option<(Option<String>, String, Option<String>)> {
	let (user, rest) = match authority.split_once('@') {
		Some((user, rest)) => (Some(user.to_string()).filter(|u| !u.is_empty()), rest),
		None => (None, authority),
	};
	let (host, port) = if let Some(inner) = rest.strip_prefix('[') {
		let close = inner.find(']')?;
		let host = &inner[..close];
		let port = inner[close + 1..].strip_prefix(':').map(str::to_string).filter(|p| !p.is_empty());
		(host.to_string(), port)
	} else if let Some((host, port)) = rest.rsplit_once(':') {
		if host.is_empty() || host.contains(':') || port.contains(':') {
			(rest.to_string(), None)
		} else if port.is_empty() {
			(host.to_string(), None)
		} else {
			(host.to_string(), Some(port.to_string()))
		}
	} else {
		(rest.to_string(), None)
	};
	if host.is_empty() {
		return None;
	}
	Some((user, host, port))
}

fn find_colon_outside_brackets(s: &str) -> Option<usize> {
	let mut in_brackets = false;
	for (i, c) in s.char_indices() {
		match c {
			'[' => in_brackets = true,
			']' => in_brackets = false,
			':' if !in_brackets => return Some(i),
			_ => {}
		}
	}
	None
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
	Uuid,
	PartUuid,
	Label,
	PartLabel,
	DevicePath,
	FilePath,
	Network,
	Other,
}

impl DeviceKind {
	pub const ALL: [DeviceKind; 7] = [
		DeviceKind::Uuid,
		DeviceKind::PartUuid,
		DeviceKind::Label,
		DeviceKind::PartLabel,
		DeviceKind::DevicePath,
		DeviceKind::FilePath,
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
			DeviceKind::FilePath => "File path",
			DeviceKind::Network => "Network location",
			DeviceKind::Other => "Other",
		}
	}

	pub fn is_local(self) -> bool {
		self == DeviceKind::FilePath || DeviceKind::LOCAL.contains(&self)
	}

	pub fn for_fs_type(fs_type: &FsType) -> &'static [DeviceKind] {
		match fs_type {
			FsType::Cifs | FsType::Smb3 | FsType::Nfs | FsType::Nfs4 | FsType::FuseSshfs => &[DeviceKind::Network],
			FsType::Iso9660 | FsType::Udf => &[DeviceKind::DevicePath, DeviceKind::Label, DeviceKind::PartLabel],
			FsType::Tmpfs | FsType::Proc | FsType::Sysfs | FsType::Devpts | FsType::Cgroup2 => &[],
			FsType::Securityfs | FsType::Debugfs | FsType::Tracefs | FsType::Configfs | FsType::Mqueue => &[],
			FsType::Hugetlbfs | FsType::Devtmpfs | FsType::P9 | FsType::Overlay | FsType::Zfs => &[],
			FsType::Ext2 | FsType::Ext3 | FsType::Ext4 | FsType::Btrfs | FsType::Xfs | FsType::F2fs => &DeviceKind::LOCAL,
			FsType::Ntfs3 | FsType::Vfat | FsType::Exfat | FsType::Bcachefs => &DeviceKind::LOCAL,
			FsType::Swap => &[
				DeviceKind::Uuid,
				DeviceKind::PartUuid,
				DeviceKind::Label,
				DeviceKind::PartLabel,
				DeviceKind::DevicePath,
				DeviceKind::FilePath,
			],
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
			DeviceKind::DevicePath => device.starts_with("/dev/").then_some(device),
			DeviceKind::FilePath => (device.starts_with('/') && !device.starts_with("/dev/")).then_some(device),
			DeviceKind::Network => (device.starts_with("//") || device.contains(":/")).then_some(device),
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
			DeviceKind::DevicePath | DeviceKind::FilePath | DeviceKind::Network | DeviceKind::Other => None,
		}
	}

	fn identify_node(self, node: &Path) -> Option<String> {
		if self == DeviceKind::DevicePath {
			return Some(friendly_device_path(node));
		}
		if self == DeviceKind::FilePath {
			return Some(node.to_string_lossy().into_owned());
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

fn network_style_for_fs(fs_type: &FsType) -> NetworkStyle {
	match fs_type {
		FsType::Cifs | FsType::Smb3 => NetworkStyle::Smb,
		_ => NetworkStyle::HostPath,
	}
}

fn default_port_for_fs(fs_type: &FsType) -> Option<u16> {
	match fs_type {
		FsType::Cifs | FsType::Smb3 => Some(445),
		FsType::Nfs | FsType::Nfs4 => Some(2049),
		FsType::FuseSshfs => Some(22),
		_ => None,
	}
}

fn resolve_test_port(fs_type: &FsType, port_text: &str) -> Option<u16> {
	if port_text.is_empty() {
		return default_port_for_fs(fs_type);
	}
	port_text.parse().ok()
}

fn set_connection_status(label: &Label, text: &str, error: bool) {
	label.set_label(text);
	label.remove_css_class("connection-ok");
	label.remove_css_class("invalid-alert");
	label.add_css_class(if error { "invalid-alert" } else { "connection-ok" });
}

fn test_network_connection(host: &str, port: u16) -> Result<(), String> {
	use std::net::ToSocketAddrs;
	use std::time::Duration;

	let addr = (host, port)
		.to_socket_addrs()
		.map_err(|err| i18n_fmt("Could not resolve '{host}': {err}", &[("{host}", host), ("{err}", &err.to_string())]))?
		.next()
		.ok_or_else(|| i18n_fmt("Could not resolve '{host}'.", &[("{host}", host)]))?;
	std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(3))
		.map(|_| ())
		.map_err(|err| {
			i18n_fmt(
				"Could not connect to {host}:{port}: {err}",
				&[("{host}", host), ("{port}", &port.to_string()), ("{err}", &err.to_string())],
			)
		})
}

#[derive(Clone)]
pub struct DeviceRowController {
	entry: GC<StabEntry>,
	entry_ctx: EntryContext,
	dropdown: DropDown,
	kinds: GC<Vec<DeviceKind>>,
	model: StringList,
	syncing: GC<bool>,
	style: GC<NetworkStyle>,
	value_entry: Entry,
	picker_btn: Button,
	issue_icon: gtk::Image,
}

fn kinds_with_selected(fs_type: &FsType, current_kind: DeviceKind) -> (Vec<DeviceKind>, usize) {
	let mut kinds = DeviceKind::for_fs_type(fs_type).to_vec();
	if !kinds.contains(&DeviceKind::Other) {
		kinds.push(DeviceKind::Other);
	}
	let selected = match kinds.iter().position(|k| *k == current_kind) {
		Some(selected) => selected,
		None => kinds.len() - 1,
	};
	(kinds, selected)
}

impl DeviceRowController {
	pub fn refresh_kinds(&self) {
		let (device, fs_type) = {
			let entry = self.entry.borrow();
			(entry.device.clone(), entry.fs_type.clone())
		};
		let (new_kinds, selected) = kinds_with_selected(&fs_type, device.kind);
		*self.kinds.borrow_mut() = new_kinds;
		let labels: Vec<String> = self.kinds.borrow().iter().map(|k| i18n(k.label())).collect();
		self.model
			.splice(0, self.model.n_items(), &labels.iter().map(String::as_str).collect::<Vec<_>>());
		self.dropdown.set_selected(selected as u32);
		self.sync_kind();
	}

	fn set_device(&self, device: DeviceValue) {
		self.entry.borrow_mut().device = device;
		self.sync_kind();
		self.entry_ctx.render();
	}

	fn refresh_issue(&self) {
		let entry = self.entry.borrow();
		let problem = problem_reports::check(&problem_reports::CheckValue::Device(entry.device.render()), &entry);
		ui_commons::update_issue_icon(&self.issue_icon, problem.as_ref());
	}

	fn sync_kind(&self) {
		let kind = self.entry.borrow().device.kind;
		let show_picker = kind != DeviceKind::Other && kind != DeviceKind::FilePath;
		*self.syncing.borrow_mut() = true;
		self.picker_btn.set_visible(show_picker);
		if show_picker {
			let icon = if kind == DeviceKind::Network {
				"preferences-system-symbolic"
			} else {
				"drive-harddisk-symbolic"
			};
			self.picker_btn.set_icon_name(icon);
		}
		let value = self.entry.borrow().device.value.clone();
		self.value_entry.set_text(&value);
		*self.syncing.borrow_mut() = false;
		self.refresh_issue();
	}

	fn open_network_editor(&self) {
		let (value, fs_type) = {
			let entry = self.entry.borrow();
			(entry.device.value.clone(), entry.fs_type.clone())
		};
		let (user, host, port, path, style) = match NetworkLocation::parse(&value) {
			Some(loc) => (loc.user, loc.host, loc.port, loc.path, loc.style),
			None => (None, String::new(), None, None, network_style_for_fs(&fs_type)),
		};
		*self.style.borrow_mut() = style;

		let user_entry = EntryRow::builder().title(i18n("User")).text(user.as_deref().unwrap_or("")).build();
		let host_entry = EntryRow::builder().title(i18n("Host")).text(&host).build();
		let port_entry = EntryRow::builder().title(i18n("Port")).text(port.as_deref().unwrap_or("")).build();
		let path_entry = EntryRow::builder().title(i18n("Path")).text(path.as_deref().unwrap_or("")).build();

		let test_btn = Button::with_label(i18n("Test connection").as_str());
		let status_label = Label::builder().halign(Align::Start).wrap(true).build();
		let test_row = GtkBox::builder()
			.orientation(Orientation::Horizontal)
			.spacing(6)
			.valign(Align::Center)
			.build();
		test_row.append(&test_btn);
		test_row.append(&status_label);

		let heading = ui_commons::dialog_heading(i18n("Network location"));

		let (cancel_btn, save_btn, buttons) = ui_commons::cancel_save_row();

		let content = ui_commons::dialog_content_box();
		content.append(&heading);
		content.append(&user_entry);
		content.append(&host_entry);
		content.append(&port_entry);
		content.append(&path_entry);
		content.append(&test_row);
		content.append(&buttons);

		{
			let (fs_type, test_btn, status_label) = (fs_type.clone(), test_btn.clone(), status_label.clone());
			let (host_entry, port_entry) = (host_entry.clone(), port_entry.clone());
			test_btn.clone().connect_clicked(move |_| {
				let host = host_entry.text().to_string();
				let Some(port) = resolve_test_port(&fs_type, &port_entry.text()) else {
					set_connection_status(&status_label, i18n("Enter a valid port to test.").as_str(), true);
					return;
				};
				if host.is_empty() {
					set_connection_status(&status_label, i18n("Enter a host to test.").as_str(), true);
					return;
				}
				test_btn.set_sensitive(false);
				set_connection_status(&status_label, i18n("Testing connection…").as_str(), false);
				let (tx, rx) = std::sync::mpsc::channel();
				let (test_btn, status_label) = (test_btn.clone(), status_label.clone());
				std::thread::spawn(move || {
					let result = match test_network_connection(&host, port) {
						Ok(()) => Ok(i18n_fmt(
							"Connected to {host}:{port}.",
							&[("{host}", &host), ("{port}", &port.to_string())],
						)),
						Err(err) => Err(err),
					};
					let _ = tx.send(result);
				});
				gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || match rx.try_recv() {
					Ok(Ok(message)) => {
						test_btn.set_sensitive(true);
						set_connection_status(&status_label, message.as_str(), false);
						gtk::glib::ControlFlow::Break
					}
					Ok(Err(err)) => {
						test_btn.set_sensitive(true);
						set_connection_status(&status_label, err.as_str(), true);
						gtk::glib::ControlFlow::Break
					}
					Err(std::sync::mpsc::TryRecvError::Disconnected) => {
						test_btn.set_sensitive(true);
						set_connection_status(&status_label, i18n("Test connection failed.").as_str(), true);
						gtk::glib::ControlFlow::Break
					}
					Err(_) => gtk::glib::ControlFlow::Continue,
				});
			});
		}

		let dialog = Dialog::builder().child(&content).follows_content_size(true).width_request(400).build();

		ui_commons::close_on_click(&cancel_btn, &dialog);

		{
			let (controller, user_entry, host_entry) = (self.clone(), user_entry.clone(), host_entry.clone());
			let (port_entry, path_entry, status_label) = (port_entry.clone(), path_entry.clone(), status_label.clone());
			let dialog = dialog.clone();
			save_btn.connect_clicked(move |_| {
				if controller.apply_network_fields(&user_entry, &host_entry, &port_entry, &path_entry) {
					dialog.close();
				} else {
					set_connection_status(&status_label, i18n("Enter a host to save.").as_str(), true);
				}
			});
		}

		let parent = ui_commons::parent_window(&self.picker_btn);
		dialog.present(parent.as_ref());
	}

	fn apply_network_fields(&self, user: &EntryRow, host: &EntryRow, port: &EntryRow, path: &EntryRow) -> bool {
		let host = host.text().to_string();
		if host.is_empty() {
			return false;
		}
		let user = user.text().to_string();
		let port = port.text().to_string();
		let path = path.text().to_string();
		let location = NetworkLocation {
			style: *self.style.borrow(),
			user: (!user.is_empty()).then_some(user),
			host,
			port: (!port.is_empty()).then_some(port),
			path: (!path.is_empty()).then_some(path),
		};
		self.set_device(DeviceValue::from(location.render(), DeviceKind::Network));
		true
	}

	fn open_device_picker(&self) {
		let kind = self.entry.borrow().device.kind;
		let mut devices: Vec<BlockDeviceInfo> = match list_block_devices() {
			Ok(devices) => devices.into_iter().filter(|device| pick_value(device, kind).is_some()).collect(),
			Err(err) => {
				ui_commons::present_simple_dialog(&self.picker_btn, i18n("Could not list devices").as_str(), &format!("{err:#}"));
				return;
			}
		};
		devices.sort_by(|a, b| a.name.cmp(&b.name));

		let store = gtk::gio::ListStore::new::<DeviceTableRow>();
		for device in &devices {
			store.append(&DeviceTableRow::new(device, kind));
		}

		let current = self.entry.borrow().device.value.clone();
		let selected_pos = (0..store.n_items()).find(|&i| {
			store
				.item(i)
				.and_then(|item| item.downcast::<DeviceTableRow>().ok())
				.is_some_and(|row| row.value() == current)
		});

		let query = Rc::new(RefCell::new(String::new()));
		let filter = CustomFilter::new({
			let query = query.clone();
			move |item| {
				let query = query.borrow().trim().to_lowercase();
				item.downcast_ref::<DeviceTableRow>()
					.map(|row| query.is_empty() || filter_row(row, &query))
					.unwrap_or(true)
			}
		});
		let filter_model = FilterListModel::new(Some(store), Some(filter.clone()));
		let selection = SingleSelection::new(Some(filter_model.clone()));
		selection.set_autoselect(false);
		if let Some(pos) = selected_pos {
			selection.set_selected(pos);
		}

		let column_view = ColumnView::builder()
			.model(&selection)
			.reorderable(false)
			.hexpand(true)
			.vexpand(true)
			.build();
		for (title, getter) in device_columns(kind) {
			column_view.append_column(&make_column(&title, getter));
		}

		let max_width = ui_commons::suggested_dialog_width(&self.picker_btn);

		let scroll = ScrolledWindow::builder()
			.child(&column_view)
			.max_content_height(300)
			.max_content_width(max_width)
			.propagate_natural_height(true)
			.propagate_natural_width(true)
			.hexpand(true)
			.build();

		let search = SearchEntry::builder()
			.placeholder_text(i18n("Filter devices…").as_str())
			.hexpand(true)
			.build();
		let empty_label = Label::builder()
			.label(i18n("No devices found"))
			.halign(Align::Start)
			.visible(false)
			.build();

		let heading = ui_commons::dialog_heading(i18n_fmt("Select {kind}", &[("{kind}", kind.label())]));
		let cancel_btn = Button::with_label(i18n("Cancel").as_str());
		cancel_btn.set_halign(Align::End);

		let content = ui_commons::dialog_content_box();
		content.append(&heading);
		content.append(&search);
		content.append(&scroll);
		content.append(&empty_label);
		content.append(&cancel_btn);
		if devices.is_empty() {
			empty_label.set_visible(true);
			scroll.set_visible(false);
		}

		let dialog = Dialog::builder().child(&content).follows_content_size(true).build();

		ui_commons::close_on_click(&cancel_btn, &dialog);

		{
			let (dialog, controller) = (dialog.clone(), self.clone());
			selection.connect_selection_changed(move |selection, _, _| {
				if selection.selected() == gtk::INVALID_LIST_POSITION {
					return;
				}
				let Some(item) = selection.selected_item() else {
					return;
				};
				let Ok(row) = item.downcast::<DeviceTableRow>() else {
					return;
				};
				let kind = controller.entry.borrow().device.kind;
				controller.set_device(DeviceValue::from(row.value(), kind));
				dialog.close();
			});
		}

		{
			let (query, filter, filter_model) = (query.clone(), filter.clone(), filter_model.clone());
			let (empty_label, scroll) = (empty_label.clone(), scroll.clone());
			search.connect_search_changed(move |search| {
				*query.borrow_mut() = search.text().to_string();
				filter.changed(gtk::FilterChange::Different);
				let empty = filter_model.n_items() == 0;
				empty_label.set_visible(empty);
				scroll.set_visible(!empty);
			});
		}

		if let Some(pos) = selected_pos {
			let column_view = column_view.clone();
			gtk::glib::idle_add_local_once(move || column_view.scroll_to(pos, None, ListScrollFlags::FOCUS, None));
		}
		let parent = ui_commons::parent_window(&self.picker_btn);
		dialog.present(parent.as_ref());
	}
}

mod imp {
	use glib::subclass::prelude::*;
	use std::cell::RefCell;

	#[derive(Default)]
	pub struct DeviceTableRow {
		pub value: RefCell<String>,
		pub name: RefCell<String>,
		pub size: RefCell<String>,
		pub label: RefCell<String>,
		pub fstype: RefCell<String>,
		pub mount: RefCell<String>,
		pub model: RefCell<String>,
	}

	#[glib::object_subclass]
	impl ObjectSubclass for DeviceTableRow {
		const NAME: &'static str = "DeviceTableRow";
		type Type = super::DeviceTableRow;
		type ParentType = glib::Object;
	}

	impl ObjectImpl for DeviceTableRow {}
}

glib::wrapper! {
	pub struct DeviceTableRow(ObjectSubclass<imp::DeviceTableRow>);
}

impl DeviceTableRow {
	fn new(device: &BlockDeviceInfo, kind: DeviceKind) -> Self {
		let row: DeviceTableRow = glib::Object::builder().build();
		let imp = row.imp();
		*imp.value.borrow_mut() = pick_value(device, kind).unwrap_or_default();
		*imp.name.borrow_mut() = device.name.clone();
		*imp.size.borrow_mut() = device.size.clone().unwrap_or_default();
		*imp.label.borrow_mut() = device.label.clone().unwrap_or_default();
		*imp.fstype.borrow_mut() = device.fstype.clone().unwrap_or_default();
		*imp.mount.borrow_mut() = device.mountpoints.join(",");
		*imp.model.borrow_mut() = device.model.clone().unwrap_or_default();
		row
	}

	fn value(&self) -> String {
		self.imp().value.borrow().clone()
	}

	fn name(&self) -> String {
		self.imp().name.borrow().clone()
	}

	fn size(&self) -> String {
		self.imp().size.borrow().clone()
	}

	fn label(&self) -> String {
		self.imp().label.borrow().clone()
	}

	fn fstype(&self) -> String {
		self.imp().fstype.borrow().clone()
	}

	fn mount(&self) -> String {
		self.imp().mount.borrow().clone()
	}

	fn model(&self) -> String {
		self.imp().model.borrow().clone()
	}
}

type DeviceColumn = (String, fn(&DeviceTableRow) -> String);

fn device_columns(kind: DeviceKind) -> Vec<DeviceColumn> {
	vec![
		(i18n(kind.label()), DeviceTableRow::value),
		(i18n("Device name"), DeviceTableRow::name),
		(i18n("Size"), DeviceTableRow::size),
		(i18n("Label"), DeviceTableRow::label),
		(i18n("File System"), DeviceTableRow::fstype),
		(i18n("Model"), DeviceTableRow::model),
	]
}

fn make_column(title: &str, getter: fn(&DeviceTableRow) -> String) -> ColumnViewColumn {
	let factory = SignalListItemFactory::new();
	factory.connect_setup(move |_, item| {
		let Some(list_item) = item.downcast_ref::<gtk::ListItem>() else {
			return;
		};
		let label = Label::builder().halign(Align::Start).ellipsize(gtk::pango::EllipsizeMode::End).build();
		list_item.set_child(Some(&label));
	});
	factory.connect_bind(move |_, item| {
		let Some(list_item) = item.downcast_ref::<gtk::ListItem>() else {
			return;
		};
		let Some(item) = list_item.item().and_then(|item| item.downcast::<DeviceTableRow>().ok()) else {
			return;
		};
		let Some(label) = list_item.child().and_then(|child| child.downcast::<Label>().ok()) else {
			return;
		};
		label.set_text(&getter(&item));
	});
	ColumnViewColumn::builder().title(title).factory(&factory).build()
}

fn filter_row(row: &DeviceTableRow, query: &str) -> bool {
	row.value().to_lowercase().contains(query)
		|| row.name().to_lowercase().contains(query)
		|| row.size().to_lowercase().contains(query)
		|| row.label().to_lowercase().contains(query)
		|| row.fstype().to_lowercase().contains(query)
		|| row.mount().to_lowercase().contains(query)
		|| row.model().to_lowercase().contains(query)
}

fn pick_value(device: &BlockDeviceInfo, kind: DeviceKind) -> Option<String> {
	match kind {
		DeviceKind::Uuid => device.uuid.clone(),
		DeviceKind::PartUuid => device.partuuid.clone(),
		DeviceKind::Label => device.label.clone(),
		DeviceKind::PartLabel => device.partlabel.clone(),
		DeviceKind::DevicePath => Some(device.path.clone()),
		DeviceKind::Network | DeviceKind::FilePath | DeviceKind::Other => None,
	}
}

pub fn add_device_row(options: &PreferencesGroup, entry_ctx: &EntryContext) -> DeviceRowController {
	let entry = entry_ctx.entry().clone();
	let (kinds_vec, selected) = {
		let entry = entry.borrow();
		kinds_with_selected(&entry.fs_type, entry.device.kind)
	};
	let kinds: GC<Vec<DeviceKind>> = GC::new(kinds_vec);

	let initial = &entry.borrow().device;

	let model = StringList::new(
		&kinds
			.borrow()
			.iter()
			.map(|k| i18n(k.label()))
			.collect::<Vec<_>>()
			.iter()
			.map(String::as_str)
			.collect::<Vec<_>>(),
	);
	let dropdown = DropDown::builder().model(&model).selected(selected as u32).valign(Align::Center).build();

	let syncing: GC<bool> = GC::new(false);
	let style: GC<NetworkStyle> = GC::new(network_style_for_fs(&entry.borrow().fs_type));

	let issue_icon = ui_commons::issue_image();
	let header = ui_commons::titled_header(i18n("Device").as_str(), None, Some(&issue_icon), &dropdown);

	let value_entry = Entry::builder().text(&initial.value).hexpand(true).margin_start(12).build();
	let picker_btn = Button::builder()
		.icon_name("preferences-system-symbolic")
		.margin_end(12)
		.tooltip_text(i18n("Edit device"))
		.build();

	let input_row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(8).build();
	input_row.append(&value_entry);
	input_row.append(&picker_btn);

	let warning = Label::builder()
		.halign(Align::Start)
		.wrap(true)
		.visible(false)
		.css_classes(["error"])
		.build();

	let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(0).margin_bottom(6).build();
	content.append(&header);
	content.append(&input_row);
	content.append(&warning);
	let row = PreferencesRow::builder().child(&content).activatable(false).build();

	options.add(&row);

	let controller = DeviceRowController {
		entry: entry.clone(),
		entry_ctx: entry_ctx.clone(),
		dropdown: dropdown.clone(),
		kinds,
		model,
		syncing,
		style,
		value_entry: value_entry.clone(),
		picker_btn: picker_btn.clone(),
		issue_icon: issue_icon.clone(),
	};

	{
		let (controller, warning) = (controller.clone(), warning.clone());
		value_entry.connect_changed(move |entry| {
			if *controller.syncing.borrow() {
				return;
			}
			let Some(&kind) = controller.kinds.borrow().get(controller.dropdown.selected() as usize) else {
				return;
			};
			warning.set_visible(false);
			controller.entry.borrow_mut().device = DeviceValue::from(entry.text(), kind);
			controller.refresh_issue();
			controller.entry_ctx.render();
		});
	}
	{
		let controller = controller.clone();
		picker_btn.connect_clicked(move |_| match controller.entry.borrow().device.kind {
			DeviceKind::Network => controller.open_network_editor(),
			kind if DeviceKind::LOCAL.contains(&kind) => controller.open_device_picker(),
			_ => {}
		});
	}
	{
		let (controller, warning) = (controller.clone(), warning.clone());
		dropdown.connect_selected_notify(move |dropdown| {
			let Some(&new_kind) = controller.kinds.borrow().get(dropdown.selected() as usize) else {
				return;
			};
			let current = controller.entry.cloned(|e| &e.device);

			if new_kind == current.kind || new_kind == DeviceKind::Other {
				controller.sync_kind();
				controller.entry_ctx.render();
				return;
			}

			if new_kind == DeviceKind::Network {
				controller.set_device(DeviceValue::from(current.value, DeviceKind::Network));
				return;
			}

			let both_local = current.kind.is_local() && new_kind.is_local();
			if both_local {
				match current.transform(new_kind) {
					Some(device) => {
						controller.set_device(device);
						warning.set_visible(false);
					}
					None => {
						controller.set_device(DeviceValue::from(current.value.clone(), new_kind));
						warning.set_label(&i18n_fmt(
							"Could not resolve a {kind} for {value}. The value was kept as-is.",
							&[("{kind}", new_kind.label()), ("{value}", &current.value)],
						));
						warning.set_visible(true);
					}
				}
			} else {
				controller.set_device(DeviceValue::new(current.value, new_kind));
			}
		});
	}

	controller.sync_kind();

	controller
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::context::FileContext;
	use crate::stab_yurself::StabFile;
	use adw::ActionRow;

	fn skip_if_no_display() -> bool {
		let opened = gtk::gdk::Display::default().is_some() || gtk::gdk::Display::open(None).is_some();
		if !opened {
			eprintln!("skipping UI test: no display available");
		}
		!opened
	}

	fn device_row(raw: &str) -> (DeviceRowController, GC<StabEntry>, PreferencesGroup) {
		let entry = GC::new(StabEntry::from(0, raw).unwrap());
		let file_ctx = FileContext::new(GC::new(StabFile::empty()), Rc::new(|| {}));
		let list_row = ActionRow::builder().build();
		let entry_ctx = file_ctx.entry(entry.clone(), &list_row);
		let group = PreferencesGroup::builder().build();
		let controller = add_device_row(&group, &entry_ctx);
		(controller, entry, group)
	}

	#[gtk::test]
	fn fs_type_change_reclassifies_device_in_ui() {
		if skip_if_no_display() {
			return;
		}
		let (controller, entry, _group) = device_row("UUID=abc /mnt/data ext4 defaults 0 2");
		assert_eq!(controller.dropdown.selected(), 0);
		assert_eq!(controller.value_entry.text().to_string(), "abc");

		entry.borrow_mut().set_fs_type(FsType::Tmpfs);
		controller.refresh_kinds();
		assert_eq!(controller.dropdown.selected(), 0);
		assert_eq!(controller.model.n_items(), 1);
		assert_eq!(controller.value_entry.text().to_string(), "UUID=abc");

		let (controller, entry, _group) = device_row("//server/share /mnt/share ext4 defaults 0 0");
		assert_eq!(controller.dropdown.selected(), 5);
		assert_eq!(controller.value_entry.text().to_string(), "//server/share");

		entry.borrow_mut().set_fs_type(FsType::Cifs);
		controller.refresh_kinds();
		assert_eq!(controller.dropdown.selected(), 0);
		assert_eq!(controller.value_entry.text().to_string(), "//server/share");
	}

	#[gtk::test]
	fn swap_device_row_preselects_by_kind() {
		if skip_if_no_display() {
			return;
		}
		let (controller, _, _group) = device_row("/dev/zram0 none swap defaults 0 0");
		assert_eq!(controller.dropdown.selected(), 4);
		assert_eq!(controller.value_entry.text().to_string(), "/dev/zram0");

		let (controller, _, _group) = device_row("/swapfile none swap defaults 0 0");
		assert_eq!(controller.dropdown.selected(), 5);
		assert_eq!(controller.value_entry.text().to_string(), "/swapfile");
	}

	#[gtk::test]
	fn dropdown_switch_transforms_uuid_to_device_path() {
		if skip_if_no_display() {
			return;
		}
		let dir = match std::fs::read_dir("/dev/disk/by-uuid") {
			Ok(dir) => dir,
			Err(_) => {
				eprintln!("skipping UI test: /dev/disk/by-uuid unavailable");
				return;
			}
		};
		let Some(entry) = dir.filter_map(Result::ok).next() else {
			eprintln!("skipping UI test: no uuid entries");
			return;
		};
		let uuid = entry.file_name().to_string_lossy().into_owned();

		let (controller, _, _group) = device_row(&format!("UUID={uuid} /mnt/data ext4 defaults 0 2"));
		assert_eq!(controller.dropdown.selected(), 0);

		controller.dropdown.set_selected(4);
		let path = controller.value_entry.text().to_string();
		assert!(path.starts_with("/dev/"), "expected a /dev path, got {path}");

		controller.dropdown.set_selected(0);
		assert_eq!(controller.value_entry.text().to_string(), uuid);
	}

	#[test]
	fn reclassify_for_fs() {
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
	fn reclassify_empty_to_network() {
		let blank = DeviceValue::from("", DeviceKind::Other);
		let re = blank.reclassify_for(&FsType::Cifs);
		assert_eq!(re.kind, DeviceKind::Network);
		assert_eq!(re.value, "");

		let re = blank.reclassify_for(&FsType::Ext4);
		assert_eq!(re.kind, DeviceKind::Other);
	}

	#[test]
	fn swap_devices_classify_by_kind() {
		let allowed = DeviceKind::for_fs_type(&FsType::Swap);
		let file = DeviceKind::classify("/swapfile", allowed);
		assert_eq!(file.kind, DeviceKind::FilePath);
		assert_eq!(file.render(), "/swapfile");

		assert_eq!(DeviceKind::classify("/dev/zram0", allowed).kind, DeviceKind::DevicePath);
		assert_eq!(DeviceKind::FilePath.value_of("/dev/sda1"), None);
		assert_eq!(
			DeviceKind::classify("UUID=77777777-7777-7777-7777-777777777777", allowed).kind,
			DeviceKind::Uuid
		);
	}

	#[test]
	fn file_path_resolves_and_transforms() {
		let dir = std::env::temp_dir().join(format!("fstabulator-fp-test-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let file = dir.join("swapfile");
		std::fs::write(&file, "").unwrap();
		let path = file.canonicalize().unwrap();

		let value = DeviceValue::from(path.to_string_lossy().into_owned(), DeviceKind::FilePath);
		assert_eq!(value.resolve_node().as_deref(), Some(path.as_path()));
		let as_device = value.transform(DeviceKind::DevicePath).unwrap();
		assert_eq!(as_device.value, path.to_string_lossy().into_owned());
		let _ = std::fs::remove_dir_all(&dir);
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

	#[test]
	fn parse_smb_locations() {
		let loc = NetworkLocation::parse("//server/share").unwrap();
		assert_eq!(loc.style, NetworkStyle::Smb);
		assert_eq!(loc.user, None);
		assert_eq!(loc.host, "server");
		assert_eq!(loc.port, None);
		assert_eq!(loc.path.as_deref(), Some("share"));
		assert_eq!(loc.render(), "//server/share");
	}

	#[test]
	fn parse_smb_user_port_and_ipv6() {
		let loc = NetworkLocation::parse("//user@server:445/share").unwrap();
		assert_eq!(loc.style, NetworkStyle::Smb);
		assert_eq!(loc.user.as_deref(), Some("user"));
		assert_eq!(loc.host, "server");
		assert_eq!(loc.port.as_deref(), Some("445"));
		assert_eq!(loc.path.as_deref(), Some("share"));
		assert_eq!(loc.render(), "//user@server:445/share");

		let loc = NetworkLocation::parse("//[2001:db8::1]/share").unwrap();
		assert_eq!(loc.host, "2001:db8::1");
		assert_eq!(loc.render(), "//[2001:db8::1]/share");

		let loc = NetworkLocation::parse("//[::1]:445/share").unwrap();
		assert_eq!(loc.host, "::1");
		assert_eq!(loc.port.as_deref(), Some("445"));
		assert_eq!(loc.render(), "//[::1]:445/share");
	}

	#[test]
	fn parse_host_path_locations() {
		let loc = NetworkLocation::parse("server:/export/path").unwrap();
		assert_eq!(loc.style, NetworkStyle::HostPath);
		assert_eq!(loc.host, "server");
		assert_eq!(loc.path.as_deref(), Some("/export/path"));
		assert_eq!(loc.render(), "server:/export/path");

		let loc = NetworkLocation::parse("user@host:/path").unwrap();
		assert_eq!(loc.user.as_deref(), Some("user"));
		assert_eq!(loc.host, "host");
		assert_eq!(loc.path.as_deref(), Some("/path"));

		let loc = NetworkLocation::parse("[2001:db8::1]:/export").unwrap();
		assert_eq!(loc.host, "2001:db8::1");
		assert_eq!(loc.path.as_deref(), Some("/export"));
		assert_eq!(loc.render(), "[2001:db8::1]:/export");

		let loc = NetworkLocation::parse("host:").unwrap();
		assert_eq!(loc.host, "host");
		assert_eq!(loc.path.as_deref(), Some(""));
		assert_eq!(loc.render(), "host:");
	}

	#[test]
	fn parse_round_trips() {
		for raw in [
			"//server/share",
			"//user@server/share",
			"//server:445/share",
			"//user@server:445/share",
			"//[::1]/share",
			"//[::1]:445/share",
			"server:/export/path",
			"user@host:/path",
			"[2001:db8::1]:/export",
			"host:",
		] {
			let Some(loc) = NetworkLocation::parse(raw) else {
				panic!("failed to parse {raw}");
			};
			assert_eq!(loc.render(), raw);
		}
	}

	#[test]
	fn parse_rejects() {
		assert_eq!(NetworkLocation::parse(""), None);
		assert_eq!(NetworkLocation::parse("server"), None);
		assert_eq!(NetworkLocation::parse("/dev/sda1"), None);
		assert_eq!(NetworkLocation::parse("UUID=abc"), None);
		assert_eq!(NetworkLocation::parse("//"), None);
	}
}
