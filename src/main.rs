mod device_value;
mod fs_options;
mod fs_value;
mod mount_point_value;
mod options_value;
mod stab_yurself;

use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, HeaderBar, LengthUnit, PreferencesGroup, SpinRow};
use gtk::{Adjustment, Align, Box as GtkBox, Image, ListBox, MenuButton, Orientation, Popover, ScrolledWindow, SearchEntry, SelectionMode, Widget};
use options_value::build_options_group;
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "org.lapissea.FSTabulator";

fn main() -> gtk::glib::ExitCode {
	let application = Application::builder().application_id(APP_ID).build();
	application.connect_activate(build_ui);
	application.run()
}

fn build_ui(application: &Application) {
	let entries: Vec<Rc<RefCell<StabEntry>>> = stab_yurself::read_fstab()
		.unwrap()
		.into_iter()
		.filter_map(|e| e.ok())
		.map(RefCell::new)
		.map(Rc::new)
		.collect();

	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();

	let list_panel = build_entry_list(&entries);
	let split_box = build_split_layout(&wrap_scroll(&list_panel), &wrap_scroll(&editor_panel));

	{
		let editor_panel = editor_panel.clone();
		let list_panel_cb = list_panel.clone();
		list_panel.connect_row_selected(move |_, row| {
			while let Some(child) = editor_panel.last_child() {
				editor_panel.remove(&child);
			}
			let Some(row) = row else { return };
			if row.index() < 0 {
				return;
			}
			let Some(entry) = entries.get(row.index() as usize) else { return };
			let Ok(action_row) = row.clone().downcast::<ActionRow>() else { return };
			build_editor_panel(&editor_panel, entry, &action_row, &list_panel_cb, &row);
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

	let window = ApplicationWindow::builder()
		.application(application)
		.title("FSTabulator")
		.default_width(800)
		.default_height(600)
		.content(&main_box)
		.build();

	attach_responsive_breakpoint(&window, &split_box);

	let provider = gtk::CssProvider::new();
	gtk::style_context_add_provider_for_display(
		&gtk::prelude::RootExt::display(&window),
		&provider,
		gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
	);

	window.present();
}

fn wrap_scroll(content: &impl IsA<Widget>) -> ScrolledWindow {
	ScrolledWindow::builder().child(content).hexpand(true).vexpand(true).build()
}

pub(crate) fn clear_list(list: &ListBox) {
	while let Some(row) = list.row_at_index(0) {
		list.remove(&row);
	}
}

pub(crate) struct SearchPicker {
	pub menu_btn: MenuButton,
	pub popover: Popover,
	pub list_box: ListBox,
}

pub(crate) fn build_search_picker(
	search_placeholder: &str,
	menu_label: &str,
	tooltip: &str,
	populate: impl Fn(&ListBox, &str) + 'static,
) -> SearchPicker {
	let search = SearchEntry::builder().placeholder_text(search_placeholder).hexpand(true).build();
	let list_box = ListBox::builder().css_classes(["boxed-list"]).hexpand(true).valign(Align::Start).build();
	let scroll = ScrolledWindow::builder()
		.child(&list_box)
		.max_content_height(240)
		.max_content_width(360)
		.propagate_natural_height(true)
		.hexpand(true)
		.build();

	let popover_content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).build();
	popover_content.append(&search);
	popover_content.append(&scroll);

	let popover = Popover::builder().child(&popover_content).build();

	let menu_btn = MenuButton::builder().label(menu_label).popover(&popover).build();
	menu_btn.set_tooltip_text(Some(tooltip));

	let populate = Rc::new(populate);
	{
		let list_box = list_box.clone();
		let search = search.clone();
		let menu_btn = menu_btn.clone();
		let populate = populate.clone();
		popover.connect_visible_notify(move |popover| {
			if popover.is_visible() {
				popover.set_size_request(menu_btn.width(), -1);
				populate(&list_box, "");
				search.set_text("");
				search.grab_focus();
			}
		});
	}
	{
		let list_box = list_box.clone();
		let populate = populate.clone();
		search.connect_search_changed(move |search| {
			populate(&list_box, &search.text());
		});
	}

	SearchPicker { menu_btn, popover, list_box }
}

fn render_list_entry_title(entry: &StabEntry) -> String {
	format!("Line {}", entry.line + 1)
}

pub(crate) fn render_list_entry(action_row: &ActionRow, entry: &StabEntry, reset_btn: Option<&gtk::Button>) {
	action_row.set_title(&render_list_entry_title(entry));
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
	update_nofail_warning(action_row, entry);
}

const NOFAIL_WARNING_CLASS: &str = "nofail-warning";

fn update_nofail_warning(action_row: &ActionRow, entry: &StabEntry) {
	if let Some(warning) = find_widget_with_class(action_row.clone().upcast(), NOFAIL_WARNING_CLASS) {
		action_row.remove(&warning);
	}
	if entry.has_option("nofail") {
		return;
	}
	let warning = Image::from_icon_name("dialog-warning-symbolic");
	warning.add_css_class(NOFAIL_WARNING_CLASS);
	warning.set_valign(Align::Center);
	warning.set_tooltip_text(Some(
		"The system may refuse to boot without this drive. If this is not intended, add the 'nofail' option. \n\
		Usually, this is wanted on root and home mounts.",
	));
	action_row.add_suffix(&warning);
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
	let original = esc(&entry.original().to_string());
	format!("<tt><i>- {original}</i>\n<b>+ {current}</b></tt>")
}

fn esc(s: &str) -> String {
	s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn build_entry_list(entries: &[Rc<RefCell<StabEntry>>]) -> ListBox {
	let list_box = ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
		.valign(Align::Start)
		.build();
	let mut first = true;
	for entry in entries {
		let entry = entry.borrow();
		let row = ActionRow::new();
		render_list_entry(&row, &entry, None);
		list_box.append(&row);
		if first {
			first = false;
			list_box.select_row(Some(&row));
		}
	}

	list_box
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
	entry: &Rc<RefCell<StabEntry>>,
	action_row: &ActionRow,
	list_box: &ListBox,
	list_row: &gtk::ListBoxRow,
) {
	let reset_btn = gtk::Button::with_label("Reset");
	reset_btn.set_sensitive(entry.borrow().is_changed());

	let edit_props = PreferencesGroup::builder().title("Edit properties").build();
	editor_panel.append(&edit_props);

	let options_group = PreferencesGroup::builder().title("Options").build();
	editor_panel.append(&options_group);

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
fn add_spin_row(
	options: &PreferencesGroup,
	entry: &Rc<RefCell<StabEntry>>,
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
