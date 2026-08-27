mod block_devices;
mod context;
mod credentials_flow;
mod device_value;
mod entry_text_edit;
mod fs_options;
mod fs_value;
mod i18n;
mod mount_point_value;
mod mount_status;
mod options_value;
mod privileged;
mod problem_reports;
mod right_panel_editor;
mod search_picker;
mod stab_yurself;
mod subvolume;
mod ui_commons;

use crate::context::FileContext;
use crate::i18n::{i18n, i18n_fmt, localized_datetime};
use crate::right_panel_editor::build_editor_panel;
use crate::search_picker::SearchPickerBuilder;
use crate::stab_yurself::{StabEntry, StabFile};
use crate::ui_commons::{ERROR_NAME, WARNING_NAME, activatable_row, clear_children, find_widget_with_class, query_matches, trash_button};
use adw::gdk::pango;
use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, HeaderBar, LengthUnit, Toast, ToastOverlay};
use anyhow::Context as _;
use gtk::{Align, Box as GtkBox, Button, Image, ListBox, MenuButton, Orientation, ScrolledWindow, SelectionMode, Widget};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::SystemTime;

pub(crate) struct GC<T>(Rc<RwLock<T>>);

impl<T> Clone for GC<T> {
	fn clone(&self) -> Self {
		Self(Rc::clone(&self.0))
	}
}

impl<T> GC<T> {
	pub(crate) fn new(value: T) -> Self {
		Self(Rc::new(RwLock::new(value)))
	}

	pub(crate) fn borrow(&self) -> RwLockReadGuard<'_, T> {
		self.0.read().unwrap_or_else(PoisonError::into_inner)
	}

	pub(crate) fn cloned<F, V: Clone>(&self, get: F) -> V
	where
		F: FnOnce(&T) -> &V,
	{
		let val = self.0.read().unwrap_or_else(PoisonError::into_inner);
		let ret = get(&val);
		ret.clone()
	}

	pub(crate) fn borrow_mut(&self) -> RwLockWriteGuard<'_, T> {
		self.0.write().unwrap_or_else(PoisonError::into_inner)
	}
}

pub(crate) type RebuildEditor = GC<Option<Rc<dyn Fn()>>>;

const APP_ID: &str = "org.lapissea.FSTabulator";
const WINDOW_TITLE: &str = "FSTabulator";
const WINDOW_TITLE_MODIFIED: &str = "FSTabulator •";

fn register_icon() -> anyhow::Result<()> {
	gtk::gio::resources_register_include!("compiled.gresource").context("Failed to register app resources")?;
	if let Some(display) = gtk::gdk::Display::default() {
		gtk::IconTheme::for_display(&display).add_resource_path("/org/lapissea/FSTabulator/icons");
	}
	gtk::Window::set_default_icon_name("fstabulator");
	Ok(())
}

fn main() -> gtk::glib::ExitCode {
	i18n::init();
	if std::env::args().any(|arg| arg == "--root-helper") {
		if let Err(err) = privileged::run_root_helper() {
			eprintln!("root-helper error: {err:#}");
			std::process::exit(1);
		}
		return gtk::glib::ExitCode::SUCCESS;
	}

	let application = Application::builder().application_id(APP_ID).build();
	application.connect_activate(build_ui);
	application.run()
}

