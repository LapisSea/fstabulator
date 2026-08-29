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
	pub fn row(&self) -> &EntryRow {
		&self.row
	}

	pub fn refresh(&self) {
		let entry = self.entry.borrow().clone();
		self.row.set_visible(entry.fs_type != FsType::Swap);
		self.row.set_text(&entry.mount_point);
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
	row.set_visible(entry.borrow().fs_type != FsType::Swap);
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::context::FileContext;
	use crate::stab_yurself::StabFile;
	use adw::ActionRow;
	use std::rc::Rc;

	fn skip_if_no_display() -> bool {
		let opened = gtk::gdk::Display::default().is_some() || gtk::gdk::Display::open(None).is_some();
		if !opened {
			eprintln!("skipping UI test: no display available");
		}
		!opened
	}

	fn entry_ctx(entry: GC<StabEntry>) -> EntryContext {
		let file_ctx = FileContext::new(GC::new(StabFile::empty()), Rc::new(|| {}));
		file_ctx.entry(entry, &ActionRow::builder().build())
	}

	fn mount_point_row(raw: &str) -> (MountPointRow, GC<StabEntry>) {
		let entry = GC::new(StabEntry::from(0, raw).unwrap());
		let row = add_mount_point_row(&PreferencesGroup::builder().build(), &entry_ctx(entry.clone()));
		(row, entry)
	}

	#[gtk::test]
	fn swap_entry_hides_mount_point_row() {
		if skip_if_no_display() {
			return;
		}
		let (row, _) = mount_point_row("/dev/zram0 none swap defaults 0 0");
		assert!(!row.row().is_visible());
	}

	#[gtk::test]
	fn non_swap_entry_shows_mount_point_row() {
		if skip_if_no_display() {
			return;
		}
		let (row, _) = mount_point_row("UUID=1 /mnt/data ext4 defaults 0 2");
		assert!(row.row().is_visible());
		assert_eq!(row.row().text().to_string(), "/mnt/data");
	}

	#[gtk::test]
	fn refresh_survives_fs_type_transition() {
		if skip_if_no_display() {
			return;
		}
		let (row, entry) = mount_point_row("/dev/zram0 none swap defaults 0 0");
		assert!(!row.row().is_visible());

		entry.borrow_mut().set_fs_type(FsType::Ext4);
		entry.borrow_mut().mount_point = "/mnt/data".to_string();
		row.refresh();

		assert!(row.row().is_visible());
		assert_eq!(row.row().text().to_string(), "/mnt/data");
		assert_eq!(entry.borrow().mount_point.as_str(), "/mnt/data");
	}

	#[gtk::test]
	fn typing_mount_point_updates_entry() {
		if skip_if_no_display() {
			return;
		}
		let (row, entry) = mount_point_row("UUID=1 /mnt/data ext4 defaults 0 2");
		row.row().set_text("/mnt/edited");
		assert_eq!(entry.borrow().mount_point.as_str(), "/mnt/edited");
	}
}
