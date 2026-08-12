use crate::device_value::DeviceValue;
use crate::fs_options::{FsOption, OptionValue};
use crate::render_list_entry;
use crate::search_picker::{ErrorRenderer, build_search_picker};
use crate::stab_yurself::StabEntry;
use crate::subvolume::{Subvol, list_subvolumes};
use crate::{GC, fs_options};
use adw::prelude::*;
use adw::{ActionRow, EntryRow, PreferencesGroup, PreferencesRow, SpinRow};
use gtk::{Align, Box as GtkBox, Button, CheckButton, DropDown, MenuButton, Orientation, StringList};
use std::rc::Rc;

pub fn build_options_group(group: &PreferencesGroup, entry: &GC<StabEntry>, action_row: &ActionRow, reset_btn: &gtk::Button) {
	while let Some(row) = group.row(0) {
		group.remove(&row);
	}
	let options = entry.cloned(|w| &w.options);
	let ctx = AddContext::new(group, entry, action_row, reset_btn);
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
	group: PreferencesGroup,
	entry: GC<StabEntry>,
	action_row: ActionRow,
	reset_btn: gtk::Button,
	index: usize,
	value: String,
}

impl AddContext {
	pub fn new(group: &PreferencesGroup, entry: &GC<StabEntry>, action_row: &ActionRow, reset_btn: &gtk::Button) -> Self {
		AddContext {
			group: group.clone(),
			entry: entry.clone(),
			action_row: action_row.clone(),
			reset_btn: reset_btn.clone(),
			index: 0,
			value: String::new(),
		}
	}
}

