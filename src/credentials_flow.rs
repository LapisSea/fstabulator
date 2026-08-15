use crate::fs_value::FsType;
use crate::popup;
use crate::privileged::{CredentialsInfo, MountCredentials, inspect_credentials_file, saved_credentials_path};
use crate::stab_yurself::StabEntry;
use crate::{GC, RebuildEditor};
use adw::prelude::*;
use adw::{AlertDialog, EntryRow, PasswordEntryRow, SwitchRow};
use gtk::{Button, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

enum CredentialsOutcome {
	NotSaved,
	SavedNew(PathBuf),
	Modified(PathBuf),
	UsedExisting(PathBuf),
}

pub fn needs_credentials(entry: &StabEntry) -> bool {
	entry.fs_type.is_network() && !(entry.has_option("password") || entry.has_option("credentials") || entry.has_option("guest"))
}

pub fn action_device(entry: &StabEntry) -> String {
	entry
		.device
		.resolve_node()
		.map(|path| path.to_string_lossy().into_owned())
		.unwrap_or_else(|| entry.device.render())
}

fn option_value(options: &[String], key: &str) -> Option<String> {
	options.iter().find_map(|option| {
		let mut parts = option.splitn(2, '=');
		if parts.next() == Some(key) {
			parts.next().map(str::to_string)
		} else {
			None
		}
	})
}

fn set_entry_option(entry: &mut StabEntry, key: &str, value: &str) {
	let full = format!("{key}={value}");
	if let Some(pos) = entry.options.iter().position(|option| option.split('=').next() == Some(key)) {
		entry.options[pos] = full;
	} else {
		entry.options.push(full);
	}
}

fn default_credentials_filename(entry: &StabEntry) -> String {
	let device = action_device(entry);
	let address = match &entry.fs_type {
		FsType::Cifs | FsType::Smb3 => device
			.strip_prefix("//")
			.and_then(|s| s.split('/').next())
			.filter(|s| !s.is_empty())
			.unwrap_or(&device),
		FsType::FuseSshfs => device
			.rsplit_once('@')
			.map(|(_, host)| host.split(':').next().unwrap_or(""))
			.filter(|host| !host.is_empty())
			.unwrap_or(&device),
		_ => &device,
	};
	let ext = match &entry.fs_type {
		FsType::FuseSshfs => "sshfs",
		_ => "cifs",
	};
	let name: String = address
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
				c
			} else {
				'_'
			}
		})
		.collect();
	let name = name.trim_matches('.').to_string();
	let name = if name.is_empty() { "credentials" } else { &name };
	format!("{name}.{ext}")
}

pub fn mount_with_credentials(btn: &Button, entry: GC<StabEntry>, snapshot: StabEntry, rebuild_editor: RebuildEditor, refresh: Rc<dyn Fn()>) {
	let device = action_device(&snapshot);
	let is_swap = snapshot.fs_type == FsType::Swap;
	let fs_type = snapshot.fs_type.to_string();
	let btn = btn.clone();
	let btn_ref = btn.clone();
	let mount_point = snapshot.mount_point.clone();
	let username = option_value(&snapshot.options, "username");
	let domain = option_value(&snapshot.options, "domain");
	let can_save = matches!(&snapshot.fs_type, FsType::Cifs | FsType::Smb3);
	let default_filename = default_credentials_filename(&snapshot);
	CredentialsDialog::new(
		&btn_ref,
		&format!("Mount {}", mount_point),
		username.as_deref(),
		domain.as_deref(),
		&default_filename,
		can_save,
		|filename| inspect_credentials_file(filename).ok(),
		move |credentials| {
			let Some(credentials) = credentials else {
				return;
			};
			if credentials.password.is_empty() {
				popup::present_simple_dialog(&btn, "Cannot mount", "A password is required.");
				return;
			}
			if credentials.save && credentials.filename.trim().is_empty() {
				popup::present_simple_dialog(&btn, "Cannot mount", "A credentials file name is required when saving credentials.");
				return;
			}
			let filename = credentials.save.then(|| credentials.filename.trim().to_string());
			let username =
				(!credentials.username.is_empty() && option_value(&snapshot.options, "username").is_none()).then_some(credentials.username);
			let domain = (!credentials.domain.is_empty() && option_value(&snapshot.options, "domain").is_none()).then_some(credentials.domain);
			let mount_credentials = MountCredentials {
				username,
				password: credentials.password,
				domain,
				filename,
			};
			let saved_path = mount_credentials.filename.as_ref().map(|f| saved_credentials_path(f));
			let outcome = match &mount_credentials.filename {
				None => CredentialsOutcome::NotSaved,
				Some(filename) => {
					let path = saved_credentials_path(filename);
					let existing = inspect_credentials_file(filename).ok().filter(|info| info.exists);
					let unchanged = existing.as_ref().is_some_and(|info| {
						info.username.as_deref() == mount_credentials.username.as_deref()
							&& info.password == mount_credentials.password
							&& info.domain.as_deref() == mount_credentials.domain.as_deref()
					});
					if unchanged {
						CredentialsOutcome::UsedExisting(path)
					} else if existing.is_some() {
						CredentialsOutcome::Modified(path)
					} else {
						CredentialsOutcome::SavedNew(path)
					}
				}
			};
			match crate::privileged::mount(&snapshot.mount_point, &device, is_swap, &fs_type, Some(mount_credentials)) {
				Ok(()) => {
					if let Some(path) = &saved_path {
						{
							let mut entry = entry.borrow_mut();
							set_entry_option(&mut entry, "credentials", &path.display().to_string());
						}
						if let Some(rebuild) = rebuild_editor.borrow().clone() {
							rebuild();
						}
					}
					present_mount_success(&btn, &snapshot.mount_point, &outcome);
					refresh();
				}
				Err(err) => {
					let Some(path) = saved_path else {
						popup::present_simple_dialog(&btn, "Could not mount", &format!("{err:#}"));
						return;
					};
					let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
					let created_this_attempt = matches!(outcome, CredentialsOutcome::SavedNew(_))
						&& inspect_credentials_file(&filename).map(|info| info.exists).unwrap_or(false);
					if !created_this_attempt {
						popup::present_simple_dialog(&btn, "Could not mount", &format!("{err:#}"));
						return;
					}
					popup::confirm_popup(
						&btn,
						"Delete credentials",
						&format!("{err:#}\n\nWould you like to delete the saved credentials file {}?", path.display()),
						None::<&gtk::Widget>,
						{
							let btn = btn.clone();
							move || match crate::privileged::delete_credentials_file(&filename) {
								Ok(()) => popup::present_simple_dialog(&btn, "Credentials deleted", "The saved credentials file was deleted."),
								Err(delete_err) => popup::present_simple_dialog(&btn, "Could not delete credentials", &format!("{delete_err:#}")),
							}
						},
					);
				}
			}
		},
	);
}

