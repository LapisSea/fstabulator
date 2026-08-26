use crate::context::EntryContext;
use crate::i18n::i18n;
use crate::problem_reports::{Problem, ProblemLevel, check_subvols_in_text, detect_issues};
use crate::stab_yurself::StabEntry;
use crate::ui_commons::{
	CHECKMARK_NAME, ERROR_NAME, WARNING_NAME, cancel_save_row, clear_children, close_on_click, dialog_content_box, dialog_heading, parent_window,
	suggested_dialog_width,
};
use adw::Dialog;
use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Button, Entry, Image, Orientation, Widget};

pub fn present(parent: &impl IsA<Widget>, entry_ctx: EntryContext, on_saved: impl FnOnce() + 'static) {
	let (entry, initial, line) = {
		let entry_ref = entry_ctx.entry().borrow();
		(entry_ctx.entry().clone(), entry_ref.to_string(), entry_ref.line)
	};

	let text_input = Entry::builder()
		.text(initial.as_str())
		.css_classes(["monospace"])
		.width_chars(40)
		.hexpand(true)
		.build();
	let issues_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).margin_top(12).build();

	let heading = dialog_heading(i18n("Edit as text"));
	let body = gtk::Label::builder()
		.label(i18n("Edit the raw fstab line. Problems with its values are listed below."))
		.halign(Align::Start)
		.wrap(true)
		.build();
	let (cancel_btn, save_btn, buttons) = cancel_save_row();

	let content = dialog_content_box();
	content.append(&heading);
	content.append(&body);
	content.append(&text_input);
	content.append(&issues_box);
	content.append(&buttons);

	let width = suggested_dialog_width(parent);
	let parent = parent_window(parent);
	let dialog = Dialog::builder().child(&content).follows_content_size(true).width_request(width).build();
	if let Some(window) = &parent
		&& let Some(surface) = window.surface()
	{
		let (window, dialog) = (window.clone(), dialog.clone());
		surface.connect_width_notify(move |_| {
			dialog.set_width_request(suggested_dialog_width(&window));
		});
	}

	close_on_click(&cancel_btn, &dialog);

	let refresh_parent = parent.clone();
	{
		let (issues_box, save_btn, input) = (issues_box.clone(), save_btn.clone(), text_input.clone());
		let refresh = move || {
			render_issues(&issues_box, &save_btn, &detect_issues(line, input.text().as_str()));
		};
		refresh();
		text_input.connect_changed(move |_| refresh());
	}
	{
		let (text_input, entry, on_saved) = (text_input.clone(), entry.clone(), std::cell::RefCell::new(Some(on_saved)));
		let (dialog, refresh_parent) = (dialog.clone(), refresh_parent.clone());
		save_btn.connect_clicked(move |_| {
			let text = text_input.text();
			let Ok(parsed) = StabEntry::from(line, text.as_str()) else {
				dialog.present(refresh_parent.as_ref());
				return;
			};
			let mut issues = detect_issues(line, text.as_str());
			issues.extend(check_subvols_in_text(line, text.as_str()));
			{
				let mut entry = entry.borrow_mut();
				entry.active = parsed.active;
				entry.device = parsed.device;
				entry.mount_point = parsed.mount_point;
				entry.fs_type = parsed.fs_type;
				entry.options = parsed.options;
				entry.dump = parsed.dump;
				entry.pass = parsed.pass;
			}
			if let Some(on_saved) = on_saved.borrow_mut().take() {
				on_saved();
			}
			dialog.close();
		});
	}
	dialog.present(parent.as_ref());
}

fn render_issues(issues_box: &GtkBox, save_btn: &Button, issues: &[Problem]) {
	clear_children(issues_box);
	for issue in issues {
		issues_box.append(&issue_row(issue));
	}
	let action = if issues.iter().any(|issue| issue.level == ProblemLevel::Error) {
		"destructive-action"
	} else {
		"suggested-action"
	};
	save_btn.set_css_classes(&[action]);
}

fn issue_row(issue: &Problem) -> GtkBox {
	let (icon_name, css_class) = match issue.level {
		ProblemLevel::Ok => (CHECKMARK_NAME, "text-edit-ok"),
		ProblemLevel::Warning => (WARNING_NAME, "text-edit-warning"),
		ProblemLevel::Error => (ERROR_NAME, "issue-error"),
	};
	let icon = Image::from_icon_name(icon_name);
	icon.add_css_class(css_class);
	icon.set_valign(Align::Center);
	icon.set_margin_end(6);
	let label = gtk::Label::builder().label(&issue.message).halign(Align::Start).wrap(true).build();
	let row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
	row.append(&icon);
	row.append(&label);
	row
}
