use crate::context::EntryContext;
use crate::fs_value::FsType;
use crate::i18n::{i18n, i18n_fmt};
use crate::mount_status::MountStatus;
use crate::options_value::build_options_group;
use crate::stab_yurself::StabEntry;
use crate::{GC, RebuildEditor, credentials_flow, device_value, entry_text_edit, fs_value, mount_point_value, mount_status, privileged, ui_commons};
use adw::prelude::*;
use adw::{EntryRow, PreferencesGroup, PreferencesRow, SpinRow, SwitchRow};
use gtk::{Adjustment, Align, Box as GtkBox, Button, ListBox, Orientation};
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Copy)]
enum MountAction {
	Mount,
	Remount,
	Unmount,
}

impl MountAction {
	fn cannot_heading(self) -> String {
		match self {
			MountAction::Mount => i18n("Cannot mount"),
			MountAction::Remount => i18n("Cannot remount"),
			MountAction::Unmount => i18n("Cannot unmount"),
		}
	}

	fn unsaved_changes_body(self) -> String {
		match self {
			MountAction::Mount => i18n("The entry has unsaved changes. Save your changes before mounting."),
			MountAction::Remount => i18n("The entry has unsaved changes. Save your changes before remounting."),
			MountAction::Unmount => i18n("The mount point has unsaved changes. Save your changes before unmounting."),
		}
	}
}

fn warn_empty_mount_point(btn: &Button, action: MountAction, mount_point: &str, exempt: bool) -> bool {
	if !exempt && mount_point.trim().is_empty() {
		ui_commons::present_simple_dialog(btn, action.cannot_heading().as_str(), i18n("The mount point is empty.").as_str());
		return true;
	}
	false
}

fn warn_unsaved_changes(btn: &Button, action: MountAction, changed: bool) -> bool {
	if changed {
		ui_commons::present_simple_dialog(btn, action.cannot_heading().as_str(), action.unsaved_changes_body().as_str());
		return true;
	}
	false
}

fn mount_action_allowed(btn: &Button, action: MountAction, mount_point: &str, exempt_empty: bool, changed: bool) -> bool {
	!warn_empty_mount_point(btn, action, mount_point, exempt_empty) && !warn_unsaved_changes(btn, action, changed)
}

