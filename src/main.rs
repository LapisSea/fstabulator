mod block_devices;
mod context;
mod credentials_flow;
mod device_value;
mod fs_options;
mod fs_value;
mod mount_point_value;
mod mount_status;
mod options_value;
mod popup;
mod privileged;
mod search_picker;
mod stab_yurself;
mod subvolume;

use crate::context::{EntryContext, FileContext};
use crate::mount_status::MountStatus;
use crate::search_picker::{ErrorRenderer, build_search_picker};
use crate::stab_yurself::{StabEntry, StabFile};
use adw::gdk::pango;
use adw::prelude::*;
use adw::{
	ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, EntryRow, HeaderBar, LengthUnit, PreferencesGroup, PreferencesRow,
	SpinRow, SwitchRow, Toast, ToastOverlay,
};
use fs_value::FsType;
use gtk::{Adjustment, Align, Box as GtkBox, Button, Image, ListBox, MenuButton, Orientation, ScrolledWindow, SelectionMode, Widget};
use options_value::build_options_group;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, SystemTime};

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

fn register_icon() {
	gtk::gio::resources_register_include!("compiled.gresource").expect("Failed to register app resources");
	if let Some(display) = gtk::gdk::Display::default() {
		gtk::IconTheme::for_display(&display).add_resource_path("/org/lapissea/FSTabulator/icons");
	}
	gtk::Window::set_default_icon_name("fstabulator");
}

