use crate::GC;
use crate::i18n::i18n;
use crate::ui_commons::clear_children;
use adw::prelude::*;
use gtk::{Align, Box as GtkBox, ListBox, MenuButton, Orientation, Popover, ScrolledWindow, SearchEntry, Widget};
use std::rc::Rc;

pub enum ErrorRenderer {
	Message(String),
	Custom(&'static dyn Fn(&anyhow::Error) -> Widget),
}

const MAX_LIST_HEIGHT: i32 = 240;

type Filter<T> = Rc<dyn Fn(&str, &T) -> bool>;

pub struct SearchPickerBuilder<T: Clone + 'static, W: IsA<Widget>> {
	search_placeholder: String,
	menu_label: String,
	tooltip: String,
	dataset: Rc<dyn Fn() -> anyhow::Result<Vec<T>>>,
	render_row: Rc<dyn Fn(&T) -> W>,
	render_error: ErrorRenderer,
	filter: Option<Filter<T>>,
	on_select: Rc<dyn Fn(T, usize)>,
}

impl<T: Clone + 'static, W: IsA<Widget>> SearchPickerBuilder<T, W> {
	pub fn new(
		menu_label: impl Into<String>,
		dataset: impl Fn() -> anyhow::Result<Vec<T>> + 'static,
		render_row: impl Fn(&T) -> W + 'static,
		on_select: impl Fn(T, usize) + 'static,
	) -> Self {
		Self {
			search_placeholder: i18n("Search"),
			menu_label: menu_label.into(),
			tooltip: String::new(),
			dataset: Rc::new(dataset),
			render_row: Rc::new(render_row),
			render_error: ErrorRenderer::Message(i18n("Failed to load")),
			filter: None,
			on_select: Rc::new(on_select),
		}
	}

	pub fn search_placeholder(mut self, value: impl Into<String>) -> Self {
		self.search_placeholder = value.into();
		self
	}

	pub fn tooltip(mut self, value: impl Into<String>) -> Self {
		self.tooltip = value.into();
		self
	}

	pub fn error_message(mut self, value: impl Into<String>) -> Self {
		self.render_error = ErrorRenderer::Message(value.into());
		self
	}

	#[allow(dead_code)]
	pub fn error_custom(mut self, value: &'static dyn Fn(&anyhow::Error) -> Widget) -> Self {
		self.render_error = ErrorRenderer::Custom(value);
		self
	}

	pub fn filter(mut self, value: impl Fn(&str, &T) -> bool + 'static) -> Self {
		self.filter = Some(Rc::new(value));
		self
	}

	pub fn build(self) -> MenuButton {
		let Self {
			search_placeholder,
			menu_label,
			tooltip,
			dataset,
			render_row,
			render_error,
			filter,
			on_select,
		} = self;
		let filter: Filter<T> = filter.unwrap_or_else(|| Rc::new(|_, _| true));
		let render_error: Rc<dyn Fn(&anyhow::Error) -> Widget> = Rc::new(move |err: &anyhow::Error| match &render_error {
			ErrorRenderer::Message(msg) => gtk::Label::new(Some(&format!("{msg}:\n{err:#}"))).upcast::<gtk::Widget>(),
			ErrorRenderer::Custom(efn) => efn(err),
		});

		let search = SearchEntry::builder().placeholder_text(search_placeholder.as_str()).hexpand(true).build();
		let list_box = ListBox::builder().css_classes(["boxed-list"]).hexpand(true).valign(Align::Start).build();
		let scroll = ScrolledWindow::builder()
			.child(&list_box)
			.max_content_height(MAX_LIST_HEIGHT)
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

		let menu_btn = MenuButton::builder().label(menu_label.as_str()).popover(&popover).build();
		if !tooltip.is_empty() {
			menu_btn.set_tooltip_text(Some(tooltip.as_str()));
		}

		let render_row: Rc<dyn Fn(&T) -> Widget> = Rc::new(move |item: &T| render_row(item).upcast());

		let items: GC<Vec<T>> = GC::new(Vec::new());
		let shown: GC<Vec<T>> = GC::new(Vec::new());

		{
			let (search, list_box, scroll) = (search.clone(), list_box.clone(), scroll.clone());
			let (menu_btn, search_box, error_box) = (menu_btn.clone(), search_box.clone(), error_box.clone());
			let (dataset, render_error) = (dataset.clone(), render_error.clone());
			let (items, shown) = (items.clone(), shown.clone());
			let (filter, render_row) = (filter.clone(), render_row.clone());
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
						render(&search, &list_box, &scroll, &items, &shown, &filter, &render_row);
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
			let (search, list_box, scroll) = (search.clone(), list_box.clone(), scroll.clone());
			let (items, shown, filter) = (items.clone(), shown.clone(), filter.clone());
			let render_row = render_row.clone();
			search.connect_search_changed(move |search| {
				render(search, &list_box, &scroll, &items, &shown, &filter, &render_row);
			});
		}

		{
			let (popover, shown, on_select) = (popover.clone(), shown.clone(), on_select.clone());
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
}

fn render<T: Clone>(
	search: &SearchEntry,
	list_box: &ListBox,
	scroll: &ScrolledWindow,
	items: &GC<Vec<T>>,
	shown: &GC<Vec<T>>,
	filter: &Filter<T>,
	render_row: &Rc<dyn Fn(&T) -> Widget>,
) {
	clear_children(list_box);
	let query = search.text();
	let mut matches = Vec::new();
	for item in items.borrow().iter() {
		if filter(&query, item) {
			list_box.append(&render_row(item));
			matches.push(item.clone());
		}
	}
	*shown.borrow_mut() = matches;
	let natural = list_box.measure(Orientation::Vertical, -1).1;
	scroll.set_height_request(if natural > 0 { natural.min(MAX_LIST_HEIGHT) } else { -1 });
}
