use crate::stab_yurself::{StabEntry, StabFile};
use crate::{GC, render_list_entry};
use adw::ActionRow;
use gtk::Button;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct FileContext {
	file: GC<StabFile>,
	refresh: Rc<dyn Fn()>,
}

impl FileContext {
	pub(crate) fn new(file: GC<StabFile>, refresh: Rc<dyn Fn()>) -> Self {
		FileContext { file, refresh }
	}

	pub(crate) fn file(&self) -> &GC<StabFile> {
		&self.file
	}

	pub(crate) fn notify(&self) {
		(self.refresh)();
	}

	pub(crate) fn entry(&self, entry: GC<StabEntry>, row: &ActionRow) -> EntryContext {
		EntryContext {
			file: self.clone(),
			entry,
			row: row.clone(),
			reset_btn: GC::new(None),
		}
	}
}

#[derive(Clone)]
pub(crate) struct EntryContext {
	file: FileContext,
	entry: GC<StabEntry>,
	row: ActionRow,
	reset_btn: GC<Option<Button>>,
}

impl EntryContext {
	pub(crate) fn set_reset_btn(&self, btn: &Button) {
		*self.reset_btn.borrow_mut() = Some(btn.clone());
	}

	pub(crate) fn entry(&self) -> &GC<StabEntry> {
		&self.entry
	}

	pub(crate) fn render(&self) {
		let reset_btn = self.reset_btn.borrow().clone();
		render_list_entry(&self.row, &self.entry.borrow(), reset_btn.as_ref());
		self.file.notify();
	}
}
