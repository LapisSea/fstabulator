use crate::stab_yurself::scan_for_backups;
use crate::subvolume::{Subvol, parse_subvolumes};
use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::credentials::{CredentialsInfo, MountCredentials, delete_credentials, inspect_credentials, mount_with_credentials};
use super::service::request;

pub(crate) fn make_backup() -> Result<()> {
	expect_done(request(PrivilegedAction::MakeBackup)?)
}

pub(crate) fn write_fstab(content: &str) -> Result<()> {
	expect_done(request(PrivilegedAction::WriteFstab(content.to_string()))?)
}

pub(crate) fn mount(mount_point: &str, device: &str, is_swap: bool, fs_type: &str, credentials: Option<MountCredentials>) -> Result<()> {
	let action = MountAction::new(mount_point, device, is_swap, fs_type, credentials);
	expect_done(request(PrivilegedAction::Mount(action))?)
}

pub(crate) fn unmount(mount_point: &str, device: &str, is_swap: bool) -> Result<()> {
	let action = MountAction::new(mount_point, device, is_swap, "", None);
	expect_done(request(PrivilegedAction::Unmount(action))?)
}

pub(crate) fn remount(mount_point: &str, is_swap: bool) -> Result<()> {
	let action = MountAction::new(mount_point, "", is_swap, "", None);
	expect_done(request(PrivilegedAction::Remount(action))?)
}

pub(crate) fn list_subvolumes(mount_point: &str) -> Result<Vec<Subvol>> {
	match request(PrivilegedAction::ListSubvolumes(mount_point.to_string()))? {
		PrivilegedResponse::Subvolumes(subvols) => Ok(subvols),
		_ => bail!("The privileged helper returned an unexpected response."),
	}
}