pub(crate) fn build_editor_panel(
	editor_panel: &gtk::Box,
	entry_ctx: &EntryContext,
	list_box: &ListBox,
	list_row: &gtk::ListBoxRow,
	rebuild_editor: RebuildEditor,
) {
	let reset_btn = Button::with_label(i18n("Reset").as_str());
	reset_btn.add_css_class("destructive-action");
	reset_btn.set_sensitive(entry_ctx.entry().borrow().is_changed());
	entry_ctx.set_reset_btn(&reset_btn);

	let edit_props = PreferencesGroup::builder().title(i18n("Edit properties")).build();
	editor_panel.append(&edit_props);

	let options_group = PreferencesGroup::builder().title(i18n("Options")).build();
	editor_panel.append(&options_group);

	let fsck_group = PreferencesGroup::builder().title(i18n("Extra")).build();
	refresh_fsck_group_visibility(&fsck_group, entry_ctx.entry());

	add_user_label_row(&edit_props, entry_ctx);
	let device_row = device_value::add_device_row(&edit_props, entry_ctx);
	let mount_point_row = mount_point_value::add_mount_point_row(&edit_props, entry_ctx);
	{
		let (entry_ctx, device_row, options_group) = (entry_ctx.clone(), device_row.clone(), options_group.clone());
		let (mount_point_row, fsck_group) = (mount_point_row.clone(), fsck_group.clone());
		fs_value::add_fs_type_row(&edit_props.clone(), &entry_ctx.clone(), {
			move || {
				device_row.refresh_kinds();
				mount_point_row.refresh();
				refresh_fsck_group_visibility(&fsck_group, entry_ctx.entry());
				build_options_group(&options_group, &entry_ctx);
			}
		});
	}

	let active_row = SwitchRow::builder()
		.title(i18n("Active"))
		.active(entry_ctx.entry().borrow().active)
		.build();
	{
		let entry_ctx = entry_ctx.clone();
		active_row.connect_active_notify(move |row| {
			entry_ctx.entry().borrow_mut().active = row.is_active();
			entry_ctx.render();
		});
	}
	edit_props.add(&active_row);

	build_options_group(&options_group, entry_ctx);

	let text = gtk::Label::builder().label(i18n("Edit as text")).wrap(true).hexpand(true).build();
	let text_edit_btn = Button::builder().child(&text).build();

	{
		let (popup_ctx, saved_ctx, device_row) = (entry_ctx.clone(), entry_ctx.clone(), device_row.clone());
		let (options_group, list_box, list_row) = (options_group.clone(), list_box.clone(), list_row.clone());
		let mount_point_row = mount_point_row.clone();
		text_edit_btn.connect_clicked(move |btn| {
			let (saved_ctx, device_row, options_group) = (saved_ctx.clone(), device_row.clone(), options_group.clone());
			let (list_box, list_row) = (list_box.clone(), list_row.clone());
			let mount_point_row = mount_point_row.clone();
			entry_text_edit::present(btn, popup_ctx.clone(), move || {
				refresh_entry_editor(&saved_ctx, &device_row, &options_group, &mount_point_row, &list_box, &list_row);
			});
		});
	}
	let button_row = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(12)
		.homogeneous(true)
		.build();
	button_row.append(&text_edit_btn);
	button_row.append(&reset_btn);
	editor_panel.append(&button_row);

	add_mount_group(editor_panel, entry_ctx.entry(), &mount_point_row, rebuild_editor);

	add_spin_row(
		&fsck_group,
		entry_ctx,
		i18n("Dump").as_str(),
		i18n("Controls the dump backup frequency; 0 disables").as_str(),
		entry_ctx.entry().borrow().dump,
		|entry, value| entry.dump = value,
	);
	add_spin_row(
		&fsck_group,
		entry_ctx,
		i18n("Pass").as_str(),
		i18n("Controls the fsck check order; 0 disables").as_str(),
		entry_ctx.entry().borrow().pass,
		|entry, value| entry.pass = value,
	);
	editor_panel.append(&fsck_group);

	let (list_box, list_row) = (list_box.clone(), list_row.clone());
	let (options_group, device_row, entry_ctx_ref) = (options_group.clone(), device_row.clone(), entry_ctx.clone());
	let mount_point_row = mount_point_row.clone();
	reset_btn.connect_clicked(move |_| {
		entry_ctx_ref.entry().borrow_mut().reset();
		refresh_entry_editor(&entry_ctx_ref, &device_row, &options_group, &mount_point_row, &list_box, &list_row);
	});
}

fn refresh_entry_editor(
	entry_ctx: &EntryContext,
	device_row: &device_value::DeviceRowController,
	options_group: &PreferencesGroup,
	mount_point_row: &mount_point_value::MountPointRow,
	list_box: &ListBox,
	list_row: &gtk::ListBoxRow,
) {
	device_row.refresh_kinds();
	build_options_group(options_group, entry_ctx);
	mount_point_row.refresh();
	entry_ctx.render();
	list_box.unselect_all();
	list_box.select_row(Some(list_row));
}

fn action_target(is_swap: bool, mount_point: &str, device: &str) -> String {
	if is_swap { device.to_string() } else { mount_point.to_string() }
}

fn report_action_outcome(btn: &Button, heading: &str, body: &str, failed: &str, result: anyhow::Result<()>, refresh: &Rc<dyn Fn()>) {
	match result {
		Ok(()) => {
			ui_commons::present_simple_dialog(btn, heading, body);
			refresh();
		}
		Err(err) => ui_commons::present_simple_dialog(btn, failed, &format!("{err:#}")),
	}
}

fn refresh_mount_group_visibility(group: &PreferencesGroup, entry: &GC<StabEntry>) {
	group.set_visible(entry.borrow().mount_point.trim() != "/");
}

fn refresh_fsck_group_visibility(group: &PreferencesGroup, entry: &GC<StabEntry>) {
	group.set_visible(entry.borrow().fs_type != FsType::Swap);
}

