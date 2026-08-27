use crate::context::EntryContext;
use crate::device_value::DeviceValue;
use crate::fs_options::{CompressionSpec, FsOption, OptionSpec, OptionValue};
use crate::i18n::i18n;
use crate::problem_reports::{CheckValue, Problem, check};
use crate::search_picker::SearchPickerBuilder;
use crate::subvolume::{Subvol, list_subvolumes};
use crate::{GC, fs_options, ui_commons};
use adw::prelude::*;
use adw::{ActionRow, EntryRow, PreferencesGroup, PreferencesRow};
use gtk::{Align, Box as GtkBox, Button, CheckButton, DropDown, MenuButton, Orientation, StringList};
use std::fmt::Display;
use std::rc::Rc;
use std::time::Duration;

pub fn build_options_group(group: &PreferencesGroup, entry_ctx: &EntryContext) {
	while let Some(row) = group.row(0) {
		group.remove(&row);
	}
	let options = entry_ctx.entry().cloned(|w| &w.options);
	let ctx = AddContext::new(group, entry_ctx);
	for (index, value) in options.iter().enumerate() {
		add_option_row(AddContext {
			index,
			value: value.clone(),
			..ctx.clone()
		});
	}
	add_add_option_row(ctx);
}

#[derive(Clone)]
pub struct AddContext {
	entry_ctx: EntryContext,
	group: PreferencesGroup,
	index: usize,
	value: FsOption,
}

impl AddContext {
	pub fn new(group: &PreferencesGroup, entry_ctx: &EntryContext) -> Self {
		AddContext {
			entry_ctx: entry_ctx.clone(),
			group: group.clone(),
			index: 0,
			value: FsOption::Named(String::new()),
		}
	}
}

