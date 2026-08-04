mod stab_yurself;

use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{
	ActionRow, Application, ApplicationWindow, Breakpoint, BreakpointCondition, HeaderBar,
	LengthUnit,
};
use gtk::{Box as GtkBox, Button, Label, ListBox, Orientation, ScrolledWindow, SelectionMode};
use std::cell::Cell;
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
	
	let editor_panel = build_editor_panel();
	
	let list_panel = build_entry_list(&entries);
	let split_box = build_split_layout(&list_panel, &editor_panel);
	
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
	
	let content_scroll = ScrolledWindow::builder()
		.child(&content_box)
		.hexpand(true)
		.vexpand(true)
		.build();
	
	let main_box = GtkBox::builder().orientation(Orientation::Vertical).build();
	main_box.append(&HeaderBar::new());
	main_box.append(&content_scroll);
	
	let window = ApplicationWindow::builder()
		.application(application)
		.title("FSTabulator")
		.default_width(400)
		.default_height(300)
		.content(&main_box)
		.build();
	
	attach_responsive_breakpoint(&window, &split_box);
	
	window.present();
}

fn build_entry_list(entries: &[StabEntry]) -> ListBox {
	let list_box = ListBox::builder()
		.selection_mode(SelectionMode::Single)
		.css_classes(["boxed-list"])
		.hexpand(true)
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
		.orientation(Orientation::Horizontal) // default: side-by-side
		.spacing(20)
		//	.homogeneous(true)
		.build();
	
	split_box.append(list_panel);
	split_box.append(editor_panel);
	
	split_box
}

fn attach_responsive_breakpoint(window: &adw::ApplicationWindow, split_box: &GtkBox) {
	let condition = BreakpointCondition::new_length(
		adw::BreakpointConditionLengthType::MaxWidth,
		500.0,
		LengthUnit::Sp,
	);
	
	let breakpoint = Breakpoint::new(condition);
	breakpoint.add_setter(
		split_box,
		"orientation",
		Some(&Orientation::Vertical.to_value()),
	);
	
	window.add_breakpoint(breakpoint);
}

fn build_editor_panel() -> gtk::Box {
	let counter_label = Label::builder()
		.label("yo yo")
		.wrap(true)
		.margin_top(24)
		.margin_bottom(24)
		.build();
	
	let increment_button = Button::builder().label("Click me").build();
	
	{
		let counter_label = counter_label.clone();
		let click_count = Rc::new(Cell::new(0));
		increment_button.connect_clicked(move |_| {
			click_count.set(click_count.get() + 1);
			counter_label.set_label(&format!("Clicks: {}", click_count.get()));
		});
	}
	
	
	let editor_panel = GtkBox::builder()
		.orientation(Orientation::Vertical) // default: side-by-side
		.spacing(12)
		.build();
	
	editor_panel.append(&counter_label);
	editor_panel.append(&increment_button);
	
	editor_panel
}