fn build_ui(application: &Application) {
	if let Err(err) = register_icon() {
		eprintln!("{err:#}");
		return;
	}

	let window_build = ApplicationWindow::builder()
		.application(application)
		.title(WINDOW_TITLE)
		.default_width(800)
		.default_height(600);

	let stab_file = GC::new(StabFile::empty());

	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();

	let list_panel = ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
		.valign(Align::Start)
		.build();

	let file_buttons_panel = GtkBox::builder().orientation(Orientation::Vertical).hexpand(true).spacing(6).build();

	let row = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.hexpand(true)
		.spacing(12)
		.homogeneous(true)
		.build();
	file_buttons_panel.append(&row);

	let make_backup_btn = make_icon_label_button("document-save-as-symbolic", i18n("Make backup").as_str());
	let save_changes_btn = make_icon_label_button("document-save-symbolic", i18n("Save changes").as_str());
	let revert_changes_btn = make_icon_label_button("edit-undo-symbolic", i18n("Revert changes").as_str());
	save_changes_btn.add_css_class("suggested-action");
	revert_changes_btn.add_css_class("destructive-action");

	let title_window: GC<Option<ApplicationWindow>> = GC::new(None);
	let file_ctx = FileContext::new(
		stab_file.clone(),
		Rc::new({
			let (stab_file, save_changes_btn, revert_changes_btn) = (stab_file.clone(), save_changes_btn.clone(), revert_changes_btn.clone());
			let title_window = title_window.clone();
			move || {
				let changed = stab_file.borrow().is_changed();
				save_changes_btn.set_sensitive(changed);
				revert_changes_btn.set_sensitive(changed);
				if let Some(window) = title_window.borrow().as_ref() {
					window.set_title(Some(if changed { WINDOW_TITLE_MODIFIED } else { WINDOW_TITLE }));
				}
			}
		}),
	);

	{
		let stab_file = stab_file.clone();
		make_backup_btn.connect_clicked(move |btn| {
			if !stab_file.borrow().is_changed() {
				perform_make_backup(btn);
				return;
			}
			let (btn, parent) = (btn.clone(), btn.clone());
			ui_commons::confirm_popup(
				&parent,
				i18n("Your changes have not been saved yet. The backup will reflect the saved /etc/fstab, not your unsaved changes. Continue?"),
				move || perform_make_backup(&btn),
			)
			.confirm_choice(i18n("Make backup"))
			.present();
		});
	}
	row.append(&make_backup_btn);

	let toast_overlay = ToastOverlay::new();
	{
		let (file_ctx, list_panel, editor_panel) = (file_ctx.clone(), list_panel.clone(), editor_panel.clone());
		let toast_overlay = toast_overlay.clone();
		ui_commons::confirm_clicked_action(&save_changes_btn, i18n("Are you sure you want to write these changes to /etc/fstab?"))
			.confirm_choice(i18n("Save"))
			.connect(move || {
				let content = {
					let file = file_ctx.file().borrow();
					file.to_string()
				};
				match privileged::write_fstab(&content) {
					Ok(()) => {
						if let Err(err) = load_fstab_file(Path::new("/etc/fstab"), &file_ctx, &list_panel, &editor_panel) {
							ui_commons::present_simple_dialog(&editor_panel, i18n("Saved, but could not reload").as_str(), &format!("{err:#}"));
							return;
						}
						toast_overlay.add_toast(Toast::new(i18n("Saved to /etc/fstab").as_str()));
					}
					Err(err) => ui_commons::present_simple_dialog(&editor_panel, i18n("Could not save").as_str(), &format!("{err:#}")),
				}
			});
	}

	{
		let (file_ctx, list_panel, editor_panel) = (file_ctx.clone(), list_panel.clone(), editor_panel.clone());
		ui_commons::confirm_clicked_action(&revert_changes_btn, i18n("Are you sure? Any changes made will be lost!"))
			.confirm_choice(i18n("Revert"))
			.connect(move || load_backup(Path::new("/etc/fstab"), &file_ctx, &list_panel, &editor_panel));
	}

	row.append(&build_restore_picker(&file_ctx, &list_panel, &editor_panel));

	let row = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.hexpand(true)
		.spacing(12)
		.homogeneous(true)
		.build();
	file_buttons_panel.append(&row);
	row.append(&save_changes_btn);
	row.append(&revert_changes_btn);

	let left_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();
	left_panel.append(
		&gtk::Label::builder()
			.label(i18n("'/etc/fstab' entries:"))
			.margin_start(20)
			.halign(Align::Start)
			.build(),
	);

	left_panel.append(&wrap_scroll(&list_panel));
	left_panel.append(&file_buttons_panel);

	let split_box = build_split_layout(&left_panel, &wrap_scroll(&editor_panel));

	let rebuild_editor: RebuildEditor = GC::new(None);
	{
		let (editor_panel, list_panel_cb, stab_file) = (editor_panel.clone(), list_panel.clone(), stab_file.clone());
		let (file_ctx, rebuild_editor) = (file_ctx.clone(), rebuild_editor.clone());
		list_panel.connect_row_selected(move |_, row| {
			clear_children(&editor_panel);
			let Some(row) = row else { return };
			if row.index() < 0 {
				return;
			}
			let Some(entry) = stab_file.borrow().entry_at(row.index() as usize).cloned() else {
				return;
			};
			let Ok(action_row) = row.clone().downcast::<ActionRow>() else { return };
			let entry_ctx = file_ctx.entry(entry, &action_row);
			build_editor_panel(&editor_panel, &entry_ctx, &list_panel_cb, row, rebuild_editor.clone());
			let builder: Rc<dyn Fn()> = Rc::new({
				let (editor_panel, list_panel_cb, entry_ctx) = (editor_panel.clone(), list_panel_cb.clone(), entry_ctx.clone());
				let (row, rebuild_editor) = (row.clone(), rebuild_editor.clone());
				move || {
					clear_children(&editor_panel);
					build_editor_panel(&editor_panel, &entry_ctx, &list_panel_cb, &row, rebuild_editor.clone());
				}
			});
			*rebuild_editor.borrow_mut() = Some(builder);
		});
	}

	let content_box = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.hexpand(true)
		.vexpand(true)
		.margin_start(10)
		.margin_end(10)
		.margin_top(10)
		.margin_bottom(10)
		.build();
	content_box.append(&split_box);

	let content_scroll = wrap_scroll(&content_box);

	let main_box = GtkBox::builder().orientation(Orientation::Vertical).build();
	main_box.append(&HeaderBar::new());
	main_box.append(&content_scroll);

	toast_overlay.set_child(Some(&main_box));
	let window = window_build.content(&toast_overlay).build();
	*title_window.borrow_mut() = Some(window.clone());

	let exiting = GC::new(false);
	{
		let (stab_file, application, exiting) = (stab_file.clone(), application.clone(), exiting.clone());
		window.connect_close_request(move |window| {
			if *exiting.borrow() || !stab_file.borrow().is_changed() {
				return gtk::glib::Propagation::Proceed;
			}
			let (window, application, exiting) = (window.clone(), application.clone(), exiting.clone());
			ui_commons::confirm_popup(&window, i18n("You have unsaved changes. They will be lost if you exit now."), move || {
				*exiting.borrow_mut() = true;
				application.quit();
			})
			.confirm_choice(i18n("Exit"))
			.present();
			gtk::glib::Propagation::Stop
		});
	}

	attach_responsive_breakpoint(&window, &split_box);

	let provider = gtk::CssProvider::new();
	provider.load_from_string(
		".invalid-alert { color: red; }\
		.mount-status-mounted { color: @success_color; }\
		.mount-status-unmounted { color: @warning_color; }\
		.mount-status-missing { color: @error_color; }\
		.connection-ok { color: @success_color; }\
		.text-edit-ok { color: @success_color; }\
		.text-edit-warning { color: @warning_color; }\
		.issue-error { color: @error_color; }",
	);
	gtk::style_context_add_provider_for_display(&RootExt::display(&window), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

	if let Err(err) = load_fstab_file(Path::new("/etc/fstab"), &file_ctx, &list_panel, &editor_panel) {
		let error_box = GtkBox::builder().orientation(Orientation::Vertical).build();
		error_box.append(&HeaderBar::new());
		build_load_error(&error_box, err);
		window.set_content(Some(&error_box));
	}

	window.present();
}

fn build_load_error(main_box: &GtkBox, err: anyhow::Error) {
	let label = gtk::Label::new(Some(i18n("Error loading fstab file!").as_str()));
	label.set_margin_top(16);
	label.set_margin_bottom(8);
	label.add_css_class("error");

	let text = gtk::Label::new(Some(&format!("{:?}", err)));
	text.set_selectable(true);
	text.set_wrap(true);
	text.set_wrap_mode(pango::WrapMode::Char);
	text.set_margin_top(8);
	text.set_margin_bottom(16);
	text.set_margin_start(16);
	text.set_margin_end(16);

	main_box.append(&label);
	main_box.append(&wrap_scroll(&text));
}

fn perform_make_backup(btn: &Button) {
	match privileged::make_backup() {
		Ok(()) => ui_commons::present_simple_dialog(btn, i18n("Backup created").as_str(), i18n("A backup of /etc/fstab was created.").as_str()),
		Err(err) => ui_commons::present_simple_dialog(btn, i18n("Could not create backup").as_str(), &format!("{err:#}")),
	}
}

fn make_icon_label_button(icon: &str, label: &str) -> Button {
	let hbox = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.halign(Align::Center)
		.spacing(6)
		.build();
	hbox.append(&Image::from_icon_name(icon));
	let text = gtk::Label::builder().label(label).wrap(true).hexpand(true).build();
	hbox.append(&text);
	let button = Button::new();
	button.set_child(Some(&hbox));
	button.set_hexpand(true);
	button
}

fn wrap_scroll(content: &impl IsA<Widget>) -> ScrolledWindow {
	ScrolledWindow::builder().child(content).hexpand(true).vexpand(true).build()
}

pub(crate) fn render_list_entry(action_row: &ActionRow, entry: &StabEntry, reset_btn: Option<&Button>) {
	match &entry.user_label {
		Some(label) => action_row.set_title(label),
		None => action_row.set_title(i18n_fmt("Line {line}", &[("{line}", &(entry.line + 1).to_string())]).as_str()),
	}
	let changed = entry.is_changed();
	action_row.set_subtitle(&render_subtitle(entry));
	action_row.set_subtitle_lines(if changed { 2 } else { 1 });
	if let Some(btn) = reset_btn {
		btn.set_sensitive(changed);
	}
	if changed {
		action_row.add_css_class("changed");
	} else {
		action_row.remove_css_class("changed");
	}
	update_list_icons(action_row, entry);
}

const NOFAIL_WARNING_CLASS: &str = "nofail-warning";
const INVALID_ALERT_CLASS: &str = "invalid-alert";
const DISABLED_ICON_CLASS: &str = "disabled-icon";
const LIST_ICON_CLASSES: [&str; 3] = [NOFAIL_WARNING_CLASS, INVALID_ALERT_CLASS, DISABLED_ICON_CLASS];

fn make_icon(icon: &str, class: &str) -> Image {
	let icon = Image::from_icon_name(icon);
	icon.add_css_class(class);
	icon.set_margin_end(4);
	icon.set_valign(Align::Center);
	icon
}

fn update_list_icons(action_row: &ActionRow, entry: &StabEntry) {
	for class in LIST_ICON_CLASSES {
		if let Some(icon) = find_widget_with_class(action_row.clone(), class) {
			action_row.remove(&icon);
		}
	}

	if !entry.active {
		let disabled = make_icon("emblem-unreadable-symbolic", DISABLED_ICON_CLASS);
		disabled.set_tooltip_text(Some(i18n("This entry is disabled (commented out).").as_str()));
		action_row.add_suffix(&disabled);
	}

	if entry.active && !entry.has_option("nofail") {
		let warning = make_icon(WARNING_NAME, NOFAIL_WARNING_CLASS);
		warning.set_tooltip_text(Some(
			i18n(
				"The system may refuse to boot without this drive. If this is not intended, add the 'nofail' option. \n\
			Usually, this is wanted on root and home mounts.",
			)
			.as_str(),
		));
		action_row.add_suffix(&warning);
	}

	if !entry.is_valid() {
		let alert = make_icon(ERROR_NAME, INVALID_ALERT_CLASS);
		alert.set_tooltip_text(Some(i18n("This entry is invalid and cannot be parsed.").as_str()));
		action_row.add_suffix(&alert);
	}
}

fn render_subtitle(entry: &StabEntry) -> String {
	let current = esc(&entry.to_string());
	if !entry.is_changed() {
		return format!("<tt>{current}</tt>");
	}
	let original = esc(&entry.original_normalized());
	format!("<tt><i>- {original}</i>\n<b>+ {current}</b></tt>")
}

fn esc(s: &str) -> String {
	s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn make_list_row(list_box: &ListBox, file_ctx: &FileContext, editor_panel: &gtk::Box, entry: &StabEntry) -> ActionRow {
	let row = ActionRow::new();
	add_delete_button(list_box, &row, file_ctx, editor_panel);
	render_list_entry(&row, entry, None);
	row
}

fn populate_list(list_box: &ListBox, file_ctx: &FileContext, editor_panel: &gtk::Box) {
	clear_children(list_box);

	let entries: Vec<GC<StabEntry>> = file_ctx.file().borrow().entries().cloned().collect();

	let mut first = true;
	for entry in &entries {
		let row = make_list_row(list_box, file_ctx, editor_panel, &entry.borrow());
		list_box.append(&row);
		if first {
			first = false;
			list_box.select_row(Some(&row));
		}
	}

	let add_row = gtk::ListBoxRow::new();
	add_row.set_selectable(false);
	add_row.set_activatable(false);
	let add_btn = Button::builder().label(i18n("Add new mount entry")).hexpand(true).build();
	add_btn.add_css_class("flat");
	add_row.set_child(Some(&add_btn));

	let (list_box_ref, add_row_ref, file_ctx_ref) = (list_box.clone(), add_row.clone(), file_ctx.clone());
	let editor_panel_ref = editor_panel.clone();
	add_btn.connect_clicked(move |_| {
		let line = {
			let file = file_ctx_ref.file().borrow();
			file.entries().map(|e| e.borrow().line).max().map_or(0, |l| l + 1)
		};
		let new_entry = StabEntry::blank(line);
		let row = make_list_row(&list_box_ref, &file_ctx_ref, &editor_panel_ref, &new_entry);
		file_ctx_ref.file().borrow_mut().push_entry(new_entry);
		list_box_ref.insert(&row, add_row_ref.index());
		list_box_ref.select_row(Some(&row));
		file_ctx_ref.notify();
	});

	list_box.append(&add_row);
}

fn build_restore_picker(file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) -> MenuButton {
	let dataset = || match stab_yurself::scan_for_backups() {
		Ok(mut ok) => {
			if ok.is_empty() {
				Err(anyhow::anyhow!("{}", i18n("No backups found")))
			} else {
				ok.sort_by(|(_, a), (_, b)| b.cmp(a));
				Ok(ok)
			}
		}
		Err(err) => Err(err),
	};

	let render_row = |backup: &(PathBuf, SystemTime)| {
		let time = localized_datetime(backup.1);
		activatable_row(time, backup.0.display().to_string())
	};

	let filter = |query: &str, backup: &(PathBuf, SystemTime)| {
		let time = localized_datetime(backup.1);
		query_matches(query, &time) || query_matches(query, &backup.0.display().to_string())
	};

	let on_select = {
		let (file_ctx, list_panel, editor_panel) = (file_ctx.clone(), list_panel.clone(), editor_panel.clone());
		move |backup: (PathBuf, SystemTime), _index| {
			let (path, file_ctx, list_panel) = (backup.0.clone(), file_ctx.clone(), list_panel.clone());
			let (editor_panel, parent_widget) = (editor_panel.clone(), editor_panel.clone());
			let backup_time = i18n_fmt(
				"You are about to restore backup from:\n{time}",
				&[("{time}", &localized_datetime(backup.1))],
			);
			ui_commons::confirm_popup(&parent_widget, i18n("Are you sure? Any changes made will be lost!"), move || {
				restore_backup(&path, &file_ctx, &list_panel, &editor_panel)
			})
			.heading(backup_time)
			.confirm_choice(i18n("Restore"))
			.present();
		}
	};

	let menu_btn = SearchPickerBuilder::new(i18n("Restore backup"), dataset, render_row, on_select)
		.search_placeholder(i18n("Search backups"))
		.tooltip(i18n("Restore from a backup file"))
		.error_message(i18n("Failed to list backups"))
		.filter(filter)
		.wrap_label(true)
		.build();

	menu_btn.set_hexpand(true);
	menu_btn
}

fn reload_editor(list_panel: &ListBox, file_ctx: &FileContext, editor_panel: &gtk::Box) {
	clear_children(editor_panel);
	populate_list(list_panel, file_ctx, editor_panel);
	file_ctx.notify();
}

fn load_fstab_file(path: &Path, file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) -> anyhow::Result<()> {
	let new_file = StabFile::read(path)?;
	*file_ctx.file().borrow_mut() = new_file;
	reload_editor(list_panel, file_ctx, editor_panel);
	Ok(())
}

fn load_backup(path: &Path, file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) {
	if let Err(err) = load_fstab_file(path, file_ctx, list_panel, editor_panel) {
		ui_commons::present_simple_dialog(editor_panel, i18n("Could not load backup").as_str(), &format!("{err:#}"));
	}
}

fn restore_backup(path: &Path, file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) {
	let result = (|| -> anyhow::Result<()> {
		let baseline = StabFile::read("/etc/fstab")?;
		let backup = StabFile::read(path)?;
		let lines = baseline.overlay_backup(&backup);
		{
			let mut file = file_ctx.file().borrow_mut();
			file.lines = lines;
			file.reference = baseline.reference;
		}
		reload_editor(list_panel, file_ctx, editor_panel);
		Ok(())
	})();
	if let Err(err) = result {
		ui_commons::present_simple_dialog(editor_panel, i18n("Could not load backup").as_str(), &format!("{err:#}"));
	}
}

fn add_info_row(grid: &gtk::Grid, row: i32, key: &str, value: &str) {
	let key_label = gtk::Label::new(Some(key));
	key_label.set_xalign(0.0);
	key_label.add_css_class("dim-label");

	let value_label = gtk::Label::new(Some(value));
	value_label.set_xalign(0.0);
	value_label.set_wrap(true);
	value_label.set_hexpand(true);

	grid.attach(&key_label, 0, row, 1, 1);
	grid.attach(&value_label, 1, row, 1, 1);
}

fn add_delete_button(list_box: &ListBox, row: &ActionRow, file_ctx: &FileContext, editor_panel: &gtk::Box) {
	let delete_btn = trash_button(i18n("Delete entry").as_str());

	row.add_suffix(&delete_btn);

	let extra_child = {
		let (row, file_ctx) = (row.clone(), file_ctx.clone());
		move || {
			let file = file_ctx.file().borrow();
			let entry = file.entry_at(row.index() as usize)?;
			let entry = entry.borrow();
			let grid = gtk::Grid::builder()
				.column_spacing(16)
				.row_spacing(6)
				.halign(Align::Fill)
				.hexpand(true)
				.build();
			let mut grid_row = 0;
			if let Some(label) = &entry.user_label {
				add_info_row(&grid, grid_row, i18n("Label").as_str(), label);
				grid_row += 1;
			}
			add_info_row(&grid, grid_row, i18n("File system").as_str(), &entry.fs_type.to_string());
			add_info_row(&grid, grid_row + 1, i18n("Device").as_str(), &entry.device.value);
			add_info_row(&grid, grid_row + 2, i18n("Mount point").as_str(), &entry.mount_point);
			Some(grid.upcast())
		}
	};

	let on_confirm = {
		let (list_box, row, file_ctx) = (list_box.clone(), row.clone(), file_ctx.clone());
		let editor_panel = editor_panel.clone();
		move || {
			let index = row.index();
			if index < 0 {
				return;
			}
			let index = index as usize;
			list_box.remove(&row);
			file_ctx.file().borrow_mut().remove_entry(index);
			clear_children(&editor_panel);
			let remaining = file_ctx.file().borrow().entries().count();
			let new_index = index.min(remaining.saturating_sub(1));
			if let Some(new_row) = list_box.row_at_index(new_index as i32) {
				list_box.select_row(Some(&new_row));
			}
			file_ctx.notify();
		}
	};

	ui_commons::confirm_clicked_action(&delete_btn, i18n("Delete this entry?"))
		.confirm_choice(i18n("Delete"))
		.extra_child(extra_child)
		.connect(on_confirm);
}

fn build_split_layout(list_panel: &impl IsA<gtk::Widget>, editor_panel: &impl IsA<gtk::Widget>) -> GtkBox {
	let split_box = GtkBox::builder()
		.hexpand(true)
		.vexpand(true)
		.orientation(Orientation::Horizontal)
		.spacing(20)
		.homogeneous(true)
		.build();

	split_box.append(list_panel);
	split_box.append(editor_panel);

	split_box
}

fn attach_responsive_breakpoint(window: &adw::ApplicationWindow, split_box: &GtkBox) {
	let condition = BreakpointCondition::new_length(adw::BreakpointConditionLengthType::MaxWidth, 700.0, LengthUnit::Sp);

	let breakpoint = Breakpoint::new(condition);
	breakpoint.add_setter(split_box, "orientation", Some(&Orientation::Vertical.to_value()));
	breakpoint.add_setter(split_box, "homogeneous", Some(&false.to_value()));

	window.add_breakpoint(breakpoint);
}