fn present_mount_success(btn: &Button, mount_point: &str, outcome: &CredentialsOutcome) {
	let mut bullets = Vec::new();
	match outcome {
		CredentialsOutcome::NotSaved => bullets.push("login used for this mount only".to_string()),
		CredentialsOutcome::SavedNew(path) => {
			bullets.push(format!("login saved to {}", glib::markup_escape_text(&path.display().to_string())));
			bullets.push("linked login to entry, <b>you must save changes to preserve this</b>".to_string());
		}
		CredentialsOutcome::Modified(path) => {
			bullets.push(format!(
				"updated existing login in {}",
				glib::markup_escape_text(&path.display().to_string())
			));
			bullets.push("linked login to entry, <b>you must save changes to preserve this</b>".to_string());
		}
		CredentialsOutcome::UsedExisting(path) => {
			bullets.push(format!(
				"using existing login from {}",
				glib::markup_escape_text(&path.display().to_string())
			));
			bullets.push("linked login to entry, <b>you must save changes to preserve this</b>".to_string());
		}
	}
	popup::present_bullet_dialog(btn, "Mounted", &format!("Mounted {mount_point}."), &bullets);
}

struct CredentialsInput {
	pub username: String,
	pub password: String,
	pub domain: String,
	pub save: bool,
	pub filename: String,
}

type CredentialsChecker = Box<dyn Fn(&str) -> Option<CredentialsInfo>>;
type CredentialsSubmit = Box<dyn FnOnce(Option<CredentialsInput>)>;

struct CredentialsDialog {
	username_row: EntryRow,
	password_row: PasswordEntryRow,
	domain_row: EntryRow,
	save_switch: SwitchRow,
	filename_row: EntryRow,
	load_button: Button,
	warning_label: gtk::Label,
	can_save: bool,
	check_existing: CredentialsChecker,
	info: RefCell<Option<CredentialsInfo>>,
	pending: RefCell<Option<glib::SourceId>>,
	on_submit: RefCell<Option<CredentialsSubmit>>,
}

