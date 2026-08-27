use adw::prelude::*;
use adw::{ActionRow, AlertDialog, Dialog};
use gtk::{Align, Box as GtkBox, Button, ListBox, Orientation, Widget};
use std::cell::RefCell;
use std::rc::Rc;

use crate::i18n::i18n;
use crate::problem_reports::{Problem, ProblemLevel};

pub(crate) fn trash_button(tooltip: &str) -> Button {
	let btn = Button::from_icon_name("user-trash-symbolic");
	btn.add_css_class("flat");
	btn.add_css_class("error");
	btn.set_valign(Align::Center);
	btn.set_tooltip_text(Some(tooltip));
	btn
}

pub(crate) const CHECKMARK_NAME: &str = "object-select-symbolic";
pub(crate) const WARNING_NAME: &str = "dialog-warning-symbolic";
pub(crate) const ERROR_NAME: &str = "dialog-error-symbolic";

pub(crate) fn issue_image() -> gtk::Image {
	let icon = gtk::Image::from_icon_name(CHECKMARK_NAME);
	icon.add_css_class("text-edit-ok");
	icon.set_valign(Align::Center);
	icon
}

pub(crate) fn update_issue_icon(icon: &gtk::Image, problem: Option<&Problem>) {
	let (name, class, message) = match problem {
		None => (CHECKMARK_NAME, "text-edit-ok", None),
		Some(problem) => match problem.level {
			ProblemLevel::Ok => (CHECKMARK_NAME, "text-edit-ok", None),
			ProblemLevel::Warning => (WARNING_NAME, "text-edit-warning", Some(problem.message.clone())),
			ProblemLevel::Error => (ERROR_NAME, "issue-error", Some(problem.message.clone())),
		},
	};
	icon.set_icon_name(Some(name));
	icon.set_tooltip_text(message.as_deref());
	for other in ["text-edit-ok", "text-edit-warning", "issue-error"] {
		if other != class {
			icon.remove_css_class(other);
		}
	}
	icon.add_css_class(class);
}

pub(crate) fn titled_header(title: &str, subtitle: Option<&str>, issue: Option<&gtk::Image>, suffix: &impl IsA<Widget>) -> GtkBox {
	let labels = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.spacing(3)
		.valign(Align::Center)
		.build();
	labels.append(&gtk::Label::builder().label(title).halign(Align::Start).wrap(true).build());
	if let Some(subtitle) = subtitle {
		labels.append(
			&gtk::Label::builder()
				.label(subtitle)
				.halign(Align::Start)
				.wrap(true)
				.css_classes(["subtitle"])
				.build(),
		);
	}
	let text = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(6)
		.margin_top(6)
		.valign(Align::Center)
		.hexpand(true)
		.build();
	if let Some(icon) = issue {
		text.append(icon);
	}
	text.append(&labels);
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
	dialog.add_response("ok", i18n("OK").as_str());
	dialog.set_default_response(Some("ok"));
	dialog.present(parent.as_ref());
}

pub fn present_bullet_dialog(widget: &impl IsA<Widget>, heading: &str, body: &str, bullets: &[String]) {
	let parent = parent_window(widget);
	let markup = bullets.iter().map(|bullet| format!("• {bullet}")).collect::<Vec<_>>().join("\n");
	let label = gtk::Label::builder().use_markup(true).label(&markup).wrap(true).xalign(0.0).build();
	let dialog = AlertDialog::builder().heading(heading).body(body).build();
	dialog.set_extra_child(Some(&label));
	dialog.add_response("ok", i18n("OK").as_str());
	dialog.set_default_response(Some("ok"));
	dialog.present(parent.as_ref());
}

pub struct ConfirmPopupBuilder {
	parent_widget: Widget,
	heading: Option<String>,
	confirm_choice: String,
	message: String,
	extra_child: Option<Widget>,
	on_confirm: Box<dyn FnOnce()>,
}

impl ConfirmPopupBuilder {
	pub fn heading(mut self, value: impl Into<String>) -> Self {
		self.heading = Some(value.into());
		self
	}

	pub fn confirm_choice(mut self, value: impl Into<String>) -> Self {
		self.confirm_choice = value.into();
		self
	}

	pub fn extra_child(mut self, value: &impl IsA<Widget>) -> Self {
		self.extra_child = Some(value.clone().upcast());
		self
	}