fn add_option_row(ctx: AddContext) {
	let trash = make_trash_button(&ctx);

	let name = ctx.value.name().to_string();
	let current = match &ctx.value {
		FsOption::Named(_) => String::new(),
		FsOption::KeyValue(_, value) => value.clone(),
	};

	let option = fs_options::lookup(&ctx.entry_ctx.entry().borrow().fs_type, &name);

	let row = ActionRow::builder().title(name.as_str()).build();
	if let Some(OptionSpec { description, .. }) = option {
		row.set_subtitle(i18n(description).as_str());
	}

	match option {
		Some(OptionSpec {
			description: _,
			value: OptionValue::Toggle,
			..
		}) if current.is_empty() => {
			row.add_suffix(&trash);
			ctx.group.add(&row);
		}
		Some(OptionSpec {
			description: _,
			value: OptionValue::Enum(values),
			..
		}) => {
			if values.contains(&current.as_str()) {
				let model = StringList::new(values);
				let dropdown = DropDown::builder().model(&model).build();
				dropdown.set_valign(Align::Center);
				if let Some(pos) = values.iter().position(|v| *v == current.as_str()) {
					dropdown.set_selected(pos as u32);
				}
				row.add_suffix(&dropdown);
				row.add_suffix(&trash);
				ctx.group.add(&row);

				let (ctx, name) = (ctx.clone(), name.clone());
				dropdown.connect_selected_notify(move |dropdown| {
					let Some(selected) = model.string(dropdown.selected()) else { return };
					set_option(&ctx, FsOption::from_kv(name.clone(), selected));
				});
			} else {
				add_free_text_option_row(ctx.clone(), &trash);
			}
		}
		Some(OptionSpec {
			description,
			value: OptionValue::Compression(algorithms),
			..
		}) => {
			let (algo, level) = CompressionSpec::split(&current);
			if algorithms.iter().any(|spec| spec.name == algo) {
				add_compression_option_row(ctx.clone(), &trash, &name, description, algorithms, algo, level);
			} else {
				add_free_text_option_row(ctx.clone(), &trash);
			}
		}
		Some(OptionSpec {
			description: _,
			value: OptionValue::Bool(bool_type),
			..
		}) => {
			let (on, off) = bool_type.values();
			let check = CheckButton::builder()
				.active(bool_type.parse(&current).unwrap_or(false))
				.valign(Align::Center)
				.build();
			row.add_suffix(&check);
			row.add_suffix(&trash);
			ctx.group.add(&row);

			let row_icon = option_row_icon(row.clone());
			refresh_option_issue(row_icon.as_ref(), &ctx);
			let (ctx, name) = (ctx.clone(), name.clone());
			let row_icon = row_icon.clone();
			check.connect_toggled(move |check| {
				let value = if check.is_active() { on } else { off };
				set_option(&ctx, FsOption::from_kv(name.clone(), value));
				refresh_option_issue(row_icon.as_ref(), &ctx);
			});
		}
		Some(OptionSpec {
			description,
			value: OptionValue::Integer,
			..
		}) => {
			add_digits_option_row(
				ctx.clone(),
				&trash,
				&name,
				description,
				&current,
				|c| c.is_ascii_digit(),
				|value| value.to_string(),
			);
		}
		Some(OptionSpec {
			description,
			value: OptionValue::IntegerRange(min, max),
			..
		}) => {
			let negative = min < 0;
			add_digits_option_row(
				ctx.clone(),
				&trash,
				&name,
				description,
				&current,
				move |c| c.is_ascii_digit() || (negative && c == '-'),
				move |value| {
					value
						.parse::<i64>()
						.map_or_else(|_| value.to_string(), |value| value.clamp(min, max).to_string())
				},
			);
		}
		Some(OptionSpec {
			description,
			value: OptionValue::String,
			..
		}) => {
			add_string_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		Some(OptionSpec {
			description,
			value: OptionValue::Size,
			..
		}) => {
			add_size_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		Some(OptionSpec {
			description,
			value: OptionValue::Octal,
			..
		}) => {
			add_digits_option_row(
				ctx.clone(),
				&trash,
				&name,
				description,
				&current,
				|c| matches!(c, '0'..='7'),
				|value| value.to_string(),
			);
		}
		Some(OptionSpec {
			description,
			value: OptionValue::Subvol,
			..
		}) => {
			add_subvol_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		None
		| Some(OptionSpec {
			value: OptionValue::Toggle, ..
		}) => {
			add_free_text_option_row(ctx.clone(), &trash);
		}
	}
}

fn make_trash_button(ctx: &AddContext) -> Button {
	let trash = ui_commons::trash_button(i18n("Remove option").as_str());

	let ctx = ctx.clone();
	trash.connect_clicked(move |_| {
		let entry = ctx.entry_ctx.entry().clone();
		entry.borrow_mut().options.remove(ctx.index);
		build_options_group(&ctx.group, &ctx.entry_ctx);
		ctx.entry_ctx.render();
	});
	trash
}

fn option_row_issue(ctx: &AddContext) -> Option<Problem> {
	let entry = ctx.entry_ctx.entry().borrow();
	entry
		.options
		.get(ctx.index)
		.and_then(|option| check(&CheckValue::Option(option.clone()), &entry))
}

fn option_row_icon(row: impl IsA<gtk::Widget>) -> Option<gtk::Image> {
	ui_commons::find_widget_with_class(row, "icon")?.downcast::<gtk::Image>().ok()
}

fn refresh_option_issue(icon: Option<&gtk::Image>, ctx: &AddContext) {
	if let Some(icon) = icon {
		ui_commons::update_issue_icon(icon, option_row_issue(ctx).as_ref());
	}
}

fn set_option(ctx: &AddContext, value: FsOption) {
	ctx.entry_ctx.entry().borrow_mut().options[ctx.index] = value;
	ctx.entry_ctx.render();
}

fn add_free_text_option_row(ctx: AddContext, trash: &gtk::Button) {
	let row = EntryRow::builder().title(i18n("Option")).text(ctx.value.to_string()).build();
	let issue_icon = ui_commons::issue_image();
	row.add_prefix(&issue_icon);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let (issue_icon, ctx) = (issue_icon.clone(), ctx.clone());
	refresh_option_issue(Some(&issue_icon), &ctx);
	row.connect_changed(move |row| {
		set_option(&ctx, FsOption::from_raw(&row.text()));
		refresh_option_issue(Some(&issue_icon), &ctx);
	});
}

fn add_entry_option_row(
	ctx: AddContext,
	trash: &gtk::Button,
	name: &str,
	description: &str,
	input: &gtk::Entry,
	input_extras: &[&impl IsA<gtk::Widget>],
	issue: Option<&gtk::Image>,
) {
	let header = ui_commons::titled_header(name, Some(i18n(description).as_str()), issue, trash);
	let input_row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
	input_row.append(input);
	for extra in input_extras {
		input_row.append(*extra);
	}
	let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).margin_bottom(6).build();
	content.append(&header);
	content.append(&input_row);
	ctx.group.add(&PreferencesRow::builder().activatable(false).child(&content).build());

	let ctx = ctx.clone();
	let name = name.to_string();
	input.connect_changed(move |input| {
		set_option(&ctx, FsOption::from_kv(name.clone(), input.text()));
	});
}

fn add_string_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let input = gtk::Entry::builder().text(current).hexpand(true).margin_start(12).margin_end(12).build();
	let issue_icon = ui_commons::issue_image();
	add_entry_option_row(ctx.clone(), trash, name, description, &input, &[] as &[&gtk::Entry], Some(&issue_icon));
	refresh_option_issue(Some(&issue_icon), &ctx);

	let (ctx, issue_icon) = (ctx.clone(), issue_icon.clone());
	input.connect_changed(move |_| refresh_option_issue(Some(&issue_icon), &ctx));
}

const SIZE_UNITS: [&str; 8] = ["B", "K", "M", "G", "T", "P", "E", "%"];

fn split_size(value: &str) -> (&str, &str) {
	let num_len = value.chars().take_while(|c| c.is_ascii_digit() || *c == '.').count();
	(&value[..num_len], &value[num_len..])
}

fn add_size_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let (num, unit) = split_size(current);
	let model = StringList::new(&SIZE_UNITS);
	let dropdown = DropDown::builder().model(&model).build();
	if let Some(pos) = SIZE_UNITS.iter().position(|u| u.eq_ignore_ascii_case(unit)) {
		dropdown.set_selected(pos as u32);
	}
	let input = gtk::Entry::builder()
		.text(num)
		.input_purpose(gtk::InputPurpose::Number)
		.width_chars(8)
		.build();
	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
	content.set_valign(Align::Center);
	content.append(&input);
	content.append(&dropdown);

	let row = ActionRow::builder().title(name).subtitle(i18n(description)).build();
	row.add_suffix(&content);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let row_icon = option_row_icon(row.clone());
	refresh_option_issue(row_icon.as_ref(), &ctx);
	let (ctx, model) = (ctx.clone(), model.clone());
	let name = name.to_string();
	let (apply_input, apply_dropdown) = (input.clone(), dropdown.clone());
	let row_icon = row_icon.clone();
	let apply = Rc::new(move || {
		let value = match model.string(apply_dropdown.selected()).as_deref() {
			Some("B") | None => apply_input.text().to_string(),
			Some(s) => format!("{}{s}", apply_input.text()),
		};
		set_option(&ctx, FsOption::from_kv(name.clone(), value));
		refresh_option_issue(row_icon.as_ref(), &ctx);
	});
	input.connect_changed({
		let apply = apply.clone();
		move |_| apply()
	});
	dropdown.connect_selected_notify(move |_| apply());
}