fn add_option_row(ctx: AddContext) {
	let trash = make_trash_button(&ctx);

	let (name, current) = ctx
		.value
		.split_once('=')
		.map(|(n, c)| (n.to_string(), c.to_string()))
		.unwrap_or_else(|| (ctx.value.clone(), String::new()));

	let option = fs_options::lookup(&ctx.entry.borrow().fs_type, &name);

	let row = ActionRow::builder().title(name.as_str()).build();
	if let Some(FsOption { description, .. }) = option {
		row.set_subtitle(description);
	}

	match option {
		Some(FsOption {
			description: _,
			value: OptionValue::Toggle,
			..
		}) if current.is_empty() => {
			row.add_suffix(&trash);
			ctx.group.add(&row);
		}
		Some(FsOption {
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

				let ctx = ctx.clone();
				let name = name.clone();
				dropdown.connect_selected_notify(move |dropdown| {
					let Some(selected) = model.string(dropdown.selected()) else { return };
					set_option(&ctx, format!("{name}={selected}"));
				});
			} else {
				add_free_text_option_row(ctx.clone(), &trash);
			}
		}
		Some(FsOption {
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

			let ctx = ctx.clone();
			let name = name.clone();
			check.connect_toggled(move |check| {
				let value = if check.is_active() { on } else { off };
				set_option(&ctx, format!("{name}={value}"));
			});
		}
		Some(FsOption {
			description,
			value: OptionValue::Integer,
			..
		}) => {
			add_spin_option_row(ctx.clone(), &trash, &name, description, &current, i32::MIN as f64, i32::MAX as f64);
		}
		Some(FsOption {
			description,
			value: OptionValue::IntegerRange(min, max),
			..
		}) => {
			add_spin_option_row(ctx.clone(), &trash, &name, description, &current, min as f64, max as f64);
		}
		Some(FsOption {
			description,
			value: OptionValue::String,
			..
		}) => {
			add_string_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		Some(FsOption {
			description,
			value: OptionValue::Size,
			..
		}) => {
			add_size_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		Some(FsOption {
			description,
			value: OptionValue::Octal,
			..
		}) => {
			add_octal_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		Some(FsOption {
			description,
			value: OptionValue::Subvol,
			..
		}) => {
			add_subvol_option_row(ctx.clone(), &trash, &name, description, &current);
		}
		None
		| Some(FsOption {
			value: OptionValue::Toggle, ..
		}) => {
			add_free_text_option_row(ctx.clone(), &trash);
		}
	}
}

fn make_trash_button(ctx: &AddContext) -> Button {
	let trash = gtk::Button::from_icon_name("user-trash-symbolic");
	trash.add_css_class("flat");
	trash.add_css_class("error");
	trash.set_valign(Align::Center);
	trash.set_tooltip_text(Some("Remove option"));

	let ctx = ctx.clone();
	trash.connect_clicked(move |_| {
		let entry = ctx.entry.clone();
		entry.borrow_mut().options.remove(ctx.index);
		let action_row = ctx.action_row.clone();
		let reset_btn = ctx.reset_btn.clone();
		build_options_group(&ctx.group, &ctx.entry, &ctx.action_row, &ctx.reset_btn);
		render_list_entry(&action_row, &entry.borrow(), Some(&reset_btn));
	});
	trash
}

fn set_option(ctx: &AddContext, value: String) {
	ctx.entry.borrow_mut().options[ctx.index] = value;
	render_list_entry(&ctx.action_row, &ctx.entry.borrow(), Some(&ctx.reset_btn));
}

fn add_free_text_option_row(ctx: AddContext, trash: &gtk::Button) {
	let row = EntryRow::builder().title("Option").text(ctx.value.as_str()).build();
	row.add_suffix(trash);
	ctx.group.add(&row);

	row.connect_changed(move |row| {
		set_option(&ctx, row.text().to_string());
	});
}

fn add_string_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let input = gtk::Entry::builder().text(current).hexpand(true).margin_start(12).margin_end(12).build();
	let title = gtk::Label::builder().label(name).halign(Align::Start).wrap(true).build();
	let subtitle = gtk::Label::builder().label(description).halign(Align::Start).wrap(true).build();
	subtitle.add_css_class("subtitle");
	let text = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.margin_top(6)
		.spacing(3)
		.valign(Align::Center)
		.hexpand(true)
		.build();
	text.append(&title);
	text.append(&subtitle);
	let header = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(6)
		.valign(Align::Center)
		.margin_start(12)
		.margin_end(12)
		.build();
	header.set_size_request(-1, 50);
	header.append(&text);
	header.append(trash);
	let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).margin_bottom(6).build();
	content.append(&header);
	content.append(&input);
	let row = PreferencesRow::builder().child(&content).build();
	ctx.group.add(&row);

	let ctx = ctx.clone();
	let name = name.to_string();
	input.connect_changed(move |input| {
		set_option(&ctx, format!("{name}={}", input.text()));
	});
}

fn add_spin_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str, min: f64, max: f64) {
	let row = SpinRow::builder()
		.title(name)
		.subtitle(description)
		.value(current.parse::<f64>().unwrap_or_default())
		.climb_rate(1.0)
		.numeric(true)
		.build();
	row.set_range(min, max);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let ctx = ctx.clone();
	let name = name.to_string();
	row.adjustment().connect_value_changed(move |adjustment| {
		set_option(&ctx, format!("{name}={}", adjustment.value().round() as i64));
	});
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

	let row = ActionRow::builder().title(name).subtitle(description).build();
	row.add_suffix(&content);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let ctx = ctx.clone();
	let model = model.clone();
	let name = name.to_string();
	let apply_input = input.clone();
	let apply_dropdown = dropdown.clone();
	let apply = Rc::new(move || {
		let value = match model.string(apply_dropdown.selected()).as_deref() {
			Some("B") | None => apply_input.text().to_string(),
			Some(s) => format!("{}{s}", apply_input.text()),
		};
		set_option(&ctx, format!("{name}={value}"));
	});
	input.connect_changed({
		let apply = apply.clone();
		move |_| apply()
	});
	dropdown.connect_selected_notify(move |_| apply());
}

fn add_octal_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let input = gtk::Entry::builder()
		.text(current)
		.input_purpose(gtk::InputPurpose::Digits)
		.width_chars(6)
		.build();
	input.set_valign(Align::Center);
	let row = ActionRow::builder().title(name).subtitle(description).build();
	row.add_suffix(&input);
	row.add_suffix(trash);
	ctx.group.add(&row);

	let ctx = ctx.clone();
	let name = name.to_string();
	input.connect_changed(move |input| {
		let text = input.text();
		let cleaned: String = text.chars().filter(|c| matches!(c, '0'..='7')).collect();
		if cleaned.as_str() != text.as_str() {
			let before: String = text
				.chars()
				.take(input.position().max(0) as usize)
				.filter(|c| matches!(c, '0'..='7'))
				.collect();
			input.set_text(&cleaned);
			input.set_position(before.chars().count() as i32);
			return;
		}
		set_option(&ctx, format!("{name}={cleaned}"));
	});
}