fn main() -> gtk::glib::ExitCode {
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
	register_icon();

	let window_build = ApplicationWindow::builder()
		.application(application)
		.title("FSTabulator")
		.default_width(800)
		.default_height(600);

	let stab_file = GC::new(StabFile::empty());

	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();

	let list_panel = build_entry_list();

	let file_buttons_panel = GtkBox::builder().orientation(Orientation::Vertical).hexpand(true).spacing(6).build();

	let row = GtkBox::builder().orientation(Orientation::Horizontal).hexpand(true).spacing(12).build();
	file_buttons_panel.append(&row);

	let make_backup_btn = make_icon_label_button("document-save-as-symbolic", "Make backup");
	let save_changes_btn = make_icon_label_button("document-save-symbolic", "Save changes");
	let revert_changes_btn = make_icon_label_button("edit-undo-symbolic", "Revert changes");
	save_changes_btn.add_css_class("suggested-action");
	revert_changes_btn.add_css_class("destructive-action");

	let file_ctx = FileContext::new(
		stab_file.clone(),
		Rc::new({
			let stab_file = stab_file.clone();
			let save_changes_btn = save_changes_btn.clone();
			let revert_changes_btn = revert_changes_btn.clone();
			move || {
				let changed = stab_file.borrow().is_changed();
				save_changes_btn.set_sensitive(changed);
				revert_changes_btn.set_sensitive(changed);
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
			let btn = btn.clone();
			let parent = btn.clone();
			popup::confirm_popup(
				&parent,
				"Make backup",
				"Your changes have not been saved yet. The backup will reflect the saved /etc/fstab, not your unsaved changes. Continue?",
				None::<&Widget>,
				move || perform_make_backup(&btn),
			);
		});
	}
	row.append(&make_backup_btn);

	let toast_overlay = ToastOverlay::new();
	{
		let file_ctx = file_ctx.clone();
		let list_panel = list_panel.clone();
		let editor_panel = editor_panel.clone();
		let toast_overlay = toast_overlay.clone();
		popup::connect_clicked_confirm(
			&save_changes_btn,
			"Save",
			"Are you sure you want to write these changes to /etc/fstab?",
			|| None,
			move || {
				let content = {
					let file = file_ctx.file().borrow();
					file.to_string()
				};
				match privileged::write_fstab(&content) {
					Ok(()) => {
						if let Err(err) = load_fstab_file(Path::new("/etc/fstab"), &file_ctx, &list_panel, &editor_panel) {
							popup::present_simple_dialog(&editor_panel, "Saved, but could not reload", &format!("{err:#}"));
							return;
						}
						toast_overlay.add_toast(Toast::new("Saved to /etc/fstab"));
					}
					Err(err) => popup::present_simple_dialog(&editor_panel, "Could not save", &format!("{err:#}")),
				}
			},
		);
	}

	{
		let file_ctx = file_ctx.clone();
		let list_panel = list_panel.clone();
		let editor_panel = editor_panel.clone();
		popup::connect_clicked_confirm(
			&revert_changes_btn,
			"Revert",
			"Are you sure? Any changes made will be lost!",
			|| None,
			move || load_backup(Path::new("/etc/fstab"), &file_ctx, &list_panel, &editor_panel),
		);
	}

	row.append(&build_restore_picker(&file_ctx, &list_panel, &editor_panel));

	let row = GtkBox::builder().orientation(Orientation::Horizontal).hexpand(true).spacing(12).build();
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
			.label("'/etc/fstab' entries:")
			.margin_start(20)
			.halign(Align::Start)
			.build(),
	);

	left_panel.append(&wrap_scroll(&list_panel));
	left_panel.append(&file_buttons_panel);

	let split_box = build_split_layout(&left_panel, &wrap_scroll(&editor_panel));

	let rebuild_editor: RebuildEditor = GC::new(None);
	{
		let editor_panel = editor_panel.clone();
		let list_panel_cb = list_panel.clone();
		let stab_file = stab_file.clone();
		let file_ctx = file_ctx.clone();
		let rebuild_editor = rebuild_editor.clone();
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
				let editor_panel = editor_panel.clone();
				let list_panel_cb = list_panel_cb.clone();
				let entry_ctx = entry_ctx.clone();
				let row = row.clone();
				let rebuild_editor = rebuild_editor.clone();
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

	attach_responsive_breakpoint(&window, &split_box);

	let provider = gtk::CssProvider::new();
	provider.load_from_data(
		".invalid-alert { color: red; }\
		.mount-status-mounted { color: @success_color; }\
		.mount-status-unmounted { color: @warning_color; }\
		.mount-status-missing { color: @error_color; }\
		.mount-point-exists { color: @success_color; }\
		.connection-ok { color: @success_color; }",
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
	let label = gtk::Label::new(Some("Error loading fstab file!"));
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
		Ok(()) => popup::present_simple_dialog(btn, "Backup created", "A backup of /etc/fstab was created."),
		Err(err) => popup::present_simple_dialog(btn, "Could not create backup", &format!("{err:#}")),
	}
}

fn make_icon_label_button(icon: &str, label: &str) -> Button {
	let hbox = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.halign(Align::Center)
		.spacing(6)
		.build();
	hbox.append(&Image::from_icon_name(icon));
	hbox.append(&gtk::Label::new(Some(label)));
	let button = Button::new();
	button.set_label(&label);
	button.set_child(Some(&hbox));
	button.set_hexpand(true);
	button
}

fn wrap_scroll(content: &impl IsA<Widget>) -> ScrolledWindow {
	ScrolledWindow::builder().child(content).hexpand(true).vexpand(true).build()
}

pub(crate) fn render_list_entry(action_row: &ActionRow, entry: &StabEntry, reset_btn: Option<&Button>) {
	action_row.set_title(&entry.user_label.as_ref().cloned().unwrap_or_else(|| format!("Line {}", entry.line + 1)));
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
		if let Some(icon) = find_widget_with_class(action_row.clone().upcast(), class) {
			action_row.remove(&icon);
		}
	}

	if !entry.active {
		let disabled = make_icon("emblem-unreadable-symbolic", DISABLED_ICON_CLASS);
		disabled.set_tooltip_text(Some("This entry is disabled (commented out)."));
		action_row.add_suffix(&disabled);
	}

	if entry.active && !entry.has_option("nofail") {
		let warning = make_icon("dialog-warning-symbolic", NOFAIL_WARNING_CLASS);
		warning.set_tooltip_text(Some(
			"The system may refuse to boot without this drive. If this is not intended, add the 'nofail' option. \n\
			Usually, this is wanted on root and home mounts.",
		));
		action_row.add_suffix(&warning);
	}

	if !entry.is_valid() {
		let alert = make_icon("dialog-error-symbolic", INVALID_ALERT_CLASS);
		alert.set_tooltip_text(Some("This entry is invalid and cannot be parsed."));
		action_row.add_suffix(&alert);
	}
}