fn add_mount_group(
	editor_panel: &gtk::Box,
	entry: &GC<StabEntry>,
	mount_point_row: &mount_point_value::MountPointRow,
	rebuild_editor: RebuildEditor,
) {
	let group = PreferencesGroup::builder().title(i18n("Mount actions")).build();
	refresh_mount_group_visibility(&group, entry);
	editor_panel.append(&group);
	{
		let (group, entry, row) = (group.clone(), entry.clone(), mount_point_row.row().clone());
		row.connect_changed(move |_| refresh_mount_group_visibility(&group, &entry));
	}

	let status_label = gtk::Label::new(None);
	status_label.set_xalign(0.5);
	status_label.set_halign(Align::Center);
	status_label.set_wrap(true);
	status_label.add_css_class("monospace");
	status_label.set_margin_top(6);
	status_label.set_margin_bottom(6);

	let status_row = PreferencesRow::builder().title(i18n("Status")).child(&status_label).build();
	group.add(&status_row);

	let mount_btn = Button::builder().label(i18n("Mount")).css_classes(["suggested-action"]).build();
	let remount_btn = Button::builder().label(i18n("Remount")).build();
	let unmount_btn = Button::builder().label(i18n("Unmount")).css_classes(["destructive-action"]).build();

	let buttons = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).hexpand(true).build();
	for btn in [&mount_btn, &remount_btn, &unmount_btn] {
		btn.set_hexpand(true);
		buttons.append(btn);
	}

	let buttons_row = PreferencesRow::builder().title(i18n("Actions")).child(&buttons).build();
	buttons_row.set_activatable(false);
	group.add(&buttons_row);

	let refresh: Rc<dyn Fn()> = Rc::new({
		let (entry, status_label, mount_btn) = (entry.clone(), status_label.clone(), mount_btn.clone());
		let (remount_btn, unmount_btn) = (remount_btn.clone(), unmount_btn.clone());
		move || {
			let entry = entry.borrow();
			let status = mount_status::detect(&entry);
			let is_swap = entry.fs_type == FsType::Swap;
			status_label.set_label(status.label().as_str());
			status_label.set_tooltip_text(Some(status.tooltip().as_str()));
			for class in [MountStatus::Mounted, MountStatus::Unmounted, MountStatus::Missing] {
				status_label.remove_css_class(class.css_class());
			}
			status_label.add_css_class(status.css_class());
			mount_btn.set_sensitive(status != MountStatus::Mounted);
			remount_btn.set_sensitive(status == MountStatus::Mounted && !is_swap);
			unmount_btn.set_sensitive(status == MountStatus::Mounted);
		}
	});
	refresh();

	{
		let (group, refresh) = (group.clone(), refresh.clone());
		gtk::glib::timeout_add_local(Duration::from_secs(2), move || {
			if !group.is_mapped() {
				return gtk::glib::ControlFlow::Break;
			}
			refresh();
			gtk::glib::ControlFlow::Continue
		});
	}

	{
		let (entry, refresh, rebuild_editor) = (entry.clone(), refresh.clone(), rebuild_editor.clone());
		let btn = mount_btn.clone();
		ui_commons::confirm_clicked_action(&mount_btn, i18n("Are you sure you want to mount this entry?"))
			.confirm_choice(i18n("Mount"))
			.guard({
				let (entry, btn) = (entry.clone(), btn.clone());
				move || {
					let entry = entry.borrow();
					let (mount_point, is_swap, changed) = (entry.mount_point.clone(), entry.fs_type == FsType::Swap, entry.is_changed());
					mount_action_allowed(&btn, MountAction::Mount, &mount_point, is_swap, changed)
				}
			})
			.connect(move || {
				let snapshot = entry.cloned(|e| e);
				if credentials_flow::needs_credentials(&snapshot) {
					credentials_flow::mount_with_credentials(&btn, entry.clone(), snapshot.clone(), rebuild_editor.clone(), refresh.clone());
				} else {
					let device = credentials_flow::action_device(&snapshot);
					let is_swap = snapshot.fs_type == FsType::Swap;
					let fs_type = snapshot.fs_type.to_string();
					let result = privileged::mount(&snapshot.mount_point, &device, is_swap, &fs_type, None);
					let target = action_target(is_swap, &snapshot.mount_point, &device);
					let body = i18n_fmt("Mounted {mount_point}.", &[("{mount_point}", &target)]);
					report_action_outcome(
						&btn,
						i18n("Mounted").as_str(),
						body.as_str(),
						i18n("Could not mount").as_str(),
						result,
						&refresh,
					);
				}
			});
	}
	{
		let (entry, refresh, btn) = (entry.clone(), refresh.clone(), remount_btn.clone());
		ui_commons::confirm_clicked_action(&remount_btn, i18n("Are you sure you want to remount this entry?"))
			.confirm_choice(i18n("Remount"))
			.guard({
				let (entry, btn) = (entry.clone(), btn.clone());
				move || {
					let entry = entry.borrow();
					let (mount_point, is_swap, changed) = (entry.mount_point.clone(), entry.fs_type == FsType::Swap, entry.is_changed());
					if is_swap {
						ui_commons::present_simple_dialog(&btn, i18n("Cannot remount").as_str(), i18n("Swap cannot be remounted.").as_str());
						return false;
					}
					mount_action_allowed(&btn, MountAction::Remount, &mount_point, false, changed)
				}
			})
			.connect(move || {
				let (mount_point, is_swap) = {
					let entry = entry.borrow();
					(entry.mount_point.clone(), entry.fs_type == FsType::Swap)
				};
				let result = privileged::remount(&mount_point, is_swap);
				let body = i18n_fmt("Remounted {mount_point}.", &[("{mount_point}", &mount_point)]);
				report_action_outcome(
					&btn,
					i18n("Remounted").as_str(),
					body.as_str(),
					i18n("Could not remount").as_str(),
					result,
					&refresh,
				);
			});
	}
	{
		let (entry, refresh, btn) = (entry.clone(), refresh.clone(), unmount_btn.clone());
		ui_commons::confirm_clicked_action(&unmount_btn, i18n("Are you sure you want to unmount this entry?"))
			.confirm_choice(i18n("Unmount"))
			.guard({
				let (entry, btn) = (entry.clone(), btn.clone());
				move || {
					let entry = entry.borrow();
					let (mount_point, is_swap, changed) = (entry.mount_point.clone(), entry.fs_type == FsType::Swap, entry.mount_point_changed());
					mount_action_allowed(&btn, MountAction::Unmount, &mount_point, is_swap, changed)
				}
			})
			.connect(move || {
				let (mount_point, device, is_swap) = {
					let entry = entry.borrow();
					(
						entry.mount_point.clone(),
						credentials_flow::action_device(&entry),
						entry.fs_type == FsType::Swap,
					)
				};
				let result = privileged::unmount(&mount_point, &device, is_swap);
				let target = action_target(is_swap, &mount_point, &device);
				let body = i18n_fmt("Unmounted {mount_point}.", &[("{mount_point}", &target)]);
				report_action_outcome(
					&btn,
					i18n("Unmounted").as_str(),
					body.as_str(),
					i18n("Could not unmount").as_str(),
					result,
					&refresh,
				);
			});
	}
}