fn expect_done(response: PrivilegedResponse) -> Result<()> {
	match response {
		PrivilegedResponse::Done => Ok(()),
		_ => bail!("The privileged helper returned an unexpected response."),
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MountAction {
	pub mount_point: String,
	pub device: String,
	pub is_swap: bool,
	pub fs_type: String,
	#[serde(default)]
	pub credentials: Option<MountCredentials>,
}

impl MountAction {
	fn new(mount_point: &str, device: &str, is_swap: bool, fs_type: &str, credentials: Option<MountCredentials>) -> Self {
		Self {
			mount_point: mount_point.to_string(),
			device: device.to_string(),
			is_swap,
			fs_type: fs_type.to_string(),
			credentials,
		}
	}
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub(super) enum PrivilegedAction {
	MakeBackup,
	WriteFstab(String),
	ListSubvolumes(String),
	Mount(MountAction),
	Unmount(MountAction),
	Remount(MountAction),
	InspectCredentials(String),
	DeleteCredentials(String),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "ok", content = "data")]
pub(super) enum PrivilegedResponse {
	Done,
	Subvolumes(Vec<Subvol>),
	CredentialsInfo(CredentialsInfo),
}

pub(super) fn execute(action: PrivilegedAction) -> Result<PrivilegedResponse> {
	match action {
		PrivilegedAction::MakeBackup => {
			create_backup()?;
			Ok(PrivilegedResponse::Done)
		}
		PrivilegedAction::WriteFstab(content) => {
			write_fstab_to_disk(&content)?;
			Ok(PrivilegedResponse::Done)
		}
		PrivilegedAction::ListSubvolumes(mount_point) => list_btrfs_subvolumes(&mount_point).map(PrivilegedResponse::Subvolumes),
		PrivilegedAction::Mount(action) => mount_now(&action),
		PrivilegedAction::Unmount(action) => unmount_now(&action),
		PrivilegedAction::Remount(action) => remount_now(&action),
		PrivilegedAction::InspectCredentials(filename) => inspect_credentials(&filename).map(PrivilegedResponse::CredentialsInfo),
		PrivilegedAction::DeleteCredentials(filename) => delete_credentials(&filename).map(|()| PrivilegedResponse::Done),
	}
}

fn mount_now(action: &MountAction) -> Result<PrivilegedResponse> {
	if action.is_swap {
		run_command("swapon", &[action.device.as_str()])?;
		return Ok(PrivilegedResponse::Done);
	}
	if let Some(credentials) = &action.credentials {
		mount_with_credentials(action, credentials)?;
	} else {
		run_command("mount", &[action.mount_point.as_str()])?;
	}
	Ok(PrivilegedResponse::Done)
}

fn unmount_now(action: &MountAction) -> Result<PrivilegedResponse> {
	if action.is_swap {
		run_command("swapoff", &[action.device.as_str()])?;
	} else {
		run_command("umount", &[action.mount_point.as_str()])?;
	}
	Ok(PrivilegedResponse::Done)
}

fn remount_now(action: &MountAction) -> Result<PrivilegedResponse> {
	if action.is_swap {
		bail!("Swap cannot be remounted.");
	}
	run_command("mount", &["-o", "remount", action.mount_point.as_str()])?;
	Ok(PrivilegedResponse::Done)
}

pub(super) fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
	run_checked(cmd, args, None)
}

pub(super) fn run_command_with_stdin(cmd: &str, args: &[&str], input: &str) -> Result<()> {
	run_checked(cmd, args, Some(input))
}

fn run_checked(cmd: &str, args: &[&str], input: Option<&str>) -> Result<()> {
	use std::io::Write;
	use std::process::Stdio;

	let mut child = std::process::Command::new("setsid")
		.arg("--wait")
		.arg(cmd)
		.args(args)
		.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.with_context(|| format!("Could not run '{cmd}'. Is it installed?"))?;

	if let Some(input) = input {
		let mut stdin = child.stdin.take().context("Could not open the command stdin")?;
		let _ = stdin.write_all(input.as_bytes());
		let _ = stdin.write_all(b"\n");
	}

	let output = child.wait_with_output().with_context(|| format!("Could not run '{cmd}'"))?;
	if !output.status.success() {
		return Err(command_error(cmd, &String::from_utf8_lossy(&output.stderr)));
	}
	Ok(())
}

fn trimmed_or(stderr: &str, default: &str) -> String {
	let trimmed = stderr.trim();
	if trimmed.is_empty() { default.to_string() } else { trimmed.to_string() }
}

fn command_error(cmd: &str, stderr: &str) -> anyhow::Error {
	let mut reason = trimmed_or(stderr, "the command failed");
	let lower = reason.to_ascii_lowercase();
	if lower.contains("password") || lower.contains("passphrase") || lower.contains("no tty") {
		reason.push_str("\n\nThe mount requested a password, but prompting on the console is disabled. Add credentials (e.g. username=, password= or credentials= options) to the entry, or provide them in the credentials dialog.");
	}
	anyhow!("{cmd} failed: {reason}")
}

fn create_backup() -> Result<()> {
	let backups = scan_for_backups().context("Could not scan for backups")?;

	if backups.len() >= 3
		&& let Some(oldest) = backups.iter().min_by_key(|backup| backup.1)
	{
		std::fs::remove_file(&oldest.0).with_context(|| format!("Could not remove old backup {}", oldest.0.display()))?;
	}

	let backup_path = format!(
		"/etc/fstab.bak_{}",
		humantime::format_rfc3339(SystemTime::now()).to_string().replace(':', "-")
	);
	std::fs::copy("/etc/fstab", &backup_path).with_context(|| format!("Could not copy /etc/fstab to {backup_path}"))?;
	Ok(())
}

fn write_fstab_to_disk(content: &str) -> Result<()> {
	std::fs::write("/etc/fstab", content).context("Could not write /etc/fstab")?;
	Ok(())
}

fn list_btrfs_subvolumes(mount_point: &str) -> Result<Vec<Subvol>> {
	let output = std::process::Command::new("btrfs")
		.args(["subvolume", "list", mount_point])
		.output()
		.context("Could not run the 'btrfs' command. Is btrfs-progs installed?")?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		let reason = trimmed_or(&stderr, "the listing failed");
		bail!("btrfs could not list subvolumes on '{mount_point}': {reason}");
	}

	Ok(parse_subvolumes(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn action_protocol_round_trip() {
		let actions = [
			PrivilegedAction::MakeBackup,
			PrivilegedAction::WriteFstab("content".to_string()),
			PrivilegedAction::ListSubvolumes("/mnt".to_string()),
			PrivilegedAction::Mount(MountAction {
				mount_point: "/mnt".to_string(),
				device: "/dev/sda1".to_string(),
				is_swap: false,
				fs_type: "ext4".to_string(),
				credentials: None,
			}),
			PrivilegedAction::Mount(MountAction {
				mount_point: "/mnt/nas".to_string(),
				device: "//server/share".to_string(),
				is_swap: false,
				fs_type: "cifs".to_string(),
				credentials: Some(MountCredentials {
					username: Some("user".to_string()),
					password: "secret".to_string(),
					domain: None,
					filename: Some("server.cifs".to_string()),
				}),
			}),
			PrivilegedAction::Unmount(MountAction {
				mount_point: "/mnt".to_string(),
				device: "/dev/sda1".to_string(),
				is_swap: false,
				fs_type: "ext4".to_string(),
				credentials: None,
			}),
			PrivilegedAction::Remount(MountAction {
				mount_point: "/mnt".to_string(),
				device: "/dev/sda1".to_string(),
				is_swap: false,
				fs_type: "ext4".to_string(),
				credentials: None,
			}),
			PrivilegedAction::InspectCredentials("srv.cifs".to_string()),
			PrivilegedAction::DeleteCredentials("srv.cifs".to_string()),
		];
		for action in actions {
			let json = serde_json::to_string(&action).unwrap();
			let parsed: PrivilegedAction = serde_json::from_str(&json).unwrap();
			assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
		}
	}

	#[test]
	fn response_protocol_round_trip() {
		let responses = [
			PrivilegedResponse::Done,
			PrivilegedResponse::Subvolumes(vec![Subvol {
				id: 1,
				path: "@".to_string(),
			}]),
			PrivilegedResponse::CredentialsInfo(CredentialsInfo {
				exists: true,
				username: Some("alice".to_string()),
				password: "s3cret".to_string(),
				domain: Some("corp".to_string()),
			}),
		];
		for response in responses {
			let json = serde_json::to_string(&response).unwrap();
			let parsed: PrivilegedResponse = serde_json::from_str(&json).unwrap();
			assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
		}
	}
}
