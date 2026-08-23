use gettextrs::{LocaleCategory, bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain};
use std::path::Path;

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