fn add_user_label_row(options: &PreferencesGroup, entry_ctx: &EntryContext) {
	let row = EntryRow::builder()
		.title(i18n("Label"))
		.text(entry_ctx.entry().borrow().user_label.as_deref().unwrap_or(""))
		.build();
	{
		let entry_ctx = entry_ctx.clone();
		row.connect_changed(move |row| {
			let text = row.text();
			{
				let mut entry = entry_ctx.entry().borrow_mut();
				entry.user_label = if text.is_empty() { None } else { Some(text.to_string()) };
			}
			entry_ctx.render();
		});
	}
	options.add(&row);
}

fn add_spin_row(
	options: &PreferencesGroup,
	entry_ctx: &EntryContext,
	title: &str,
	subtitle: &str,
	initial: u8,
	apply: impl Fn(&mut StabEntry, u8) + 'static,
) {
	let entry_ctx = entry_ctx.clone();

	let adjustment = Adjustment::builder().value(f64::from(initial)).step_increment(1.0).build();

	let row = SpinRow::new(Some(&adjustment), 1.0, 0);
	row.set_title(title);
	row.set_subtitle(subtitle);
	row.set_range(0.0, 255.0);
	row.set_climb_rate(1.0);
	row.set_numeric(true);
	row.set_value(f64::from(initial));
	let row_ref = row.clone();
	row.adjustment().connect_value_changed(move |_| {
		let value = row_ref.value().round() as u8;
		{
			let mut entry = entry_ctx.entry().borrow_mut();
			apply(&mut entry, value);
		}
		entry_ctx.render();
	});
	options.add(&row);
}
