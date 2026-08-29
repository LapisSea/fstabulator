use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TEXT_DOMAIN: &str = "fstabulator";

fn main() {
	glib_build_tools::compile_resources(&["resources"], "resources/gresource.xml", "compiled.gresource");
	compile_locales();
}

fn compile_locales() {
	println!("cargo:rerun-if-changed=po");
	println!("cargo:rerun-if-env-changed=LOCALEDIR");
	let build_root = match std::env::var_os("OUT_DIR") {
		Some(out_dir) => PathBuf::from(out_dir).join("locale"),
		None => return,
	};
	// .mo files are always compiled into OUT_DIR; the LOCALEDIR env var
	// (set by the RPM build) overrides the directory the binary looks in
	// at runtime.
	let locale_root = match std::env::var_os("LOCALEDIR") {
		Some(dir) => PathBuf::from(dir),
		None => build_root.clone(),
	};
	println!("cargo:rustc-env=LOCALEDIR={}", locale_root.display());

	let Ok(po_files) = fs::read_dir("po") else {
		return;
	};
	for po in po_files
		.flatten()
		.map(|entry| entry.path())
		.filter(|path| path.extension().is_some_and(|ext| ext == "po"))
	{
		let Some(lang) = po.file_stem() else { continue };
		let messages_dir = build_root.join(lang).join("LC_MESSAGES");
		if let Err(err) = fs::create_dir_all(&messages_dir) {
			println!("cargo:warning=could not create {}: {err}", messages_dir.display());
			continue;
		}
		let mo = messages_dir.join(format!("{TEXT_DOMAIN}.mo"));
		match Command::new("msgfmt").arg("--check").arg("-o").arg(&mo).arg(&po).status() {
			Ok(status) if status.success() => {}
			Ok(status) => println!("cargo:warning=msgfmt failed on {} ({status})", po.display()),
			Err(err) => println!("cargo:warning=could not run msgfmt for {}: {err}", po.display()),
		}
	}
}
