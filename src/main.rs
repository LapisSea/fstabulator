mod device_value;
mod fs_value;
mod stab_yurself;

use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, EntryRow, HeaderBar, LengthUnit, PreferencesGroup, SpinRow};
use gtk::{Adjustment, Align, Box as GtkBox, ListBox, Orientation, ScrolledWindow, SelectionMode, Widget};
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
	//provider.load_from_data("row.changed { background-color: rgba(255, 180, 0, 0.2); }");
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

pub(crate) fn render_list_entry(action_row: &ActionRow, entry: &StabEntry, reset_btn: Option<&gtk::Button>) {
	action_row.set_title(&format!("Line {}", entry.line));
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
	let condition = BreakpointCondition::new_length(adw::BreakpointConditionLengthType::MaxWidth, 600.0, LengthUnit::Sp);

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

	let options = PreferencesGroup::builder().title("Edit properties").build();

	editor_panel.append(&options);
	
	device_value::add_device_row(&options, entry, action_row, &reset_btn);
	add_editable_row(
		&options,
		entry,
		action_row,
		"Mount point",
		&entry.borrow().mount_point,
		&reset_btn,
		|entry, value| {
			entry.mount_point = value.to_string();
			true
		},
	);
	fs_value::add_fs_type_row(&options, entry, action_row, &reset_btn);
	for (i, val) in entry.borrow().options.iter().enumerate() {
		let apply = move |entry: &mut StabEntry, value: &str| {
			entry.options[i] = value.to_string();
			true
		};
		add_editable_row(&options, entry, action_row, &format!("Option {i}"), val, &reset_btn, apply);
	}
	
	add_spin_row(&options, entry, action_row, "Dump", entry.borrow().dump, &reset_btn, |entry, value| {
		entry.dump = value
	});
	add_spin_row(&options, entry, action_row, "Pass", entry.borrow().pass, &reset_btn, |entry, value| {
		entry.pass = value
	});
	
	editor_panel.append(&reset_btn);
	
	let entry = entry.clone();
	let action_row = action_row.clone();
	let list_box = list_box.clone();
	let list_row = list_row.clone();
	reset_btn.connect_clicked(move |_| {
		entry.borrow_mut().reset();
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

fn add_editable_row(
	options: &PreferencesGroup,
	entry: &Rc<RefCell<StabEntry>>,
	action_row: &ActionRow,
	title: &str,
	initial: &str,
	reset_btn: &gtk::Button,
	apply: impl Fn(&mut StabEntry, &str) -> bool + 'static,
) {
	let entry = entry.clone();
	let action_row = action_row.clone();
	let reset_btn = reset_btn.clone();
	let row = EntryRow::builder().title(title).text(initial).build();
	row.connect_changed(move |row| {
		let mut entry = entry.borrow_mut();
		if !apply(&mut entry, &row.text()) {
			return;
		}
		render_list_entry(&action_row, &entry, Some(&reset_btn));
	});
	options.add(&row);
}
