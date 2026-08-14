use crate::stab_yurself::scan_for_backups;
use crate::subvolume::{Subvol, parse_subvolumes};
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::time::SystemTime;

pub fn make_backup() -> Result<()> {
	PrivilegedAction::MakeBackup.run().map(|_| ())
}

pub fn write_fstab(content: &str) -> Result<()> {
	PrivilegedAction::WriteFstab(content.to_string()).run().map(|_| ())
}

pub fn list_subvolumes(mount_point: &str) -> Result<Vec<Subvol>> {
	match PrivilegedAction::ListSubvolumes(mount_point.to_string()).run()? {
		PrivilegedResponse::Subvolumes(subvols) => Ok(subvols),
		PrivilegedResponse::Done => unreachable!(),
	}
}

enum PrivilegedAction {
	MakeBackup,
	WriteFstab(String),
	ListSubvolumes(String),
}

enum PrivilegedResponse {
	Done,
	Subvolumes(Vec<Subvol>),
}

impl PrivilegedAction {
	fn run(&self) -> Result<PrivilegedResponse> {
		match self {
			PrivilegedAction::MakeBackup => {
				create_backup()?;
				Ok(PrivilegedResponse::Done)
			}
			PrivilegedAction::WriteFstab(content) => {
				write_fstab_to_disk(content)?;
				Ok(PrivilegedResponse::Done)
			}
			PrivilegedAction::ListSubvolumes(mount_point) => {
				list_btrfs_subvolumes(mount_point).map(PrivilegedResponse::Subvolumes)
			}
		}
	}
}

fn create_backup() -> Result<()> {
	let backups = scan_for_backups().context("Could not scan for backups")?;

	let mut commands = Vec::new();

	if backups.len() >= 3 {
		let oldest = backups.iter().reduce(|a, b| if a.1 < b.1 { a } else { b }).expect("no backups found");
		let path = &oldest.0;
		commands.push(format!("rm -f '{}'", path.display()));
	}

	let backup_path = format!(
		"/etc/fstab.bak_{}",
		humantime::format_rfc3339(SystemTime::now()).to_string().replace(':', "-")
	);
	commands.push(format!("cp /etc/fstab '{}'", backup_path));

	run_pkexec_script(&commands.join(" && "))
}

fn write_fstab_to_disk(content: &str) -> Result<()> {
	let mut child = std::process::Command::new("pkexec")
		.arg("sh")
		.arg("-c")
		.arg("cat > /etc/fstab")
		.stdin(std::process::Stdio::piped())
		.spawn()
		.context("Could not launch pkexec")?;

	child.stdin.take().context("Could not open pkexec stdin")?.write_all(content.as_bytes())?;
	let status = child.wait().context("Could not wait for pkexec")?;

	if !status.success() {
		bail!("pkexec failed: {}", status);
	}

	Ok(())
}

fn list_btrfs_subvolumes(mount_point: &str) -> Result<Vec<Subvol>> {
	let output = std::process::Command::new("pkexec")
		.args(["btrfs", "subvolume", "list", mount_point])
		.output()
		.context("The 'btrfs' command needs elevated privileges to list subvolumes, but 'pkexec' could not be run. Is polkit installed?")?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let reason = if stderr.trim().is_empty() {
			"the authentication was cancelled or declined".to_string()
		} else {
			stderr.trim().to_string()
		};
		bail!("btrfs needs elevated privileges to list subvolumes on '{mount_point}': {reason}");
	}

	Ok(parse_subvolumes(&String::from_utf8_lossy(&output.stdout)))
}

fn run_pkexec_script(script: &str) -> Result<()> {
	let status = std::process::Command::new("pkexec")
		.arg("sh")
		.arg("-c")
		.arg(script)
		.status()
		.context("Could not launch pkexec")?;

	if !status.success() {
		bail!("pkexec failed: {}", status);
	}

	Ok(())
}