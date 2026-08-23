use adw::ActionRow;
use adw::AlertDialog;
use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Button, ListBox, Orientation, Widget};

use std::cell::RefCell;
use std::rc::Rc;
pub(crate) fn trash_button(tooltip: &str) -> Button {
	let btn = Button::from_icon_name("user-trash-symbolic");
	btn.add_css_class("flat");
	btn.add_css_class("error");
	btn.set_valign(Align::Center);
	btn.set_tooltip_text(Some(tooltip));
	btn
}

pub(crate) fn titled_header(title: &str, subtitle: Option<&str>, suffix: &impl IsA<Widget>) -> GtkBox {
	let text = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.margin_top(6)
		.spacing(3)
		.valign(Align::Center)
		.hexpand(true)
		.build();
	text.append(&gtk::Label::builder().label(title).halign(Align::Start).wrap(true).build());
	if let Some(subtitle) = subtitle {
		text.append(
			&gtk::Label::builder()
				.label(subtitle)
				.halign(Align::Start)
				.wrap(true)
				.css_classes(["subtitle"])
				.build(),
		);
	}
	let header = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(6)
		.valign(Align::Center)
		.margin_start(12)
		.margin_end(12)
		.build();
	header.set_size_request(-1, 50);
	header.append(&text);
	header.append(suffix);
	header
}

pub(crate) fn activatable_row(title: impl Into<gtk::glib::GString>, subtitle: impl Into<gtk::glib::GString>) -> ActionRow {
	let row = ActionRow::builder().title(title).subtitle(subtitle).build();
	row.set_activatable(true);
	row
}

pub(crate) fn query_matches(query: &str, field: &str) -> bool {
	let query = query.trim().to_lowercase();
	query.is_empty() || field.to_lowercase().contains(&query)
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

pub(crate) fn find_widget_with_class<W: IsA<Widget>>(widget: W, class: &str) -> Option<gtk::Widget> {
	if widget.has_css_class(class) {
		return Some(widget.upcast());
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

pub fn present_simple_dialog(widget: &impl IsA<Widget>, heading: &str, body: &str) {
	let parent = parent_window(widget);
	let dialog = AlertDialog::builder().heading(heading).body(body).build();
	dialog.add_response("ok", "OK");
	dialog.set_default_response(Some("ok"));
	dialog.present(parent.as_ref());
}

pub fn present_bullet_dialog(widget: &impl IsA<Widget>, heading: &str, body: &str, bullets: &[String]) {
	let parent = parent_window(widget);
	let markup = bullets.iter().map(|bullet| format!("• {bullet}")).collect::<Vec<_>>().join("\n");
	let label = gtk::Label::builder().use_markup(true).label(&markup).wrap(true).xalign(0.0).build();
	let dialog = AlertDialog::builder().heading(heading).body(body).build();
	dialog.set_extra_child(Some(&label));
	dialog.add_response("ok", "OK");
	dialog.set_default_response(Some("ok"));
	dialog.present(parent.as_ref());
}

pub fn confirm_popup(
	parent_widget: &impl IsA<gtk::Widget>,
	heading: Option<&str>,
	confirm_choice: &str,
	message: &str,
	extra_child: Option<&impl IsA<gtk::Widget>>,
	on_confirm: impl FnOnce() + 'static,
) {
	let dialog = AlertDialog::builder().body(message).build();
	if let Some(heading) = heading {
		dialog.set_heading(Some(heading));
	}
	if let Some(child) = extra_child {
		dialog.set_extra_child(Some(child));
	}
	dialog.add_response("cancel", "Cancel");
	dialog.add_response("confirm", confirm_choice);
	dialog.set_default_response(Some("cancel"));
	dialog.set_close_response("cancel");
	let parent = parent_window(parent_widget);
	let on_confirm = RefCell::new(Some(on_confirm));
	dialog.connect_response(None, move |_, response| {
		if response == "confirm"
			&& let Some(on_confirm) = on_confirm.borrow_mut().take()
		{
			on_confirm();
		}
	});
	dialog.present(parent.as_ref());
}

pub(crate) fn parent_window(widget: &impl IsA<Widget>) -> Option<gtk::Window> {
	widget.root().and_then(|root| root.downcast::<gtk::Window>().ok())
}

pub fn connect_clicked_confirm(
	button: &gtk::Button,
	confirm_choice: &'static str,
	message: &'static str,
	extra_child: impl FnMut() -> Option<gtk::Widget> + 'static,
	on_confirm: impl FnMut() + 'static,
) {
	let button_click = button.clone();
	let extra_child = RefCell::new(extra_child);
	let on_confirm = Rc::new(RefCell::new(on_confirm));
	button.connect_clicked(move |_| {
		let extra_child = extra_child.borrow_mut()();
		confirm_popup(&button_click, None, confirm_choice, message, extra_child.as_ref(), {
			let on_confirm = on_confirm.clone();
			move || on_confirm.borrow_mut()()
		});
	});
}
