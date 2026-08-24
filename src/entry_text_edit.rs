use crate::context::EntryContext;
use crate::device_value::{DeviceKind, DeviceValue};
use crate::fs_options::{self, FsOption, OptionSpec, OptionValue};
use crate::fs_value::FsType;
use crate::i18n::{i18n, i18n_fmt};
use crate::stab_yurself::StabEntry;
use crate::ui_commons::{cancel_save_row, clear_children, close_on_click, dialog_content_box, dialog_heading, parent_window, suggested_dialog_width};
use adw::Dialog;
use adw::prelude::*;
use gtk::{Align, Box as GtkBox, Entry, Image, Orientation, Widget};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IssueKind {
	Ok,
	Warning,
	Error,
}

pub struct Issue {
	kind: IssueKind,
	message: String,
}

pub fn detect_issues(line: usize, text: &str) -> Vec<Issue> {
	let entry = match StabEntry::from(line, text) {
		Ok(entry) => entry,
		Err(err) => {
			return vec![Issue {
				kind: IssueKind::Error,
				message: format!("{}: {err}", i18n("Could not parse entry")),
			}];
		}
	};

	let mut issues = Vec::new();

	let mount_point = entry.mount_point.trim();
	if entry.fs_type != FsType::Swap && !mount_point.is_empty() && !Path::new(mount_point).is_dir() {
		issues.push(Issue {
			kind: IssueKind::Warning,
			message: i18n_fmt("Mount point does not exist: {point}", &[("{point}", mount_point)]),
		});
	}

	if let Some(message) = device_problem(&entry.device, &entry.fs_type) {
		issues.push(Issue {
			kind: IssueKind::Warning,
			message,
		});
	}

	if matches!(entry.fs_type, FsType::Other(_)) {
		issues.push(Issue {
			kind: IssueKind::Warning,
			message: i18n_fmt("Unknown file system: {fs_type}", &[("{fs_type}", &entry.fs_type.to_string())]),
		});
	} else {
		for option in &entry.options {
			let Some(spec) = fs_options::lookup(&entry.fs_type, option.name()) else {
				issues.push(Issue {
					kind: IssueKind::Warning,
					message: i18n_fmt("Unknown option: {option}", &[("{option}", option.name())]),
				});
				continue;
			};
			if let Some(message) = option_value_problem(spec, option) {
				issues.push(Issue {
					kind: IssueKind::Warning,
					message,
				});
			}
		}
	}

	if issues.is_empty() {
		issues.push(Issue {
			kind: IssueKind::Ok,
			message: i18n("No problems detected"),
		});
	}
	issues
}

fn device_problem(device: &DeviceValue, fs_type: &FsType) -> Option<String> {
	if device.value.trim().is_empty() {
		return None;
	}
	if device.kind == DeviceKind::Other && !matches!(fs_type, FsType::Other(_)) && !DeviceKind::for_fs_type(fs_type).is_empty() {
		return Some(i18n_fmt("Unknown device type: {device}", &[("{device}", &device.render())]));
	}
	let checkable = DeviceKind::LOCAL.contains(&device.kind);
	if !checkable || device.resolve_node().is_some() {
		return None;
	}
	Some(i18n_fmt("Device does not exist: {device}", &[("{device}", &device.render())]))
}

fn option_value_problem(spec: OptionSpec, option: &FsOption) -> Option<String> {
	let name = option.name();
	let value = match option {
		FsOption::Named(_) => None,
		FsOption::KeyValue(_, value) => Some(value),
	};
	match spec.value {
		OptionValue::Toggle => {
			if value.is_some() {
				Some(i18n_fmt("{option} should not have a value", &[("{option}", name)]))
			} else {
				None
			}
		}
		OptionValue::Enum(values) => value.filter(|value| values.contains(&value.as_str())).map_or_else(
			|| {
				Some(i18n_fmt(
					"{option} should be one of: {values}",
					&[("{option}", name), ("{values}", &values.join(", "))],
				))
			},
			|_| None,
		),
		OptionValue::Compression(algorithms) => {
			let valid = match value {
				None => true,
				Some(value) => algorithms.iter().any(|spec| spec.is_valid(value)),
			};
			if valid {
				None
			} else {
				let values: Vec<&str> = algorithms.iter().map(|spec| spec.name).collect();
				Some(i18n_fmt(
					"{option} should be one of: {values}, optionally followed by a level",
					&[("{option}", name), ("{values}", &values.join(", "))],
				))
			}
		}
		OptionValue::Integer => value
			.filter(|value| value.parse::<i64>().is_ok())
			.map_or_else(|| Some(i18n_fmt("{option} should be a number", &[("{option}", name)])), |_| None),
		OptionValue::IntegerRange(min, max) => value
			.and_then(|value| value.parse::<i64>().ok())
			.filter(|value| (min..=max).contains(value))
			.map_or_else(
				|| {
					Some(i18n_fmt(
						"{option} should be a number between {min} and {max}",
						&[("{option}", name), ("{min}", &min.to_string()), ("{max}", &max.to_string())],
					))
				},
				|_| None,
			),
		OptionValue::Octal => value
			.filter(|value| !value.is_empty() && value.chars().all(|c| matches!(c, '0'..='7')))
			.map_or_else(|| Some(i18n_fmt("{option} should be an octal number", &[("{option}", name)])), |_| None),
		OptionValue::Size => value.filter(|value| is_size_value(value)).map_or_else(
			|| {
				Some(i18n_fmt(
					"{option} should be a number with a size unit, for example 16K",
					&[("{option}", name)],
				))
			},
			|_| None,
		),
		OptionValue::Bool(bool_type) => {
			let (on, off) = bool_type.values();
			value
				.filter(|value| {
					let lowered = value.to_ascii_lowercase();
					lowered == on || lowered == off
				})
				.map_or_else(
					|| {
						Some(i18n_fmt(
							"{option} should be {on} or {off}",
							&[("{option}", name), ("{on}", on), ("{off}", off)],
						))
					},
					|_| None,
				)
		}
		OptionValue::String | OptionValue::Subvol => None,
	}
}