impl CredentialsDialog {
	#[allow(clippy::too_many_arguments)]
	fn new(
		parent: &impl IsA<gtk::Widget>,
		heading: &str,
		prefill_username: Option<&str>,
		prefill_domain: Option<&str>,
		default_filename: &str,
		can_save: bool,
		check_existing: impl Fn(&str) -> Option<CredentialsInfo> + 'static,
		on_submit: impl FnOnce(Option<CredentialsInput>) + 'static,
	) -> Rc<Self> {
		let username_row = EntryRow::builder().title("Username").text(prefill_username.unwrap_or("")).build();
		let password_row = PasswordEntryRow::builder().title("Password").build();
		let domain_row = EntryRow::builder().title("Domain (optional)").text(prefill_domain.unwrap_or("")).build();

		let extra = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
		extra.append(&username_row);
		extra.append(&password_row);
		extra.append(&domain_row);

		let save_switch = SwitchRow::builder()
			.title("Save for auto-mount")
			.subtitle("Writes a credentials file referenced from the fstab entry.")
			.active(true)
			.build();
		let filename_row = EntryRow::builder().title("Credentials file").text(default_filename).build();
		let load_button = Button::builder()
			.label("Load existing credentials")
			.has_frame(false)
			.visible(false)
			.build();
		let warning_label = gtk::Label::builder()
			.label("Will overwrite the existing file!")
			.wrap(true)
			.halign(gtk::Align::Start)
			.visible(false)
			.build();

		if can_save {
			let save_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6).build();
			save_box.append(&save_switch);
			save_box.append(&filename_row);
			save_box.append(&load_button);
			save_box.append(&warning_label);
			extra.append(&save_box);
		}

		let this = Rc::new(Self {
			username_row,
			password_row,
			domain_row,
			save_switch,
			filename_row,
			load_button,
			warning_label,
			can_save,
			check_existing: Box::new(check_existing),
			info: RefCell::new(None),
			pending: RefCell::new(None),
			on_submit: RefCell::new(Some(Box::new(on_submit))),
		});

		this.save_switch.connect_active_notify({
			let this = Rc::clone(&this);
			move |row| {
				this.filename_row.set_sensitive(row.is_active());
				this.schedule_check();
			}
		});
		this.filename_row.connect_changed({
			let this = Rc::clone(&this);
			move |_| this.schedule_check()
		});
		this.username_row.connect_changed({
			let this = Rc::clone(&this);
			move |_| this.update_ui()
		});
		this.password_row.connect_changed({
			let this = Rc::clone(&this);
			move |_| this.update_ui()
		});
		this.domain_row.connect_changed({
			let this = Rc::clone(&this);
			move |_| this.update_ui()
		});
		this.load_button.connect_clicked({
			let this = Rc::clone(&this);
			move |_| {
				let Some(stored) = this.info.borrow().clone().filter(|stored| stored.exists) else {
					return;
				};
				this.username_row.set_text(stored.username.as_deref().unwrap_or(""));
				this.password_row.set_text(&stored.password);
				this.domain_row.set_text(stored.domain.as_deref().unwrap_or(""));
			}
		});

		let dialog = AlertDialog::builder()
			.heading(heading)
			.body("This filesystem may require credentials to mount.")
			.build();
		dialog.set_extra_child(Some(&extra));
		dialog.add_response("cancel", "Cancel");
		dialog.add_response("connect", "Connect");
		dialog.set_default_response(Some("connect"));
		dialog.set_close_response("cancel");

		dialog.connect_response(None, {
			let this = Rc::clone(&this);
			move |_, response| {
				if let Some(source) = this.pending.borrow_mut().take() {
					source.remove();
				}
				let Some(on_submit) = this.on_submit.borrow_mut().take() else {
					return;
				};
				if response == "connect" {
					on_submit(Some(CredentialsInput {
						username: this.username_row.text().to_string(),
						password: this.password_row.text().to_string(),
						domain: this.domain_row.text().to_string(),
						save: this.can_save && this.save_switch.is_active(),
						filename: this.filename_row.text().to_string(),
					}));
				} else {
					on_submit(None);
				}
			}
		});
		dialog.present(popup::parent_window(parent).as_ref());
		this
	}

	fn update_ui(&self) {
		let stored = self.info.borrow();
		let Some(stored) = stored.as_ref().filter(|stored| stored.exists) else {
			self.load_button.set_visible(false);
			self.warning_label.set_visible(false);
			return;
		};
		self.load_button.set_visible(true);
		let username = self.username_row.text();
		let password = self.password_row.text();
		let domain = self.domain_row.text();
		let different =
			username != stored.username.as_deref().unwrap_or("") || password != stored.password || domain != stored.domain.as_deref().unwrap_or("");
		self.warning_label.set_visible(different);
	}

	fn schedule_check(self: &Rc<Self>) {
		if let Some(source) = self.pending.borrow_mut().take() {
			source.remove();
		}
		if !self.save_switch.is_active() {
			*self.info.borrow_mut() = None;
			self.update_ui();
			return;
		}
		let filename = self.filename_row.text().to_string();
		if filename.trim().is_empty() {
			*self.info.borrow_mut() = None;
			self.update_ui();
			return;
		}
		let source = glib::timeout_add_local_once(Duration::from_millis(200), {
			let this = Rc::clone(self);
			move || {
				let info = (this.check_existing)(&filename);
				*this.info.borrow_mut() = info;
				*this.pending.borrow_mut() = None;
				this.update_ui();
			}
		});
		*self.pending.borrow_mut() = Some(source);
	}
}
