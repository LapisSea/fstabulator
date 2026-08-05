mod stab_yurself;

use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, EntryRow, HeaderBar, LengthUnit, PreferencesGroup};
use gtk::{Align, Box as GtkBox, Button, Label, ListBox, Orientation, ScrolledWindow, SelectionMode, Text, Widget};
use std::cell::Cell;
use std::fmt::format;
use std::rc::Rc;

const APP_ID: &str = "org.lapissea.FSTabulator";

fn main() -> gtk::glib::ExitCode {
	let application = Application::builder().application_id(APP_ID).build();
	application.connect_activate(build_ui);
	application.run()
}

fn build_ui(application: &Application) {
	let entries: Vec<_> = stab_yurself::read_fstab()
		.unwrap()
		.into_iter()
		.filter_map(|e| e.ok())
		.collect();
	
	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.vexpand(true)
		.hexpand(true)
		.spacing(12)
		.build();
	
	let list_panel = build_entry_list(&entries);
	let split_box = build_split_layout(&wrap_scroll(&list_panel), &wrap_scroll(&editor_panel));
	
	list_panel.connect_row_selected(move |_, row| {
		while let Some(child) = editor_panel.last_child() {
			editor_panel.remove(&child);
		}
		let Some(row) = row else { return };
		if row.index() < 0 { return; }
		let Some(entry) = entries.get(row.index() as usize) else { return };
		build_editor_panel(&editor_panel, entry);
	});
	
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
	
	window.present();
}

fn wrap_scroll(content: &impl IsA<Widget>) -> ScrolledWindow {
	ScrolledWindow::builder()
		.child(content)
		.hexpand(true)
		.vexpand(true)
		.build()
}

fn build_entry_list(entries: &[StabEntry]) -> ListBox {
	let list_box = ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
		.valign(Align::Start)
		.build();
	let mut first = true;
	for entry in entries {
		let row = ActionRow::builder()
			.title(format!("Line {}", entry.line))
			.subtitle(&entry.to_string())
			.build();
		list_box.append(&row);
		if first {
			first = false;
			list_box.select_row(Some(&row));
		}
	}
	
	list_box
}

fn build_split_layout(
	list_panel: &impl IsA<gtk::Widget>,
	editor_panel: &impl IsA<gtk::Widget>,
) -> GtkBox {
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
	let condition = BreakpointCondition::new_length(
		adw::BreakpointConditionLengthType::MaxWidth,
		600.0,
		LengthUnit::Sp,
	);
	
	let breakpoint = Breakpoint::new(condition);
	breakpoint.add_setter(
		split_box,
		"orientation",
		Some(&Orientation::Vertical.to_value()),
	);
	breakpoint.add_setter(
		split_box,
		"homogeneous",
		Some(&false.to_value()),
	);
	
	window.add_breakpoint(breakpoint);
}

fn build_editor_panel(editor_panel: &gtk::Box, entry: &StabEntry) {
	let options = PreferencesGroup::builder()
		.title("Edit properties")
		.build();
	
	editor_panel.append(&options);
	
	options.add(&EntryRow::builder()
		.title("Device").text(&entry.device)
		.build());
	
	options.add(&EntryRow::builder()
		.title("Mount point").text(&entry.mount_point)
		.build());
	
	options.add(&EntryRow::builder()
		.title("File system").text(&entry.fs_type)
		.build());
	
	
	for (i, val) in entry.options.iter().enumerate() {
		options.add(&EntryRow::builder()
			.title(format!("Option {i}: ")).text(val)
			.build());
	}
	
	options.add(&EntryRow::builder()
		.title("Dump").text(&entry.dump.to_string())
		.build());
	
	options.add(&EntryRow::builder()
		.title("Pass").text(&entry.pass.to_string())
		.build());
	
}