fn add_subvol_option_row(ctx: AddContext, trash: &gtk::Button, name: &str, description: &str, current: &str) {
	let input = gtk::Entry::builder().text(current).hexpand(true).build();
	let find_btn = build_subvol_find_button(&ctx, &input, name);

	let title = gtk::Label::builder().label(name).halign(Align::Start).wrap(true).build();
	let subtitle = gtk::Label::builder().label(description).halign(Align::Start).wrap(true).build();
	subtitle.add_css_class("subtitle");
	let text = GtkBox::builder()
		.orientation(Orientation::Vertical)
		.margin_top(6)
		.spacing(3)
		.valign(Align::Center)
		.hexpand(true)
		.build();
	text.append(&title);
	text.append(&subtitle);
	let header = GtkBox::builder()
		.orientation(Orientation::Horizontal)
		.spacing(6)
		.valign(Align::Center)
		.margin_start(12)
		.margin_end(12)
		.build();
	header.set_size_request(-1, 50);
	header.append(&text);
	header.append(trash);

	let input_row = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
	input_row.append(&input);
	input_row.append(&find_btn);

	let content = GtkBox::builder().orientation(Orientation::Vertical).spacing(6).margin_bottom(6).build();
	content.append(&header);
	content.append(&input_row);
	let row = PreferencesRow::builder().child(&content).build();
	ctx.group.add(&row);

	let ctx = ctx.clone();
	let name = name.to_string();
	input.connect_changed(move |input| {
		set_option(&ctx, format!("{name}={}", input.text()));
	});
}

