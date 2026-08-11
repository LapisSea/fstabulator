use crate::GC;
use crate::render_list_entry;
use crate::stab_yurself::StabEntry;
use adw::prelude::*;
use adw::{ActionRow, EntryRow, PreferencesGroup};

pub fn add_mount_point_row(options: &PreferencesGroup, entry: &GC<StabEntry>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	let entry = entry.clone();
	let action_row = action_row.clone();
	let reset_btn = reset_btn.clone();

	let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
	folder_btn.add_css_class("flat");
	folder_btn.set_tooltip_text(Some("Choose folder"));

	let row = EntryRow::builder().title("Mount point").text(&entry.borrow().mount_point).build();
	row.add_suffix(&folder_btn);
	{
		let entry = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		row.connect_changed(move |row| {
			let mut entry = entry.borrow_mut();
			entry.mount_point = row.text().to_string();
			render_list_entry(&action_row, &entry, Some(&reset_btn));
		});
	}
	{
		let row = row.clone();
		let entry = entry.clone();
		let action_row = action_row.clone();
		let reset_btn = reset_btn.clone();
		folder_btn.connect_clicked(move |folder_btn| {
			let dialog = gtk::FileChooserNative::builder()
				.title("Choose mount point")
				.action(gtk::FileChooserAction::SelectFolder)
				.build();
			if let Some(window) = folder_btn.root().and_then(|root| root.downcast::<gtk::Window>().ok()) {
				dialog.set_transient_for(Some(&window));
			}
			let text = row.text();
			if !text.is_empty() {
				let _ = dialog.set_file(&gtk::gio::File::for_path(text.as_str()));
			}
			let row = row.clone();
			let entry = entry.clone();
			let action_row = action_row.clone();
			let reset_btn = reset_btn.clone();
			dialog.connect_response(move |dialog, response| {
				if response == gtk::ResponseType::Accept {
					if let Some(path) = dialog.file().and_then(|file| file.path()) {
						let text = path.to_string_lossy().into_owned();
						row.set_text(&text);
						let mut entry = entry.borrow_mut();
						entry.mount_point = text;
						render_list_entry(&action_row, &entry, Some(&reset_btn));
					}
				}
			});
			dialog.show();
		});
	}
	options.add(&row);
}
