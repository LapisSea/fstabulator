use crate::stab_yurself::scan_for_backups;
use crate::subvolume::{Subvol, parse_subvolumes};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub fn make_backup() -> Result<()> {
	expect_done(crate::privileged_service::request(PrivilegedAction::MakeBackup)?)
}

pub fn write_fstab(content: &str) -> Result<()> {
	expect_done(crate::privileged_service::request(PrivilegedAction::WriteFstab(content.to_string()))?)
}

pub fn list_subvolumes(mount_point: &str) -> Result<Vec<Subvol>> {
	match crate::privileged_service::request(PrivilegedAction::ListSubvolumes(mount_point.to_string()))? {
		PrivilegedResponse::Subvolumes(subvols) => Ok(subvols),
		PrivilegedResponse::Done => bail!("The privileged helper returned an unexpected response."),
	}
}

fn expect_done(response: PrivilegedResponse) -> Result<()> {
	match response {
		PrivilegedResponse::Done => Ok(()),
		PrivilegedResponse::Subvolumes(_) => bail!("The privileged helper returned an unexpected response."),
	}
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub(crate) enum PrivilegedAction {
	MakeBackup,
	WriteFstab(String),
	ListSubvolumes(String),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "ok", content = "data")]
pub(crate) enum PrivilegedResponse {
	Done,
	Subvolumes(Vec<Subvol>),
}

pub(crate) fn execute(action: PrivilegedAction) -> Result<PrivilegedResponse> {
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
	}
}

fn create_backup() -> Result<()> {
	let backups = scan_for_backups().context("Could not scan for backups")?;

	if backups.len() >= 3 {
		let oldest = backups.iter().reduce(|a, b| if a.1 < b.1 { a } else { b }).expect("no backups found");
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
		let reason = if stderr.trim().is_empty() {
			"the listing failed".to_string()
		} else {
			stderr.trim().to_string()
		};
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
		];
		for response in responses {
			let json = serde_json::to_string(&response).unwrap();
			let parsed: PrivilegedResponse = serde_json::from_str(&json).unwrap();
			assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
		}
	}
}