fn filter_input(input: &gtk::Entry, valid: impl Fn(char) -> bool + 'static, on_changed: impl Fn(&str) + 'static) {
	let apply_input = input.clone();
	input.connect_changed(move |input| {
		let text = input.text();
		let cleaned: String = text.chars().filter(|c| valid(*c)).collect();
		if cleaned.as_str() != text.as_str() {
			let before: String = text.chars().take(input.position().max(0) as usize).filter(|c| valid(*c)).collect();
			let idle_input = apply_input.clone();
			gtk::glib::idle_add_local(move || {
				idle_input.set_text(&cleaned);
				idle_input.set_position(before.chars().count() as i32);
				gtk::glib::ControlFlow::Break
			});
			return;
		}
		on_changed(&cleaned);
	});
}

fn add_compression_option_row(
	ctx: AddContext,
	trash: &gtk::Button,
	name: &str,
	description: &str,
	algorithms: &'static [CompressionSpec],
	current_algo: &str,
	current_level: Option<&str>,
) {
	let names: Vec<&str> = algorithms.iter().map(|spec| spec.name).collect();
	let model = StringList::new(&names);
	let dropdown = DropDown::builder().model(&model).build();
	dropdown.set_valign(Align::Center);
	if let Some(pos) = algorithms.iter().position(|spec| spec.name == current_algo) {
		dropdown.set_selected(pos as u32);
	}
	let level_entry = gtk::Entry::builder()
		.text(current_level.unwrap_or_default())
		.input_purpose(gtk::InputPurpose::Digits)
		.width_chars(2)
		.max_width_chars(2)
		.build();
	level_entry.set_valign(Align::Center);
	let content = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
	content.append(&dropdown);
	content.append(&level_entry);

	let row = ActionRow::builder().title(name).subtitle(i18n(description)).build();
	row.add_suffix(&content);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let row_icon = option_row_icon(row.clone());
	refresh_option_issue(row_icon.as_ref(), &ctx);
	let (ctx, name) = (ctx.clone(), name.to_string());
	let apply = Rc::new({
		let (dropdown, level_entry) = (dropdown.clone(), level_entry.clone());
		let row_icon = row_icon.clone();
		move || {
			let spec = algorithms[dropdown.selected() as usize];
			let level = level_entry.text().to_string();
			let value = if spec.levels.is_some() && !level.is_empty() {
				format!("{}:{level}", spec.name)
			} else {
				spec.name.to_string()
			};
			set_option(&ctx, FsOption::from_kv(&name, value));
			refresh_option_issue(row_icon.as_ref(), &ctx);
		}
	});
	dropdown.connect_selected_notify({
		let (apply, level_entry) = (apply.clone(), level_entry.clone());
		move |dropdown| {
			let spec = algorithms[dropdown.selected() as usize];
			if spec.levels.is_none() && !level_entry.text().is_empty() {
				level_entry.set_text("");
			}
			apply();
		}
	});
	filter_input(&level_entry, |c| c.is_ascii_digit(), move |_| apply());
}

