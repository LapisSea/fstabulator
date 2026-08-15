use adw::AlertDialog;
use adw::prelude::*;
use gtk::Widget;
use std::cell::RefCell;
use std::rc::Rc;

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
	confirm_choice: &str,
	message: &str,
	extra_child: Option<&impl IsA<gtk::Widget>>,
	on_confirm: impl FnOnce() + 'static,
) {
	let dialog = AlertDialog::builder().body(message).build();
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
		if response == "confirm" {
			if let Some(on_confirm) = on_confirm.borrow_mut().take() {
				on_confirm();
			}
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
		confirm_popup(&button_click, confirm_choice, message, extra_child.as_ref(), {
			let on_confirm = on_confirm.clone();
			move || on_confirm.borrow_mut()()
		});
	});
}
