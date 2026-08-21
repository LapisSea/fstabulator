use crate::context::EntryContext;
use adw::prelude::*;
use adw::{EntryRow, PreferencesGroup};
use std::path::Path;

pub fn add_mount_point_row(options: &PreferencesGroup, entry_ctx: &EntryContext) {
	let entry = entry_ctx.entry().clone();

	let folder_btn = gtk::Button::from_icon_name("folder-open-symbolic");
	folder_btn.add_css_class("flat");
	folder_btn.set_tooltip_text(Some("Choose folder"));

	let exists_icon = gtk::Image::from_icon_name("object-select-symbolic");
	exists_icon.add_css_class("mount-point-exists");
	exists_icon.set_valign(gtk::Align::Center);

	let row = EntryRow::builder().title("Mount point").text(&entry.borrow().mount_point).build();
	row.add_suffix(&folder_btn);
	row.add_suffix(&exists_icon);
	update_exists_icon(&exists_icon, entry.borrow().mount_point.as_str());
	{
		let (entry_ctx, entry, exists_icon) = (entry_ctx.clone(), entry.clone(), exists_icon.clone());
		row.connect_changed(move |row| {
			{
				let mut entry = entry.borrow_mut();
				entry.mount_point = row.text().to_string();
			}
			update_exists_icon(&exists_icon, row.text().as_str());
			entry_ctx.render();
		});
	}
	{
		let (row, entry, entry_ctx) = (row.clone(), entry.clone(), entry_ctx.clone());
		let exists_icon = exists_icon.clone();
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
			let (row, entry, entry_ctx) = (row.clone(), entry.clone(), entry_ctx.clone());
			let exists_icon = exists_icon.clone();
			dialog.connect_response(move |dialog, response| {
				if response == gtk::ResponseType::Accept
					&& let Some(path) = dialog.file().and_then(|file| file.path())
				{
					let text = path.to_string_lossy().into_owned();
					row.set_text(&text);
					{
						let mut entry = entry.borrow_mut();
						entry.mount_point = text;
					}
					update_exists_icon(&exists_icon, row.text().as_str());
					entry_ctx.render();
				}
			});
			dialog.show();
		});
	}
	options.add(&row);
}

fn update_exists_icon(icon: &gtk::Image, mount_point: &str) {
	let exists = Path::new(mount_point.trim()).is_dir();
	icon.set_visible(exists);
	icon.set_tooltip_text(Some(if exists { "Path exists" } else { "Path does not exist" }));
}