fn add_digits_option_row<T: Display>(
	ctx: AddContext,
	trash: &gtk::Button,
	name: &str,
	description: &str,
	current: &str,
	valid_digit: impl Fn(char) -> bool + 'static,
	map_value: impl Fn(&str) -> T + 'static,
) {
	let input = gtk::Entry::builder()
		.text(current)
		.input_purpose(gtk::InputPurpose::Digits)
		.width_chars(6)
		.build();
	input.set_valign(Align::Center);
	let row = ActionRow::builder().title(name).subtitle(i18n(description)).build();
	row.add_suffix(&input);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let row_icon = option_row_icon(row.clone());
	refresh_option_issue(row_icon.as_ref(), &ctx);
	let (ctx, name) = (ctx.clone(), name.to_string());
	let row_icon = row_icon.clone();
	let apply_input = input.clone();
	filter_input(&input, valid_digit, move |cleaned| {
		let value = map_value(cleaned).to_string();
		if value.as_str() != cleaned {
			let (idle_input, idle_value) = (apply_input.clone(), value.clone());
			gtk::glib::idle_add_local(move || {
				idle_input.set_text(&idle_value);
				gtk::glib::ControlFlow::Break
			});
		}
		set_option(&ctx, FsOption::from_kv(&name, &value));
		refresh_option_issue(row_icon.as_ref(), &ctx);
	});
}

fn add_subvol_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let input = gtk::Entry::builder().text(current).hexpand(true).margin_start(12).build();
	let find_btn = build_subvol_find_button(&ctx, &input, name);
	let issue_icon = ui_commons::issue_image();
	let by_id = name == "subvolid";
	add_entry_option_row(ctx.clone(), trash, name, description, &input, &[&find_btn], Some(&issue_icon));
	run_subvol_check(&ctx, &input, &issue_icon, by_id);

	let (ctx, issue_icon) = (ctx.clone(), issue_icon.clone());
	input.connect_changed(move |input| run_subvol_check(&ctx, input, &issue_icon, by_id));
}

fn run_subvol_check(ctx: &AddContext, input: &gtk::Entry, issue_icon: &gtk::Image, by_id: bool) {
	let (entry, value) = {
		let entry = ctx.entry_ctx.entry().borrow();
		(entry.clone(), input.text().to_string())
	};
	let (input, issue_icon) = (input.clone(), issue_icon.clone());
	let (tx, rx) = std::sync::mpsc::channel();
	let thread_value = value.clone();
	std::thread::spawn(move || {
		let target = CheckValue::Subvolume {
			value: thread_value,
			by_id,
			no_permission_ask: true,
		};
		let _ = tx.send(check(&target, &entry));
	});
	if let Ok(problem) = rx.recv_timeout(Duration::from_millis(60)) {
		ui_commons::update_issue_icon(&issue_icon, problem.as_ref());
		return;
	}
	gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || match rx.try_recv() {
		Ok(problem) => {
			if input.text().as_str() == value.as_str() {
				ui_commons::update_issue_icon(&issue_icon, problem.as_ref());
			}
			gtk::glib::ControlFlow::Break
		}
		Err(std::sync::mpsc::TryRecvError::Disconnected) => gtk::glib::ControlFlow::Break,
		Err(_) => gtk::glib::ControlFlow::Continue,
	});
}

