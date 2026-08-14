mod device_value;
mod fs_options;
mod fs_value;
mod mount_point_value;
mod options_value;
mod search_picker;
mod stab_yurself;
mod subvolume;

use crate::search_picker::{ErrorRenderer, build_search_picker};
use crate::stab_yurself::{StabEntry, StabLine};
use adw::gdk::pango;
use adw::prelude::*;
use adw::{
	ActionRow, AlertDialog, Application, ApplicationWindow, Breakpoint, BreakpointCondition, EntryRow, HeaderBar, LengthUnit, PreferencesGroup,
	SpinRow, SwitchRow,
};
use gtk::{Adjustment, Align, Box as GtkBox, Image, ListBox, MenuButton, Orientation, ScrolledWindow, SelectionMode, Widget};
use options_value::build_options_group;
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

const APP_ID: &str = "org.lapissea.FSTabulator";

fn main() -> gtk::glib::ExitCode {
	let application = Application::builder().application_id(APP_ID).build();
	application.connect_activate(build_ui);
	application.run()
}

fn build_ui(application: &Application) {
	let window_build = ApplicationWindow::builder()
		.application(application)
		.title("FSTabulator")
		.default_width(800)
		.default_height(600);

	let fstab_file = match stab_yurself::read_fstab() {
		Ok(data) => data,
		Err(err) => {
			let main_box = GtkBox::builder().orientation(Orientation::Vertical).build();
			main_box.append(&HeaderBar::new());
			build_load_error(&main_box, err);
			window_build.content(&main_box).build().present();
			return;
		}
	};

	let entries: GC<Vec<GC<StabEntry>>> = GC::new(
		fstab_file
			.into_iter()
			.filter_map(|e| match e {
				StabLine::Entry(e) => Some(e),
				_ => None,
			})
			.map(GC::new)
			.collect(),
	);

	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();

	let list_panel = build_entry_list(&entries, &editor_panel);

	let file_buttons_panel = GtkBox::builder().orientation(Orientation::Vertical).hexpand(true).spacing(6).build();

	let row = GtkBox::builder().orientation(Orientation::Horizontal).hexpand(true).spacing(12).build();
	file_buttons_panel.append(&row);

	let make_backup_btn = make_icon_label_button("document-save-as-symbolic", "Make backup");
	{
		make_backup_btn.connect_clicked(move |btn| match stab_yurself::make_backup() {
			Ok(()) => present_simple_dialog(btn, "Backup created", "A backup of /etc/fstab was created."),
			Err(err) => present_simple_dialog(btn, "Could not create backup", &format!("{err:#}")),
		});
	}
	row.append(&make_backup_btn);

	row.append(&build_restore_picker(&entries, &list_panel, &editor_panel));

	let row = GtkBox::builder().orientation(Orientation::Horizontal).hexpand(true).spacing(12).build();
	file_buttons_panel.append(&row);

	let save_changes_btn = make_icon_label_button("document-save-symbolic", "Save changes");
	{
		save_changes_btn.connect_clicked(move |_| {
			println!("Save changes");
		});
	}
	row.append(&save_changes_btn);

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

	{
		let editor_panel = editor_panel.clone();
		let list_panel_cb = list_panel.clone();
		let entries = entries.clone();
		list_panel.connect_row_selected(move |_, row| {
			while let Some(child) = editor_panel.last_child() {
				editor_panel.remove(&child);
			}
			let Some(row) = row else { return };
			if row.index() < 0 {
				return;
			}
			let Some(entry) = entries.borrow().get(row.index() as usize).cloned() else {
				return;
			};
			let Ok(action_row) = row.clone().downcast::<ActionRow>() else { return };
			build_editor_panel(&editor_panel, &entry, &action_row, &list_panel_cb, &row);
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

	let window = window_build.content(&main_box).build();

	attach_responsive_breakpoint(&window, &split_box);

	let provider = gtk::CssProvider::new();
	provider.load_from_data(".invalid-alert { color: red; }");
	gtk::style_context_add_provider_for_display(&RootExt::display(&window), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

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

fn make_icon_label_button(icon: &str, label: &str) -> gtk::Button {
	let hbox = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.halign(Align::Center)
		.spacing(6)
		.build();
	hbox.append(&Image::from_icon_name(icon));
	hbox.append(&gtk::Label::new(Some(label)));
	let button = gtk::Button::new();
	button.set_label(&label);
	button.set_child(Some(&hbox));
	button.set_hexpand(true);
	button
}

fn wrap_scroll(content: &impl IsA<Widget>) -> ScrolledWindow {
	ScrolledWindow::builder().child(content).hexpand(true).vexpand(true).build()
}

pub(crate) fn render_list_entry(action_row: &ActionRow, entry: &StabEntry, reset_btn: Option<&gtk::Button>) {
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

fn make_list_row(list_box: &ListBox, entries: &GC<Vec<GC<StabEntry>>>, editor_panel: &gtk::Box, entry: &GC<StabEntry>) -> ActionRow {
	let row = ActionRow::new();
	add_delete_button(list_box, &row, entries, editor_panel);
	render_list_entry(&row, &entry.borrow(), None);
	row
}

fn build_entry_list(entries: &GC<Vec<GC<StabEntry>>>, editor_panel: &gtk::Box) -> ListBox {
	let list_box = ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
		.valign(Align::Start)
		.build();
	populate_list(&list_box, entries, editor_panel);
	list_box
}

fn populate_list(list_box: &ListBox, entries: &GC<Vec<GC<StabEntry>>>, editor_panel: &gtk::Box) {
	while let Some(child) = list_box.first_child() {
		list_box.remove(&child);
	}

	let mut first = true;
	for entry in entries.borrow().iter() {
		let row = make_list_row(list_box, entries, editor_panel, entry);
		list_box.append(&row);
		if first {
			first = false;
			list_box.select_row(Some(&row));
		}
	}

	let add_row = gtk::ListBoxRow::new();
	add_row.set_selectable(false);
	add_row.set_activatable(false);
	let add_btn = gtk::Button::builder().label("Add new mount entry").hexpand(true).build();
	add_btn.add_css_class("flat");
	add_row.set_child(Some(&add_btn));

	let list_box_ref = list_box.clone();
	let add_row_ref = add_row.clone();
	let entries_ref = entries.clone();
	let editor_panel_ref = editor_panel.clone();
	add_btn.connect_clicked(move |_| {
		let line = entries_ref.borrow().iter().map(|e| e.borrow().line).max().map_or(0, |l| l + 1);
		let new_entry = GC::new(StabEntry::blank(line));
		entries_ref.borrow_mut().push(new_entry.clone());

		let row = make_list_row(&list_box_ref, &entries_ref, &editor_panel_ref, &new_entry);
		list_box_ref.insert(&row, add_row_ref.index());
		list_box_ref.select_row(Some(&row));
	});

	list_box.append(&add_row);
}

fn present_simple_dialog(widget: &impl IsA<Widget>, heading: &str, body: &str) {
	let parent = widget.root().and_then(|root| root.downcast::<gtk::Window>().ok());
	let dialog = AlertDialog::builder().heading(heading).body(body).build();
	dialog.add_response("ok", "OK");
	dialog.set_default_response(Some("ok"));
	dialog.present(parent.as_ref());
}

fn build_restore_picker(entries: &GC<Vec<GC<StabEntry>>>, list_panel: &ListBox, editor_panel: &gtk::Box) -> MenuButton {
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
		let entries = entries.clone();
		let list_panel = list_panel.clone();
		let editor_panel = editor_panel.clone();
		move |backup: (PathBuf, SystemTime), _index| {
			let changed = entries.borrow().iter().any(|e| e.borrow().is_changed());
			if !changed {
				load_backup(&backup.0, &entries, &list_panel, &editor_panel);
				return;
			}
			let dialog = AlertDialog::builder()
				.heading("Restore backup?")
				.body("Are you sure? Any changes made will be lost!")
				.build();
			dialog.add_response("cancel", "Cancel");
			dialog.add_response("restore", "Restore");
			dialog.set_default_response(Some("cancel"));
			dialog.set_close_response("cancel");
			let parent = editor_panel.root().and_then(|root| root.downcast::<gtk::Window>().ok());
			let path = backup.0.clone();
			let entries = entries.clone();
			let list_panel = list_panel.clone();
			let editor_panel = editor_panel.clone();
			dialog.connect_response(None, move |_, response| {
				if response == "restore" {
					load_backup(&path, &entries, &list_panel, &editor_panel);
				}
			});
			dialog.present(parent.as_ref());
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

fn load_backup(path: &Path, entries: &GC<Vec<GC<StabEntry>>>, list_panel: &ListBox, editor_panel: &gtk::Box) {
	let lines = match stab_yurself::read_fstab_from(path) {
		Ok(lines) => lines,
		Err(err) => {
			present_simple_dialog(editor_panel, "Could not load backup", &format!("{err:#}"));
			return;
		}
	};
	let new_entries: Vec<GC<StabEntry>> = lines
		.into_iter()
		.filter_map(|line| match line {
			StabLine::Entry(e) => Some(GC::new(e)),
			_ => None,
		})
		.collect();
	*entries.borrow_mut() = new_entries;
	while let Some(child) = editor_panel.last_child() {
		editor_panel.remove(&child);
	}
	populate_list(list_panel, entries, editor_panel);
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

fn add_delete_button(list_box: &ListBox, row: &ActionRow, entries: &GC<Vec<GC<StabEntry>>>, editor_panel: &gtk::Box) {
	let delete_btn = gtk::Button::from_icon_name("user-trash-symbolic");
	delete_btn.add_css_class("flat");
	delete_btn.add_css_class("error");
	delete_btn.set_valign(Align::Center);
	delete_btn.set_tooltip_text(Some("Delete entry"));

	row.add_suffix(&delete_btn);

	let list_box = list_box.clone();
	let row = row.clone();
	let entries = entries.clone();
	let editor_panel = editor_panel.clone();
	let delete_btn_ref = delete_btn.clone();
	delete_btn.connect_clicked(move |_| {
		let borrowed_entries = entries.borrow();
		let entry = borrowed_entries.get(row.index() as usize).map(|e| e.borrow());

		let dialog = adw::AlertDialog::builder().heading("Delete this entry?").build();
		if let Some(entry) = entry {
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
			dialog.set_extra_child(Some(&grid));
		} else {
			dialog.set_body("The entry will be removed from the list.");
		}
		dialog.add_response("cancel", "Cancel");
		dialog.add_response("delete", "Delete");
		dialog.set_default_response(Some("cancel"));
		dialog.set_close_response("cancel");

		let list_box = list_box.clone();
		let row = row.clone();
		let entries = entries.clone();
		let editor_panel = editor_panel.clone();
		dialog.connect_response(None, move |_, response| {
			if response != "delete" {
				return;
			}
			let index = row.index();
			if index < 0 {
				return;
			}
			let index = index as usize;
			list_box.remove(&row);
			entries.borrow_mut().remove(index);
			while let Some(child) = editor_panel.last_child() {
				editor_panel.remove(&child);
			}
			let remaining = entries.borrow().len();
			let new_index = index.min(remaining.saturating_sub(1));
			if let Some(new_row) = list_box.row_at_index(new_index as i32) {
				list_box.select_row(Some(&new_row));
			}
		});

		let parent = delete_btn_ref.root().and_then(|root| root.downcast::<gtk::Window>().ok());
		dialog.present(parent.as_ref());
	});
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

fn build_editor_panel(editor_panel: &gtk::Box, entry: &GC<StabEntry>, action_row: &ActionRow, list_box: &ListBox, list_row: &gtk::ListBoxRow) {
	let reset_btn = gtk::Button::with_label("Reset");
	reset_btn.set_sensitive(entry.borrow().is_changed());

	let edit_props = PreferencesGroup::builder().title("Edit properties").build();
	editor_panel.append(&edit_props);

	let options_group = PreferencesGroup::builder().title("Options").build();
	editor_panel.append(&options_group);

	add_user_label_row(&edit_props, entry, action_row, &reset_btn);
	device_value::add_device_row(&edit_props, entry, action_row, &reset_btn);
	mount_point_value::add_mount_point_row(&edit_props, entry, action_row, &reset_btn);
	{
		let (options_group, reset_btn) = (options_group.clone(), reset_btn.clone());
		let (action_row, entry) = (action_row.clone(), entry.clone());
		fs_value::add_fs_type_row(&edit_props.clone(), &entry.clone(), &action_row.clone(), &reset_btn.clone(), {
			move || {
				build_options_group(&options_group, &entry, &action_row, &reset_btn);
			}
		});
	}

	add_spin_row(&edit_props, entry, action_row, "Dump", entry.borrow().dump, &reset_btn, |entry, value| {
		entry.dump = value
	});
	add_spin_row(&edit_props, entry, action_row, "Pass", entry.borrow().pass, &reset_btn, |entry, value| {
		entry.pass = value
	});

	let active_row = SwitchRow::builder().title("Active").active(entry.borrow().active).build();
	{
		let (entry, action_row, reset_btn) = (entry.clone(), action_row.clone(), reset_btn.clone());
		active_row.connect_active_notify(move |row| {
			let mut entry = entry.borrow_mut();
			entry.active = row.is_active();
			render_list_entry(&action_row, &entry, Some(&reset_btn));
		});
	}
	edit_props.add(&active_row);

	build_options_group(&options_group, entry, action_row, &reset_btn);

	editor_panel.append(&reset_btn);

	let entry = entry.clone();
	let action_row = action_row.clone();
	let list_box = list_box.clone();
	let list_row = list_row.clone();
	let options_group = options_group.clone();
	let reset_btn_ref = reset_btn.clone();
	reset_btn.connect_clicked(move |_| {
		entry.borrow_mut().reset();
		build_options_group(&options_group, &entry, &action_row, &reset_btn_ref);
		render_list_entry(&action_row, &entry.borrow(), None);
		list_box.unselect_all();
		list_box.select_row(Some(&list_row));
	});
}
fn add_user_label_row(options: &PreferencesGroup, entry: &GC<StabEntry>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	let row = EntryRow::builder()
		.title("Label")
		.text(entry.borrow().user_label.as_deref().unwrap_or(""))
		.build();
	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		row.connect_changed(move |row| {
			let text = row.text();
			let mut entry = entry.borrow_mut();
			entry.user_label = if text.is_empty() { None } else { Some(text.to_string()) };
			render_list_entry(&action_row, &entry, Some(&reset_btn));
		});
	}
	options.add(&row);
}

fn add_spin_row(
	options: &PreferencesGroup,
	entry: &GC<StabEntry>,
	action_row: &ActionRow,
	title: &str,
	initial: u8,
	reset_btn: &gtk::Button,
	apply: impl Fn(&mut StabEntry, u8) + 'static,
) {
	let entry = entry.clone();
	let action_row = action_row.clone();
	let reset_btn = reset_btn.clone();

	let adjustment = Adjustment::builder().value(f64::from(initial)).step_increment(1.0).build();

	let row = SpinRow::new(Some(&adjustment), 1.0, 0);
	row.set_title(title);
	row.set_range(0.0, 255.0);
	row.set_climb_rate(1.0);
	row.set_numeric(true);
	row.set_value(f64::from(initial));
	let row_ref = row.clone();
	row.adjustment().connect_value_changed(move |_| {
		let mut entry = entry.borrow_mut();
		apply(&mut entry, row_ref.value().round() as u8);
		render_list_entry(&action_row, &entry, Some(&reset_btn));
	});
	options.add(&row);
}
