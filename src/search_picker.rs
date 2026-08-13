use crate::GC;
use adw::prelude::*;
use gtk::{Align, Box as GtkBox, ListBox, MenuButton, Orientation, Popover, ScrolledWindow, SearchEntry, Widget};
use std::rc::Rc;

pub enum ErrorRenderer {
	Message(&'static str),
	Custom(&'static dyn Fn(&anyhow::Error) -> Widget),
}

pub fn build_search_picker<T: Clone + 'static>(
	search_placeholder: &str,
	menu_label: &str,
	tooltip: &str,
	dataset: impl Fn() -> Result<Vec<T>, anyhow::Error> + 'static,
	render_row: impl Fn(&T) -> Widget + 'static,
	render_error: ErrorRenderer,
	filter: impl Fn(&str, &T) -> bool + 'static,
	on_select: impl Fn(T, usize) + 'static,
) -> MenuButton {
	let render_error: Rc<dyn Fn(&anyhow::Error) -> Widget> = Rc::new(move |err: &anyhow::Error| match &render_error {
		ErrorRenderer::Message(msg) => gtk::Label::new(Some(&format!("{msg}:\n{err:#}"))).upcast::<gtk::Widget>(),
		ErrorRenderer::Custom(efn) => efn(err),
	});

	let search = SearchEntry::builder().placeholder_text(search_placeholder).hexpand(true).build();
	let list_box = ListBox::builder().css_classes(["boxed-list"]).hexpand(true).valign(Align::Start).build();
	let scroll = ScrolledWindow::builder()
		.child(&list_box)
		.max_content_height(240)
		.max_content_width(360)
		.propagate_natural_height(true)
		.hexpand(true)
		.build();

	let search_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).build();
	search_box.append(&search);
	search_box.append(&scroll);

	let error_box = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).build();
	error_box.set_visible(false);

	let popover_content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).build();
	popover_content.append(&search_box);
	popover_content.append(&error_box);

	let popover = Popover::builder().child(&popover_content).build();

	let menu_btn = MenuButton::builder().label(menu_label).popover(&popover).build();
	menu_btn.set_tooltip_text(Some(tooltip));

	let dataset: Rc<dyn Fn() -> Result<Vec<T>, anyhow::Error>> = Rc::new(dataset);
	let render_row: Rc<dyn Fn(&T) -> Widget> = Rc::new(render_row);
	let filter: Rc<dyn Fn(&str, &T) -> bool> = Rc::new(filter);
	let on_select: Rc<dyn Fn(T, usize)> = Rc::new(on_select);

	let items: GC<Vec<T>> = GC::new(Vec::new());
	let shown: GC<Vec<T>> = GC::new(Vec::new());

	{
		let search = search.clone();
		let list_box = list_box.clone();
		let menu_btn = menu_btn.clone();
		let search_box = search_box.clone();
		let error_box = error_box.clone();
		let dataset = dataset.clone();
		let render_error = render_error.clone();
		let items = items.clone();
		let shown = shown.clone();
		let filter = filter.clone();
		let render_row = render_row.clone();
		popover.connect_visible_notify(move |popover| {
			if !popover.is_visible() {
				return;
			}
			if menu_btn.label().is_some_and(|label| !label.is_empty()) {
				popover.set_size_request(menu_btn.width(), -1);
			}
			match dataset() {
				Ok(data) => {
					search_box.set_visible(true);
					error_box.set_visible(false);
					clear_children(&error_box);
					*items.borrow_mut() = data;
					search.set_text("");
					render(&search, &list_box, &items, &shown, &filter, &render_row);
					search.grab_focus();
				}
				Err(err) => {
					search_box.set_visible(false);
					error_box.set_visible(true);
					clear_children(&error_box);
					error_box.append(&render_error(&err));
				}
			}
		});
	}

	{
		let search = search.clone();
		let list_box = list_box.clone();
		let items = items.clone();
		let shown = shown.clone();
		let filter = filter.clone();
		let render_row = render_row.clone();
		search.connect_search_changed(move |search| {
			render(search, &list_box, &items, &shown, &filter, &render_row);
		});
	}

	{
		let popover = popover.clone();
		let shown = shown.clone();
		let on_select = on_select.clone();
		list_box.connect_row_activated(move |_, row| {
			let index = row.index();
			if index < 0 {
				return;
			}
			let index = index as usize;
			let Some(item) = shown.borrow().get(index).cloned() else {
				return;
			};
			on_select(item, index);
			popover.popdown();
		});
	}

	menu_btn
}

fn clear_list(list: &ListBox) {
	while let Some(row) = list.row_at_index(0) {
		list.remove(&row);
	}
}

fn clear_children(container: &GtkBox) {
	while let Some(child) = container.last_child() {
		container.remove(&child);
	}
}

fn render<T: Clone>(
	search: &SearchEntry,
	list_box: &ListBox,
	items: &GC<Vec<T>>,
	shown: &GC<Vec<T>>,
	filter: &Rc<dyn Fn(&str, &T) -> bool>,
	render_row: &Rc<dyn Fn(&T) -> Widget>,
) {
	clear_list(list_box);
	let query = search.text();
	let mut matches = Vec::new();
	for item in items.borrow().iter() {
		if filter(&query, item) {
			list_box.append(&render_row(item));
			matches.push(item.clone());
		}
	}
	*shown.borrow_mut() = matches;
}