fn find_widget_with_class(widget: gtk::Widget, class: &str) -> Option<gtk::Widget> {
	if widget.has_css_class(class) {
		return Some(widget);
	}
	let mut child = widget.first_child();
	while let Some(child_widget) = child {
		let next = child_widget.next_sibling();
		if let Some(found) = find_widget_with_class(child_widget.clone(), class) {
			return Some(found);
		}
		child = next;
	}
	None
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

fn build_entry_list() -> ListBox {
	ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
		.valign(Align::Start)
		.build()
}

pub(crate) fn clear_children<W: IsA<gtk::Widget>>(widget: &W) {
	let widget = widget.upcast_ref::<gtk::Widget>();
	if let Some(list_box) = widget.downcast_ref::<ListBox>() {
		while let Some(row) = list_box.row_at_index(0) {
			list_box.remove(&row);
		}
	} else {
		while let Some(child) = widget.first_child() {
			child.unparent();
		}
	}
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
	let add_btn = Button::builder().label("Add new mount entry").hexpand(true).build();
	add_btn.add_css_class("flat");
	add_row.set_child(Some(&add_btn));

	let list_box_ref = list_box.clone();
	let add_row_ref = add_row.clone();
	let file_ctx_ref = file_ctx.clone();
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
		Ok(ok) => {
			if ok.is_empty() {
				Err(anyhow::anyhow!("No backups found"))
			} else {
				Ok(ok)
			}
		}
		Err(err) => Err(err),
	};

	let render_row = |backup: &(PathBuf, SystemTime)| {
		let time = humantime::format_rfc3339(backup.1).to_string();
		let row = ActionRow::builder().title(time).subtitle(backup.0.display().to_string()).build();
		row.set_activatable(true);
		row.upcast::<Widget>()
	};

	let filter = |query: &str, backup: &(PathBuf, SystemTime)| {
		if query.trim().is_empty() {
			return true;
		}
		let query = query.to_lowercase();
		let time = humantime::format_rfc3339(backup.1).to_string().to_lowercase();
		backup.0.display().to_string().to_lowercase().contains(&query) || time.contains(&query)
	};

	let on_select = {
		let file_ctx = file_ctx.clone();
		let list_panel = list_panel.clone();
		let editor_panel = editor_panel.clone();
		move |backup: (PathBuf, SystemTime), _index| {
			let path = backup.0.clone();
			let file_ctx = file_ctx.clone();
			let list_panel = list_panel.clone();
			let editor_panel = editor_panel.clone();
			let parent_widget = editor_panel.clone();
			popup::confirm_popup(
				&parent_widget,
				"Restore",
				"Are you sure? Any changes made will be lost!",
				None::<&Widget>,
				move || restore_backup(&path, &file_ctx, &list_panel, &editor_panel),
			);
		}
	};

	let menu_btn = build_search_picker(
		"Search backups",
		"Restore backup",
		"Restore from a backup file",
		dataset,
		render_row,
		ErrorRenderer::Message("Failed to list backups"),
		filter,
		on_select,
	);

	menu_btn.set_hexpand(true);
	menu_btn
}

fn load_fstab_file(path: &Path, file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) -> anyhow::Result<()> {
	let new_file = StabFile::read(path)?;
	*file_ctx.file().borrow_mut() = new_file;
	clear_children(editor_panel);
	populate_list(list_panel, file_ctx, editor_panel);
	file_ctx.notify();
	Ok(())
}

