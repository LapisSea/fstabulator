use crate::context::EntryContext;
use adw::prelude::*;
use adw::{EntryRow, PreferencesGroup};

pub fn add_mount_point_row(options: &PreferencesGroup, entry_ctx: &EntryContext) {
	let entry = entry_ctx.entry().clone();

	let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
	folder_btn.add_css_class("flat");
	folder_btn.set_tooltip_text(Some("Choose folder"));

	let row = EntryRow::builder().title("Mount point").text(&entry.borrow().mount_point).build();
	row.add_suffix(&folder_btn);
	{
		let entry_ctx = entry_ctx.clone();
		let entry = entry.clone();
		row.connect_changed(move |row| {
			{
				let mut entry = entry.borrow_mut();
				entry.mount_point = row.text().to_string();
			}
			entry_ctx.render();
		});
	}
	{
		let row = row.clone();
		let entry = entry.clone();
		let entry_ctx = entry_ctx.clone();
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
			let entry_ctx = entry_ctx.clone();
			dialog.connect_response(move |dialog, response| {
				if response == gtk::ResponseType::Accept {
					if let Some(path) = dialog.file().and_then(|file| file.path()) {
						let text = path.to_string_lossy().into_owned();
						row.set_text(&text);
						{
							let mut entry = entry.borrow_mut();
							entry.mount_point = text;
						}
						entry_ctx.render();
					}
				}
			});
			dialog.show();
		});
	}
	options.add(&row);
}
