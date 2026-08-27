use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

use super::actions::{MountAction, PrivilegedAction, PrivilegedResponse, run_command, run_command_with_stdin};
use super::service::request;

pub(super) const CREDENTIALS_DIR: &str = "/etc/fstab.credentials.d";

pub(crate) fn saved_credentials_path(filename: &str) -> std::path::PathBuf {
	Path::new(CREDENTIALS_DIR).join(filename)
}

pub(crate) fn inspect_credentials_file(filename: &str) -> Result<CredentialsInfo> {
	match request(PrivilegedAction::InspectCredentials(filename.to_string()))? {
		PrivilegedResponse::CredentialsInfo(info) => Ok(info),
		PrivilegedResponse::Done => bail!("The privileged helper returned an unexpected response."),
		PrivilegedResponse::Subvolumes(_) => bail!("The privileged helper returned an unexpected response."),
	}
}

pub(crate) fn delete_credentials_file(filename: &str) -> Result<()> {
	match request(PrivilegedAction::DeleteCredentials(filename.to_string()))? {
		PrivilegedResponse::Done => Ok(()),
		PrivilegedResponse::Subvolumes(_) => bail!("The privileged helper returned an unexpected response."),
		PrivilegedResponse::CredentialsInfo(_) => bail!("The privileged helper returned an unexpected response."),
	}
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct MountCredentials {
	pub username: Option<String>,
	pub password: String,
	pub domain: Option<String>,
	/// When set, the credentials are persisted to `/etc/fstab.credentials.d/<filename>`
	/// instead of a temporary file, so the fstab entry can reference them on boot.
	#[serde(default)]
	pub filename: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CredentialsInfo {
	/// Whether the credentials file exists on disk.
	pub exists: bool,
	#[serde(default)]
	pub username: Option<String>,
	#[serde(default)]
	pub password: String,
	#[serde(default)]
	pub domain: Option<String>,
}

pub(super) fn mount_with_credentials(action: &MountAction, credentials: &MountCredentials) -> Result<()> {
	match action.fs_type.as_str() {
		"cifs" | "smb3" => {
			let (path, persistent) = match &credentials.filename {
				Some(filename) => (save_credentials_file(filename, credentials)?, true),
				None => (write_credentials_file(credentials)?, false),
			};
			let result = run_command("mount", &["-o", &format!("credentials={}", path.display()), action.mount_point.as_str()]);
			if !persistent {
				let _ = std::fs::remove_file(&path);
			}
			result
		}
		"fuse.sshfs" => {
			let mut options = String::from("password_stdin");
			if let Some(username) = &credentials.username
				&& !username.is_empty()
			{
				options.push_str(",user=");
				options.push_str(username);
			}
			run_command_with_stdin("mount", &["-o", &options, action.mount_point.as_str()], &credentials.password)
		}
		_ => run_command("mount", &[action.mount_point.as_str()]),
	}
}

fn credentials_content(credentials: &MountCredentials) -> String {
	let mut content = String::new();
	if let Some(username) = &credentials.username {
		content.push_str(&format!("username={username}\n"));
	}
	content.push_str(&format!("password={}\n", credentials.password));
	if let Some(domain) = &credentials.domain {
		content.push_str(&format!("domain={domain}\n"));
	}
	content
}

fn save_credentials_file(filename: &str, credentials: &MountCredentials) -> Result<std::path::PathBuf> {
	save_credentials_file_in(Path::new(CREDENTIALS_DIR), filename, credentials)
}

fn save_credentials_file_in(dir: &Path, filename: &str, credentials: &MountCredentials) -> Result<std::path::PathBuf> {
	validate_credentials_filename(filename)?;
	std::fs::create_dir_all(dir).with_context(|| format!("Could not create directory {}", dir.display()))?;
	let path = dir.join(filename);
	write_credentials_to(&path, credentials, false)?;
	Ok(path)
}

fn validate_credentials_filename(filename: &str) -> Result<()> {
	if filename.is_empty() || filename == "." || filename == ".." || filename.contains(['/', '\\', '\0']) {
		bail!("Invalid credentials file name {filename:?}. Use a plain file name without path separators.");
	}
	Ok(())
}

fn write_credentials_to(path: &Path, credentials: &MountCredentials, create_new: bool) -> Result<()> {
	use std::io::Write;
	use std::os::unix::fs::OpenOptionsExt;

	let mut options = std::fs::OpenOptions::new();
	options.write(true).mode(0o600);
	if create_new {
		options.create_new(true);
	} else {
		options.create(true).truncate(true);
	}
	let mut file = options
		.open(path)
		.with_context(|| format!("Could not create credentials file {}", path.display()))?;
	file.write_all(credentials_content(credentials).as_bytes())
		.context("Could not write credentials file")?;
	Ok(())
}

pub(super) fn inspect_credentials(filename: &str) -> Result<CredentialsInfo> {
	validate_credentials_filename(filename)?;
	read_credentials_file(&saved_credentials_path(filename))
}

pub(super) fn delete_credentials(filename: &str) -> Result<()> {
	validate_credentials_filename(filename)?;
	let path = saved_credentials_path(filename);
	match std::fs::remove_file(&path) {
		Ok(()) => Ok(()),
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(err) => Err(err).with_context(|| format!("Could not delete credentials file {}", path.display())),
	}
}

fn read_credentials_file(path: &Path) -> Result<CredentialsInfo> {
	match std::fs::read_to_string(path) {
		Ok(content) => Ok(read_credentials_from_content(&content, true)),
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(CredentialsInfo {
			exists: false,
			username: None,
			password: String::new(),
			domain: None,
		}),
		Err(err) => Err(err).with_context(|| format!("Could not read credentials file {}", path.display())),
	}
}

fn read_credentials_from_content(content: &str, exists: bool) -> CredentialsInfo {
	let mut username = None;
	let mut password = String::new();
	let mut domain = None;
	for line in content.lines() {
		let Some((key, value)) = line.split_once('=') else {
			continue;
		};
		match key {
			"username" => username = Some(value.to_string()),
			"password" => password = value.to_string(),
			"domain" => domain = Some(value.to_string()),
			_ => {}
		}
	}
	CredentialsInfo {
		exists,
		username,
		password,
		domain,
	}
}

fn write_credentials_file(credentials: &MountCredentials) -> Result<std::path::PathBuf> {
	let file_name = format!(
		"fstabulator-credentials-{}-{}.cifs",
		std::process::id(),
		SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos())
	);
	let path = std::env::temp_dir().join(file_name);
	write_credentials_to(&path, credentials, true)?;
	Ok(path)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn test_creds_dir() -> std::path::PathBuf {
		let nanos = SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
		std::env::temp_dir().join(format!("fstabulator-test-{}-{}", std::process::id(), nanos))
	}

	#[test]
	fn credentials_file_contains_fields() {
		let credentials = MountCredentials {
			username: Some("alice".to_string()),
			password: "s3cret".to_string(),
			domain: Some("corp".to_string()),
			filename: None,
		};
		let path = write_credentials_file(&credentials).unwrap();
		let content = std::fs::read_to_string(&path).unwrap();
		assert!(content.contains("username=alice\n"));
		assert!(content.contains("password=s3cret\n"));
		assert!(content.contains("domain=corp\n"));
		let _ = std::fs::remove_file(&path);
	}

	#[test]
	fn saved_credentials_file() {
		let dir = test_creds_dir();
		let credentials = MountCredentials {
			username: Some("bob".to_string()),
			password: "hunter2".to_string(),
			domain: None,
			filename: Some("srv.cifs".to_string()),
		};
		let path = save_credentials_file_in(&dir, "srv.cifs", &credentials).unwrap();
		assert_eq!(path, dir.join("srv.cifs"));
		assert_eq!(path.parent().unwrap().to_str().unwrap(), dir.to_str().unwrap());
		let content = std::fs::read_to_string(&path).unwrap();
		assert!(content.contains("username=bob\n"));
		assert!(content.contains("password=hunter2\n"));
		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_dir(&dir);
	}

	#[test]
	fn saved_credentials_file_overrides() {
		let dir = test_creds_dir();
		let first = MountCredentials {
			username: Some("alice".to_string()),
			password: "one".to_string(),
			domain: None,
			filename: Some("srv.cifs".to_string()),
		};
		let second = MountCredentials {
			username: Some("bob".to_string()),
			password: "two".to_string(),
			domain: None,
			filename: Some("srv.cifs".to_string()),
		};
		let path = save_credentials_file_in(&dir, "srv.cifs", &first).unwrap();
		let path2 = save_credentials_file_in(&dir, "srv.cifs", &second).unwrap();
		assert_eq!(path, path2);
		let content = std::fs::read_to_string(&path).unwrap();
		assert!(content.contains("username=bob\n"));
		assert!(content.contains("password=two\n"));
		assert!(!content.contains("alice"));
		assert!(!content.contains("one"));
		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_dir(&dir);
	}

	#[test]
	fn inspect_credentials() {
		let dir = test_creds_dir();
		let credentials = MountCredentials {
			username: Some("alice".to_string()),
			password: "s3cret".to_string(),
			domain: Some("corp".to_string()),
			filename: None,
		};
		let path = save_credentials_file_in(&dir, "srv.cifs", &credentials).unwrap();
		let info = read_credentials_file(&path).unwrap();
		assert!(info.exists);
		assert_eq!(info.username.as_deref(), Some("alice"));
		assert_eq!(info.password, "s3cret");
		assert_eq!(info.domain.as_deref(), Some("corp"));
		let missing = read_credentials_file(&dir.join("absent.cifs")).unwrap();
		assert!(!missing.exists);
		assert_eq!(missing.password, "");
		assert!(missing.username.is_none());
		assert!(missing.domain.is_none());
		let _ = std::fs::remove_file(&path);
		let _ = std::fs::remove_dir(&dir);
	}

	#[test]
	fn reject_bad_names() {
		let dir = test_creds_dir();
		let credentials = MountCredentials {
			username: None,
			password: "pw".to_string(),
			domain: None,
			filename: None,
		};
		for name in ["", "..", "a/b", "../evil", "a\\b"] {
			assert!(save_credentials_file_in(&dir, name, &credentials).is_err(), "should reject {name:?}");
		}
		let _ = std::fs::remove_dir(&dir);
	}
}
