use crate::context::EntryContext;
use crate::fs_value::FsType;
use crate::i18n::i18n;
use crate::stab_yurself::StabEntry;
use crate::{GC, problem_reports, ui_commons};
use adw::prelude::*;
use adw::{EntryRow, PreferencesGroup};

#[derive(Clone)]
pub struct MountPointRow {
	entry: GC<StabEntry>,
	row: EntryRow,
	icon: gtk::Image,
}

impl MountPointRow {
	pub fn refresh(&self) {
		let text = self.entry.borrow().mount_point.clone();
		self.row.set_text(&text);
		update_status_icon(&self.icon, &self.entry.borrow());
	}
}

pub fn add_mount_point_row(options: &PreferencesGroup, entry_ctx: &EntryContext) -> MountPointRow {
	let entry = entry_ctx.entry().clone();

	let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
	folder_btn.add_css_class("flat");
	folder_btn.set_tooltip_text(Some(i18n("Choose folder").as_str()));

	let exists_icon = ui_commons::issue_image();

	let row = EntryRow::builder().title(i18n("Mount point")).text(&entry.borrow().mount_point).build();
	row.add_suffix(&folder_btn);
	row.add_prefix(&exists_icon);
	update_status_icon(&exists_icon, &entry.borrow());
	{
		let (entry_ctx, entry, exists_icon) = (entry_ctx.clone(), entry.clone(), exists_icon.clone());
		row.connect_changed(move |row| {
			{
				let mut entry = entry.borrow_mut();
				entry.mount_point = row.text().to_string();
			}
			update_status_icon(&exists_icon, &entry.borrow());
			entry_ctx.render();
		});
	}
	{
		let (row, entry, entry_ctx) = (row.clone(), entry.clone(), entry_ctx.clone());
		let exists_icon = exists_icon.clone();
		folder_btn.connect_clicked(move |folder_btn| {
			let dialog = gtk::FileDialog::builder().title(i18n("Choose mount point")).build();
			let text = row.text();
			if !text.is_empty() {
				dialog.set_initial_folder(Some(&gtk::gio::File::for_path(text.as_str())));
			}
			let parent = folder_btn.root().and_then(|root| root.downcast::<gtk::Window>().ok());
			let (row, entry, entry_ctx) = (row.clone(), entry.clone(), entry_ctx.clone());
			let exists_icon = exists_icon.clone();
			dialog.select_folder(parent.as_ref(), None::<&gtk::gio::Cancellable>, move |result| {
				if let Ok(file) = result
					&& let Some(path) = file.path()
				{
					let text = path.to_string_lossy().into_owned();
					row.set_text(&text);
					{
						let mut entry = entry.borrow_mut();
						entry.mount_point = text;
					}
					update_status_icon(&exists_icon, &entry.borrow());
					entry_ctx.render();
				}
			});
		});
	}
	options.add(&row);
	MountPointRow {
		entry,
		row,
		icon: exists_icon,
	}
}

fn update_status_icon(icon: &gtk::Image, entry: &StabEntry) {
	let problem = problem_reports::check(&problem_reports::CheckValue::MountPoint(entry.mount_point.clone()), entry);
	ui_commons::update_issue_icon(icon, problem.as_ref());
	if problem.is_none() && entry.fs_type != FsType::Swap && !entry.mount_point.trim().is_empty() {
		icon.set_tooltip_text(Some(i18n("Path exists").as_str()));
	}
}