fn build_subvol_find_button(ctx: &AddContext, input: &gtk::Entry, name: &str) -> MenuButton {
	let cache: GC<Option<(DeviceValue, Vec<Subvol>)>> = GC::new(None);

	let dataset = {
		let (ctx, cache) = (ctx.clone(), cache.clone());
		move || {
			let device = ctx.entry_ctx.entry().cloned(|e| &e.device);
			let Some(path) = device.resolve_node() else {
				return Err(anyhow::anyhow!("Could not find local device for \"{}\"", device.render()));
			};
			let mut cache = cache.borrow_mut();
			let subvols = match cache.as_ref() {
				Some((cached, subvols)) if cached == &device => subvols.clone(),
				_ => list_subvolumes(&path)?,
			};
			*cache = Some((device, subvols.clone()));
			Ok(subvols)
		}
	};
	let render_row = |subvol: &Subvol| ui_commons::activatable_row(subvol.path.clone(), format!("ID {}", subvol.id));
	let filter =
		|query: &str, subvol: &Subvol| ui_commons::query_matches(query, &subvol.path) || ui_commons::query_matches(query, &subvol.id.to_string());
	let on_select = {
		let (input, ctx, name) = (input.clone(), ctx.clone(), name.to_string());
		move |subvol: Subvol, _| {
			let value = if name == "subvolid" { subvol.id.to_string() } else { subvol.path };
			input.set_text(&value);
			set_option(&ctx, FsOption::from_kv(name.clone(), value));
		}
	};

	let menu_btn = SearchPickerBuilder::new("", dataset, render_row, on_select)
		.search_placeholder(i18n("Search subvolumes"))
		.tooltip(i18n("Find a subvolume on this device"))
		.error_message(i18n("Failed to fetch subvolumes"))
		.filter(filter)
		.build();
	menu_btn.set_icon_name("folder-search-symbolic");
	menu_btn.add_css_class("flat");
	menu_btn
}

fn add_add_option_row(ctx: AddContext) {
	let entry_ref = ctx.entry_ctx.entry().borrow();
	let existing: Vec<&str> = entry_ref.options.iter().map(fs_options::FsOption::name).collect();
	let available: Vec<OptionSpec> = fs_options::options_for(&entry_ref.fs_type)
		.into_iter()
		.filter(|o| !existing.contains(&o.name))
		.collect();
	drop(entry_ref);

	let dataset = {
		let available = available.clone();
		move || Ok(available.clone())
	};
	let render_row = |option: &OptionSpec| ui_commons::activatable_row(option.name, i18n(option.description));
	let filter = |query: &str, option: &OptionSpec| {
		ui_commons::query_matches(query, option.name) || ui_commons::query_matches(query, i18n(option.description).as_str())
	};
	let on_select = {
		let ctx = ctx.clone();
		move |option: OptionSpec, _index: usize| {
			ctx.entry_ctx.entry().borrow_mut().options.push(default_option_value(option));
			build_options_group(&ctx.group, &ctx.entry_ctx);
			ctx.entry_ctx.render();
		}
	};

	let menu_btn = SearchPickerBuilder::new(i18n("Add option…"), dataset, render_row, on_select)
		.search_placeholder(i18n("Search options"))
		.tooltip(i18n("Choose an option to add to this entry"))
		.error_message(i18n("Error loading options"))
		.filter(filter)
		.build();

	let row = PreferencesRow::builder().title(i18n("Add option")).child(&menu_btn).build();
	ctx.group.add(&row);
}