fn load_backup(path: &Path, file_ctx: &FileContext, list_panel: &ListBox, editor_panel: &gtk::Box) {
	if let Err(err) = load_fstab_file(path, file_ctx, list_panel, editor_panel) {
		popup::present_simple_dialog(editor_panel, "Could not load backup", &format!("{err:#}"));
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
		clear_children(editor_panel);
		populate_list(list_panel, file_ctx, editor_panel);
		file_ctx.notify();
		Ok(())
	})();
	if let Err(err) = result {
		popup::present_simple_dialog(editor_panel, "Could not load backup", &format!("{err:#}"));
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
	let delete_btn = Button::from_icon_name("user-trash-symbolic");
	delete_btn.add_css_class("flat");
	delete_btn.add_css_class("error");
	delete_btn.set_valign(Align::Center);
	delete_btn.set_tooltip_text(Some("Delete entry"));

	row.add_suffix(&delete_btn);

	let extra_child = {
		let row = row.clone();
		let file_ctx = file_ctx.clone();
		move || {
			let file = file_ctx.file().borrow();
			let Some(entry) = file.entry_at(row.index() as usize) else {
				return None;
			};
			let entry = entry.borrow();
			let grid = gtk::Grid::builder()
				.column_spacing(16)
				.row_spacing(6)
				.halign(Align::Fill)
				.hexpand(true)
				.build();
			let mut grid_row = 0;
			if let Some(label) = &entry.user_label {
				add_info_row(&grid, grid_row, "Label", label);
				grid_row += 1;
			}
			add_info_row(&grid, grid_row, "File system", &entry.fs_type.to_string());
			add_info_row(&grid, grid_row + 1, "Device", &entry.device.value);
			add_info_row(&grid, grid_row + 2, "Mount point", &entry.mount_point);
			Some(grid.upcast())
		}
	};

	let on_confirm = {
		let list_box = list_box.clone();
		let row = row.clone();
		let file_ctx = file_ctx.clone();
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

	popup::connect_clicked_confirm(&delete_btn, "Delete", "Delete this entry?", extra_child, on_confirm);
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

fn build_editor_panel(
	editor_panel: &gtk::Box,
	entry_ctx: &EntryContext,
	list_box: &ListBox,
	list_row: &gtk::ListBoxRow,
	rebuild_editor: RebuildEditor,
) {
	let reset_btn = Button::with_label("Reset");
	reset_btn.set_sensitive(entry_ctx.entry().borrow().is_changed());
	entry_ctx.set_reset_btn(&reset_btn);

	let edit_props = PreferencesGroup::builder().title("Edit properties").build();
	editor_panel.append(&edit_props);

	let options_group = PreferencesGroup::builder().title("Options").build();
	editor_panel.append(&options_group);

	add_user_label_row(&edit_props, entry_ctx);
	let device_row = device_value::add_device_row(&edit_props, entry_ctx);
	mount_point_value::add_mount_point_row(&edit_props, entry_ctx);
	{
		let (entry_ctx, device_row, options_group) = (entry_ctx.clone(), device_row.clone(), options_group.clone());
		fs_value::add_fs_type_row(&edit_props.clone(), &entry_ctx.clone(), {
			move || {
				device_row.refresh_kinds();
				build_options_group(&options_group, &entry_ctx);
			}
		});
	}

	let active_row = SwitchRow::builder().title("Active").active(entry_ctx.entry().borrow().active).build();
	{
		let entry_ctx = entry_ctx.clone();
		active_row.connect_active_notify(move |row| {
			entry_ctx.entry().borrow_mut().active = row.is_active();
			entry_ctx.render();
		});
	}
	edit_props.add(&active_row);

	build_options_group(&options_group, entry_ctx);

	editor_panel.append(&reset_btn);

	add_mount_group(editor_panel, entry_ctx.entry(), rebuild_editor);

	let fsck_group = PreferencesGroup::builder().title("Extra").build();
	add_spin_row(
		&fsck_group,
		entry_ctx,
		"Dump",
		"Controls the dump backup frequency; 0 disables",
		entry_ctx.entry().borrow().dump,
		|entry, value| entry.dump = value,
	);
	add_spin_row(
		&fsck_group,
		entry_ctx,
		"Pass",
		"Controls the fsck check order; 0 disables",
		entry_ctx.entry().borrow().pass,
		|entry, value| entry.pass = value,
	);
	editor_panel.append(&fsck_group);

	let (list_box, list_row) = (list_box.clone(), list_row.clone());
	let options_group = options_group.clone();
	let device_row = device_row.clone();
	let entry_ctx_ref = entry_ctx.clone();
	reset_btn.connect_clicked(move |_| {
		entry_ctx_ref.entry().borrow_mut().reset();
		device_row.refresh_kinds();
		build_options_group(&options_group, &entry_ctx_ref);
		entry_ctx_ref.render();
		list_box.unselect_all();
		list_box.select_row(Some(&list_row));
	});
}

fn present_unsaved_changes(btn: &Button, heading: &str, subject: &str) {
	popup::present_simple_dialog(
		btn,
		&format!("Cannot {heading}"),
		&format!("{subject} has unsaved changes. Save your changes before {heading}ing."),
	);
}

fn report_action_outcome(btn: &Button, done: &str, failed: &str, subject: &str, result: anyhow::Result<()>, refresh: &Rc<dyn Fn()>) {
	match result {
		Ok(()) => {
			popup::present_simple_dialog(btn, done, &format!("{done} {subject}."));
			refresh();
		}
		Err(err) => popup::present_simple_dialog(btn, failed, &format!("{err:#}")),
	}
}

fn add_mount_group(editor_panel: &gtk::Box, entry: &GC<StabEntry>, rebuild_editor: RebuildEditor) {
	let group = PreferencesGroup::builder().title("Mount actions").build();
	editor_panel.append(&group);

	let status_label = gtk::Label::new(None);
	status_label.set_xalign(0.5);
	status_label.set_halign(Align::Center);
	status_label.set_wrap(true);
	status_label.add_css_class("monospace");
	status_label.set_margin_top(6);
	status_label.set_margin_bottom(6);

	let status_row = PreferencesRow::builder().title("Status").child(&status_label).build();
	group.add(&status_row);

	let mount_btn = Button::builder().label("Mount").css_classes(["suggested-action"]).build();
	let remount_btn = Button::builder().label("Remount").build();
	let unmount_btn = Button::builder().label("Unmount").css_classes(["destructive-action"]).build();

	let buttons = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).hexpand(true).build();
	for btn in [&mount_btn, &remount_btn, &unmount_btn] {
		btn.set_hexpand(true);
		buttons.append(btn);
	}

	let buttons_row = PreferencesRow::builder().title("Actions").child(&buttons).build();
	buttons_row.set_activatable(false);
	group.add(&buttons_row);

	let refresh: Rc<dyn Fn()> = Rc::new({
		let entry = entry.clone();
		let status_label = status_label.clone();
		let mount_btn = mount_btn.clone();
		let remount_btn = remount_btn.clone();
		let unmount_btn = unmount_btn.clone();
		move || {
			let entry = entry.borrow();
			let status = mount_status::detect(&entry);
			let is_swap = entry.fs_type == FsType::Swap;
			status_label.set_label(status.label());
			status_label.set_tooltip_text(Some(status.tooltip()));
			for class in [MountStatus::Mounted, MountStatus::Unmounted, MountStatus::Missing] {
				status_label.remove_css_class(class.css_class());
			}
			status_label.add_css_class(status.css_class());
			mount_btn.set_sensitive(status != MountStatus::Mounted);
			remount_btn.set_sensitive(status == MountStatus::Mounted && !is_swap);
			unmount_btn.set_sensitive(status == MountStatus::Mounted);
		}
	});
	refresh();

	{
		let group = group.clone();
		let refresh = refresh.clone();
		gtk::glib::timeout_add_local(Duration::from_secs(2), move || {
			if !group.is_mapped() {
				return gtk::glib::ControlFlow::Break;
			}
			refresh();
			gtk::glib::ControlFlow::Continue
		});
	}

	{
		let entry = entry.clone();
		let refresh = refresh.clone();
		let rebuild_editor = rebuild_editor.clone();
		let btn = mount_btn.clone();
		popup::connect_clicked_confirm(
			&mount_btn,
			"Mount",
			"Are you sure you want to mount this entry?",
			|| None,
			move || {
				let snapshot = entry.cloned(|e| e);
				if snapshot.mount_point.trim().is_empty() && snapshot.fs_type != FsType::Swap {
					popup::present_simple_dialog(&btn, "Cannot mount", "The mount point is empty.");
					return;
				}
				if snapshot.is_changed() {
					present_unsaved_changes(&btn, "mount", "The entry");
					return;
				}
				if credentials_flow::needs_credentials(&snapshot) {
					credentials_flow::mount_with_credentials(&btn, entry.clone(), snapshot.clone(), rebuild_editor.clone(), refresh.clone());
				} else {
					let device = credentials_flow::action_device(&snapshot);
					let is_swap = snapshot.fs_type == FsType::Swap;
					let fs_type = snapshot.fs_type.to_string();
					let result = privileged::mount(&snapshot.mount_point, &device, is_swap, &fs_type, None);
					report_action_outcome(&btn, "Mounted", "Could not mount", &snapshot.mount_point, result, &refresh);
				}
			},
		);
	}
	{
		let entry = entry.clone();
		let refresh = refresh.clone();
		let btn = remount_btn.clone();
		popup::connect_clicked_confirm(
			&remount_btn,
			"Remount",
			"Are you sure you want to remount this entry?",
			|| None,
			move || {
				let (mount_point, is_swap, changed) = {
					let entry = entry.borrow();
					(entry.mount_point.clone(), entry.fs_type == FsType::Swap, entry.is_changed())
				};
				if is_swap {
					popup::present_simple_dialog(&btn, "Cannot remount", "Swap cannot be remounted.");
					return;
				}
				if mount_point.trim().is_empty() {
					popup::present_simple_dialog(&btn, "Cannot remount", "The mount point is empty.");
					return;
				}
				if changed {
					present_unsaved_changes(&btn, "remount", "The entry");
					return;
				}
				let result = privileged::remount(&mount_point, is_swap);
				report_action_outcome(&btn, "Remounted", "Could not remount", &mount_point, result, &refresh);
			},
		);
	}
	{
		let entry = entry.clone();
		let refresh = refresh.clone();
		let btn = unmount_btn.clone();
		popup::connect_clicked_confirm(
			&unmount_btn,
			"Unmount",
			"Are you sure you want to unmount this entry?",
			|| None,
			move || {
				let (mount_point, device, is_swap, mount_point_changed) = {
					let entry = entry.borrow();
					(
						entry.mount_point.clone(),
						credentials_flow::action_device(&entry),
						entry.fs_type == FsType::Swap,
						entry.mount_point_changed(),
					)
				};
				if mount_point.trim().is_empty() && !is_swap {
					popup::present_simple_dialog(&btn, "Cannot unmount", "The mount point is empty.");
					return;
				}
				if mount_point_changed {
					present_unsaved_changes(&btn, "unmount", "The mount point");
					return;
				}
				let result = privileged::unmount(&mount_point, &device, is_swap);
				report_action_outcome(&btn, "Unmounted", "Could not unmount", &mount_point, result, &refresh);
			},
		);
	}
}

fn add_user_label_row(options: &PreferencesGroup, entry_ctx: &EntryContext) {
	let row = EntryRow::builder()
		.title("Label")
		.text(entry_ctx.entry().borrow().user_label.as_deref().unwrap_or(""))
		.build();
	{
		let entry_ctx = entry_ctx.clone();
		row.connect_changed(move |row| {
			let text = row.text();
			{
				let mut entry = entry_ctx.entry().borrow_mut();
				entry.user_label = if text.is_empty() { None } else { Some(text.to_string()) };
			}
			entry_ctx.render();
		});
	}
	options.add(&row);
}

fn add_spin_row(
	options: &PreferencesGroup,
	entry_ctx: &EntryContext,
	title: &str,
	subtitle: &str,
	initial: u8,
	apply: impl Fn(&mut StabEntry, u8) + 'static,
) {
	let entry_ctx = entry_ctx.clone();

	let adjustment = Adjustment::builder().value(f64::from(initial)).step_increment(1.0).build();

	let row = SpinRow::new(Some(&adjustment), 1.0, 0);
	row.set_title(title);
	row.set_subtitle(subtitle);
	row.set_range(0.0, 255.0);
	row.set_climb_rate(1.0);
	row.set_numeric(true);
	row.set_value(f64::from(initial));
	let row_ref = row.clone();
	row.adjustment().connect_value_changed(move |_| {
		let value = row_ref.value().round() as u8;
		{
			let mut entry = entry_ctx.entry().borrow_mut();
			apply(&mut entry, value);
		}
		entry_ctx.render();
	});
	options.add(&row);
}