fn build_subvol_find_button(ctx: &AddContext, input: &gtk::Entry, name: &str) -> MenuButton {
	let cache: GC<Option<(DeviceValue, Vec<Subvol>)>> = GC::new(None);

	let dataset = {
		let (ctx, cache) = (ctx.clone(), cache.clone());
		move || {
			let device = ctx.entry.cloned(|e| &e.device);
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
	let render_row = |subvol: &Subvol| {
		let row = ActionRow::builder()
			.title(subvol.path.clone())
			.subtitle(format!("ID {}", subvol.id))
			.build();
		row.set_activatable(true);
		row.upcast::<gtk::Widget>()
	};
	let render_error = |err: &anyhow::Error| {
		let label = gtk::Label::new(Some(&format!("{err:#}")));
		label.set_wrap(true);
		label.upcast::<gtk::Widget>()
	};
	let filter = |query: &str, subvol: &Subvol| {
		let query = query.trim().to_lowercase();
		query.is_empty() || subvol.path.to_lowercase().contains(&query) || subvol.id.to_string().contains(&query)
	};
	let on_select = {
		let (input, ctx, name) = (input.clone(), ctx.clone(), name.to_string());
		move |subvol: Subvol, _| {
			let value = if name == "subvolid" { subvol.id.to_string() } else { subvol.path };
			input.set_text(&value);
			set_option(&ctx, format!("{name}={value}"));
		}
	};

	let picker = build_search_picker(
		"Search subvolumes",
		"",
		"Find a subvolume on this device",
		dataset,
		render_row,
		ErrorRenderer::Message("Failed to fetch subvolumes"),
		filter,
		on_select,
	);
	picker.menu_btn.set_icon_name("folder-search-symbolic");
	picker.menu_btn.add_css_class("flat");
	picker.menu_btn
}

fn add_add_option_row(ctx: AddContext) {
	let entry_ref = ctx.entry.borrow();
	let existing: Vec<&str> = entry_ref
		.options
		.iter()
		.map(|o| o.split_once('=').map(|(n, _)| n).unwrap_or(o.as_str()))
		.collect();
	let available: Vec<FsOption> = fs_options::options_for(&entry_ref.fs_type)
		.into_iter()
		.filter(|o| !existing.contains(&o.name))
		.collect();
	drop(entry_ref);

	let dataset = {
		let available = available.clone();
		move || Ok(available.clone())
	};
	let render_row = |option: &FsOption| {
		let row = ActionRow::builder().title(option.name).subtitle(option.description).build();
		row.set_activatable(true);
		row.upcast::<gtk::Widget>()
	};
	let filter = |query: &str, option: &FsOption| {
		let query = query.trim().to_lowercase();
		query.is_empty() || option.name.to_lowercase().contains(&query) || option.description.to_lowercase().contains(&query)
	};
	let on_select = {
		let ctx = ctx.clone();
		move |option: FsOption, _index: usize| {
			ctx.entry.borrow_mut().options.push(default_option_value(option));
			build_options_group(&ctx.group, &ctx.entry, &ctx.action_row, &ctx.reset_btn);
			render_list_entry(&ctx.action_row, &ctx.entry.borrow(), Some(&ctx.reset_btn));
		}
	};

	let picker = build_search_picker(
		"Search options",
		"Add option…",
		"Choose an option to add to this entry",
		dataset,
		render_row,
		ErrorRenderer::Message("Error loading options"),
		filter,
		on_select,
	);
	let menu_btn = picker.menu_btn;

	let row = PreferencesRow::builder().title("Add option").child(&menu_btn).build();
	ctx.group.add(&row);
}

fn default_option_value(option: FsOption) -> String {
	let name = option.name;
	if let Some(default) = option.default {
		return format!("{name}={default}");
	}
	match option.value {
		OptionValue::Toggle => name.to_string(),
		OptionValue::Enum(values) => format!("{name}={}", values.first().copied().unwrap_or_default()),
		OptionValue::Integer => format!("{name}=0"),
		OptionValue::IntegerRange(min, max) => format!("{name}={}", 0.clamp(min, max)),
		OptionValue::Octal => format!("{name}=0"),
		OptionValue::Size => format!("{name}=0"),
		OptionValue::Bool(bool_type) => format!("{name}={}", bool_type.values().0),
		OptionValue::String => format!("{name}="),
		OptionValue::Subvol => format!("{name}="),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opt(value: OptionValue) -> FsOption {
		FsOption {
			name: "opt",
			description: "",
			value,
			default: None,
		}
	}

	#[test]
	fn default_option_values() {
		assert_eq!(default_option_value(opt(OptionValue::Toggle)), "opt");
		assert_eq!(default_option_value(opt(OptionValue::Enum(&["a", "b", "c"]))), "opt=a");
		assert_eq!(default_option_value(opt(OptionValue::Integer)), "opt=0");
		assert_eq!(default_option_value(opt(OptionValue::IntegerRange(-5, 5))), "opt=0");
		assert_eq!(default_option_value(opt(OptionValue::IntegerRange(3, 10))), "opt=3");
		assert_eq!(default_option_value(opt(OptionValue::IntegerRange(-10, -3))), "opt=-3");
		assert_eq!(default_option_value(opt(OptionValue::Octal)), "opt=0");
		assert_eq!(default_option_value(opt(OptionValue::Size)), "opt=0");
		assert_eq!(default_option_value(opt(OptionValue::Bool(fs_options::BoolType::YesNo))), "opt=yes");
		// assert_eq!(default_option_value(opt(OptionValue::Bool(fs_options::BoolType::TrueFalse))), "opt=true");
		assert_eq!(default_option_value(opt(OptionValue::Bool(fs_options::BoolType::OneZero))), "opt=1");
		assert_eq!(default_option_value(opt(OptionValue::String)), "opt=");
		assert_eq!(default_option_value(opt(OptionValue::Subvol)), "opt=");
	}

	#[test]
	fn documented_defaults_take_precedence() {
		use crate::fs_value::FsType;
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Btrfs, "compress").unwrap()),
			"compress=zstd"
		);
		assert_eq!(
			default_option_value(fs_options::lookup(&FsType::Btrfs, "space_cache").unwrap()),
			"space_cache=v2"
		);
		assert_eq!(default_option_value(fs_options::lookup(&FsType::Ext4, "data").unwrap()), "data=ordered");
		assert_eq!(default_option_value(fs_options::lookup(&FsType::Tmpfs, "mode").unwrap()), "mode=01777");
		assert_eq!(default_option_value(fs_options::lookup(&FsType::Overlay, "xino").unwrap()), "xino=auto");
		assert_eq!(default_option_value(fs_options::lookup(&FsType::Nfs, "vers").unwrap()), "vers=4.2");
	}
}