fn default_option_value(option: OptionSpec) -> FsOption {
	if let Some(default) = option.default {
		return FsOption::from_kv(option.name, default);
	}
	match option.value {
		OptionValue::Toggle => FsOption::from_named(option.name),
		OptionValue::Enum(values) => FsOption::from_kv(option.name, values.first().copied().unwrap_or_default()),
		OptionValue::Compression(algorithms) => FsOption::from_kv(option.name, algorithms.first().map(|spec| spec.name).unwrap_or_default()),
		OptionValue::Integer => FsOption::from_kv(option.name, "0"),
		OptionValue::IntegerRange(min, max) => FsOption::from_kv(option.name, 0.clamp(min, max).to_string()),
		OptionValue::Octal => FsOption::from_kv(option.name, "0"),
		OptionValue::Size => FsOption::from_kv(option.name, "0"),
		OptionValue::Bool(bool_type) => FsOption::from_kv(option.name, bool_type.values().0),
		OptionValue::String => FsOption::from_kv(option.name, ""),
		OptionValue::Subvol => FsOption::from_kv(option.name, ""),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opt(value: OptionValue) -> OptionSpec {
		OptionSpec {
			name: "opt",
			description: "",
			value,
			default: None,
		}
	}

	#[test]
	fn default_option_values() {
		assert_eq!(default_option_value(opt(OptionValue::Toggle)), FsOption::from_named("opt"));
		assert_eq!(
			default_option_value(opt(OptionValue::Enum(&["a", "b", "c"]))),
			FsOption::from_kv("opt", "a")
		);
		const COMPRESSION: [CompressionSpec; 2] = [
			CompressionSpec {
				name: "a",
				levels: Some((0, 9)),
			},
			CompressionSpec { name: "b", levels: None },
		];
		assert_eq!(
			default_option_value(opt(OptionValue::Compression(&COMPRESSION))),
			FsOption::from_kv("opt", "a")
		);
		assert_eq!(default_option_value(opt(OptionValue::Integer)), FsOption::from_kv("opt", "0"));
		assert_eq!(default_option_value(opt(OptionValue::IntegerRange(-5, 5))), FsOption::from_kv("opt", "0"));
		assert_eq!(default_option_value(opt(OptionValue::IntegerRange(3, 10))), FsOption::from_kv("opt", "3"));
		assert_eq!(
			default_option_value(opt(OptionValue::IntegerRange(-10, -3))),
			FsOption::from_kv("opt", "-3")
		);
		assert_eq!(default_option_value(opt(OptionValue::Octal)), FsOption::from_kv("opt", "0"));
		assert_eq!(default_option_value(opt(OptionValue::Size)), FsOption::from_kv("opt", "0"));
		assert_eq!(
			default_option_value(opt(OptionValue::Bool(fs_options::BoolType::YesNo))),
			FsOption::from_kv("opt", "yes")
		);
		assert_eq!(
			default_option_value(opt(OptionValue::Bool(fs_options::BoolType::OneZero))),
			FsOption::from_kv("opt", "1")
		);
		assert_eq!(default_option_value(opt(OptionValue::String)), FsOption::from_kv("opt", ""));
		assert_eq!(default_option_value(opt(OptionValue::Subvol)), FsOption::from_kv("opt", ""));
	}

	#[test]
	fn documented_defaults() {
		use crate::fs_value::FsType;
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Btrfs, "compress").unwrap()),
			FsOption::from_kv("compress", "zstd")
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Btrfs, "space_cache").unwrap()),
			FsOption::from_kv("space_cache", "v2")
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Ext4, "data").unwrap()),
			FsOption::from_kv("data", "ordered")
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Tmpfs, "mode").unwrap()),
			FsOption::from_kv("mode", "01777")
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Overlay, "xino").unwrap()),
			FsOption::from_kv("xino", "auto")
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Nfs, "vers").unwrap()),
			FsOption::from_kv("vers", "4.2")
		);
	}

	#[test]
	fn from_raw_and_name() {
		assert_eq!(FsOption::from_raw("defaults"), FsOption::from_named("defaults"));
		assert_eq!(FsOption::from_raw("subvol=@home"), FsOption::from_kv("subvol", "@home"));
		assert_eq!(FsOption::from_raw("a=b=c"), FsOption::from_kv("a", "b=c"));
		assert_eq!(FsOption::from_raw("").name(), "");
		assert_eq!(FsOption::from_raw("nofail").name(), "nofail");
		assert_eq!(FsOption::from_raw("mode=0755").name(), "mode");
	}
}
