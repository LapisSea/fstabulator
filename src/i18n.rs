use chrono::{DateTime, Local, Locale};
use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain};
use std::path::Path;
use std::time::SystemTime;

const TEXT_DOMAIN: &str = "fstabulator";

pub fn init() {
	// SAFETY: called once at startup before any locale-dependent work on the main thread.
	unsafe { setlocale(LocaleCategory::LcAll, "") };
	// A missing catalog is fine: gettext falls back to the untranslated msgid.
	let _ = bindtextdomain(TEXT_DOMAIN, Path::new(env!("LOCALEDIR")));
	let _ = bind_textdomain_codeset(TEXT_DOMAIN, "UTF-8");
	let _ = textdomain(TEXT_DOMAIN);
}

pub fn i18n(msgid: &str) -> String {
	gettext(msgid)
}

/// Translates msgid then substitutes named placeholders like "{path}".
/// Needed because format! requires a literal template.
pub fn i18n_fmt(msgid: &str, replacements: &[(&str, &str)]) -> String {
	replacements
		.iter()
		.fold(gettext(msgid), |text, (token, value)| text.replace(token, value))
}

pub fn localized_datetime(time: SystemTime) -> String {
	let locale = gtk_locale_to_chrono();
	let local_time: DateTime<Local> = time.into();
	local_time.format_localized("%A, %d %B %Y - %H:%M", locale).to_string()
}

fn gtk_locale_to_chrono() -> Locale {
	let lang_names = glib::language_names();
	for name in lang_names.iter() {
		if let Some(locale) = str_to_chrono_locale(name.as_str()) {
			return locale;
		}
	}
	Locale::en_US
}

fn str_to_chrono_locale(name: &str) -> Option<Locale> {
	let base = name.split('.').next().unwrap_or(name);
	match base {
		"hr_HR" | "hr" => Some(Locale::hr_HR),
		"de_DE" | "de" => Some(Locale::de_DE),
		"en_US" | "en" => Some(Locale::en_US),
		"fr_FR" | "fr" => Some(Locale::fr_FR),
		"ja_JP" | "ja" => Some(Locale::ja_JP),
		_ => None,
	}
}