	pub fn present(self) {
		let Self {
			parent_widget,
			heading,
			confirm_choice,
			message,
			extra_child,
			on_confirm,
		} = self;
		let mut dialog_builder = AlertDialog::builder().body(&message);
		if let Some(heading) = &heading {
			dialog_builder = dialog_builder.heading(heading.as_str());
		}
		let dialog = dialog_builder.build();
		if let Some(child) = extra_child {
			dialog.set_extra_child(Some(&child));
		}
		dialog.add_response("cancel", i18n("Cancel").as_str());
		dialog.add_response("confirm", confirm_choice.as_str());
		dialog.set_default_response(Some("cancel"));
		dialog.set_close_response("cancel");
		let parent = parent_window(&parent_widget);
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
}

pub(crate) fn parent_window(widget: &impl IsA<Widget>) -> Option<gtk::Window> {
	widget.root().and_then(|root| root.downcast::<gtk::Window>().ok())
}

pub(crate) const DIALOG_MARGIN: i32 = 16;

pub(crate) fn dialog_content_box() -> GtkBox {
	GtkBox::builder()
		.orientation(Orientation::Vertical)
		.spacing(6)
		.margin_start(DIALOG_MARGIN)
		.margin_end(DIALOG_MARGIN)
		.margin_top(DIALOG_MARGIN)
		.margin_bottom(DIALOG_MARGIN)
		.build()
}

pub(crate) fn close_on_click(btn: &Button, dialog: &Dialog) {
	let dialog = dialog.clone();
	btn.connect_clicked(move |_| {
		dialog.close();
	});
}

pub(crate) fn dialog_heading(title: impl Into<String>) -> gtk::Label {
	gtk::Label::builder()
		.label(title.into())
		.css_classes(["title-1"])
		.halign(Align::Start)
		.build()
}

pub(crate) fn cancel_save_row() -> (Button, Button, GtkBox) {
	let cancel_btn = Button::with_label(i18n("Cancel").as_str());
	let save_btn = Button::with_label(i18n("Save").as_str());
	save_btn.add_css_class("suggested-action");
	let row = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(6)
		.halign(Align::End)
		.build();
	row.append(&cancel_btn);
	row.append(&save_btn);
	(cancel_btn, save_btn, row)
}

pub(crate) fn suggested_dialog_width(widget: &impl IsA<Widget>) -> i32 {
	parent_window(widget)
		.as_ref()
		.map(|window| window.width() * 9 / 10)
		.filter(|width| *width > 0)
		.unwrap_or(600)
}

pub fn confirm_popup(parent_widget: &impl IsA<Widget>, message: impl Into<String>, on_confirm: impl FnOnce() + 'static) -> ConfirmPopupBuilder {
	ConfirmPopupBuilder {
		parent_widget: parent_widget.clone().upcast(),
		heading: None,
		confirm_choice: i18n("Confirm"),
		message: message.into(),
		extra_child: None,
		on_confirm: Box::new(on_confirm),
	}
}

pub fn confirm_clicked_action(button: &gtk::Button, message: impl Into<String>) -> ConfirmActionBuilder {
	ConfirmActionBuilder {
		button: button.clone(),
		confirm_choice: i18n("Confirm"),
		message: message.into(),
		guard: RefCell::new(Box::new(|| true)),
		extra_child: RefCell::new(Box::new(|| None)),
	}
}

pub struct ConfirmActionBuilder {
	button: Button,
	confirm_choice: String,
	message: String,
	guard: RefCell<Box<dyn FnMut() -> bool>>,
	extra_child: RefCell<Box<dyn FnMut() -> Option<gtk::Widget>>>,
}

impl ConfirmActionBuilder {
	pub fn guard(mut self, value: impl FnMut() -> bool + 'static) -> Self {
		self.guard = RefCell::new(Box::new(value));
		self
	}

	pub fn confirm_choice(mut self, value: impl Into<String>) -> Self {
		self.confirm_choice = value.into();
		self
	}

	pub fn extra_child(mut self, value: impl FnMut() -> Option<gtk::Widget> + 'static) -> Self {
		self.extra_child = RefCell::new(Box::new(value));
		self
	}

	pub fn connect(self, on_confirm: impl FnMut() + 'static) {
		let Self {
			button,
			confirm_choice,
			message,
			guard,
			extra_child,
		} = self;
		let on_confirm = Rc::new(RefCell::new(on_confirm));
		button.clone().connect_clicked(move |_| {
			if !guard.borrow_mut()() {
				return;
			}
			let extra_child = extra_child.borrow_mut()();
			let mut popup = confirm_popup(&button, message.clone(), {
				let on_confirm = on_confirm.clone();
				move || on_confirm.borrow_mut()()
			})
			.confirm_choice(confirm_choice.clone());
			if let Some(extra_child) = extra_child {
				popup = popup.extra_child(&extra_child);
			}
			popup.present();
		});
	}
}