fn is_size_value(value: &str) -> bool {
	let num_len = value
		.char_indices()
		.find(|(_, c)| !c.is_ascii_digit() && *c != '.')
		.map(|(idx, _)| idx)
		.unwrap_or(value.len());
	let (number, unit) = value.split_at(num_len);
	!number.is_empty()
		&& number.chars().all(|c| c.is_ascii_digit() || c == '.')
		&& (unit.is_empty() || (unit.len() == 1 && "BKMGTPE%".contains(unit.to_ascii_uppercase().as_str())))
}

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
			let issues = detect_issues(line, input.text().as_str());
			clear_children(&issues_box);
			for issue in &issues {
				issues_box.append(&issue_row(issue));
			}
			save_btn.set_sensitive(issues.iter().all(|issue| issue.kind != IssueKind::Error));
		};
		refresh();
		text_input.connect_changed(move |_| refresh());
	}
	{
		let (text_input, entry, on_saved) = (text_input.clone(), entry.clone(), std::cell::RefCell::new(Some(on_saved)));
		let (dialog, refresh_parent) = (dialog.clone(), refresh_parent.clone());
		save_btn.connect_clicked(move |_| match StabEntry::from(line, text_input.text().as_str()) {
			Ok(parsed) => {
				let mut entry = entry.borrow_mut();
				entry.active = parsed.active;
				entry.device = parsed.device;
				entry.mount_point = parsed.mount_point;
				entry.fs_type = parsed.fs_type;
				entry.options = parsed.options;
				entry.dump = parsed.dump;
				entry.pass = parsed.pass;
				drop(entry);
				if let Some(on_saved) = on_saved.borrow_mut().take() {
					on_saved();
				}
				dialog.close();
			}
			Err(_) => dialog.present(refresh_parent.as_ref()),
		});
	}
	dialog.present(parent.as_ref());
}

fn issue_row(issue: &Issue) -> GtkBox {
	let (icon_name, css_class) = match issue.kind {
		IssueKind::Ok => ("emblem-ok-symbolic", "text-edit-ok"),
		IssueKind::Warning => ("dialog-warning-symbolic", "text-edit-warning"),
		IssueKind::Error => ("dialog-error-symbolic", "text-edit-error"),
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unparseable_text_is_an_error() {
		let issues = detect_issues(3, "not a valid entry");
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].kind, IssueKind::Error);
	}

	#[test]
	fn clean_entry_reports_ok() {
		let issues = detect_issues(0, "none / tmpfs defaults 0 0");
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].kind, IssueKind::Ok);
	}

	#[test]
	fn missing_mount_point_warns() {
		let issues = detect_issues(0, "none /no/such/mount/point tmpfs defaults 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("/no/such/mount/point"))
		);
	}

	#[test]
	fn swap_entry_skips_mount_point_check() {
		let issues = detect_issues(0, "/dev/zram0 none swap defaults 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("Mount point does not exist")));
	}

	#[test]
	fn missing_device_and_unknown_option_warn() {
		let issues = detect_issues(0, "UUID=12345678-1234-1234-1234-123456789abc /mnt/nope ext4 bogus_option 0 2");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("UUID=12345678"))
		);
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("bogus_option"))
		);
	}

	#[test]
	fn unknown_file_system_warns_and_skips_option_checks() {
		let issues = detect_issues(0, "none /mnt/x borkfs bogus_option 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("borkfs"))
		);
		assert!(!issues.iter().any(|issue| issue.message.contains("bogus_option")));
	}

	#[test]
	fn unknown_device_type_warns() {
		let issues = detect_issues(0, "LABEL=boot /mnt/x cifs defaults 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("LABEL=boot"))
		);
	}

	#[test]
	fn invalid_option_values_warn() {
		let issues = detect_issues(0, "none / tmpfs nofail=1,X-mount.nocanonicalize=wrong 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("nofail"))
		);
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("X-mount.nocanonicalize"))
		);
	}

	#[test]
	fn non_numeric_integer_option_warns() {
		let issues = detect_issues(0, "none /mnt/x ext2 resgid=abc 0 2");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("resgid"))
		);
	}

	#[test]
	fn compression_option_accepts_bare_algorithm_and_level() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:3 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:0 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
	}

	#[test]
	fn compression_option_rejects_out_of_range_level() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:16 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("compress"))
		);
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zlib:10 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("compress"))
		);
	}

	#[test]
	fn compression_option_rejects_level_for_levelless_algorithm() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=lzo:3 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("compress"))
		);
	}

	#[test]
	fn compression_option_rejects_unknown_algorithm() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=bork:3 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.kind == IssueKind::Warning && issue.message.contains("compress"))
		);
	}
}
