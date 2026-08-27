use anyhow::anyhow;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct NamedId {
	pub name: String,
	pub id: u32,
}

pub fn users() -> anyhow::Result<Vec<NamedId>> {
	Ok(parse_passwd(&run_getent("passwd")?))
}

pub fn groups() -> anyhow::Result<Vec<NamedId>> {
	Ok(parse_group(&run_getent("group")?))
}

fn run_getent(database: &str) -> anyhow::Result<String> {
	let output = Command::new("getent").arg(database).output()?;
	match output.status.code() {
		Some(0) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
		Some(2) => Ok(String::new()),
		_ => {
			let stderr = String::from_utf8_lossy(&output.stderr);
			Err(anyhow!("getent {database} failed: {stderr}"))
		}
	}
}

pub fn parse_passwd(text: &str) -> Vec<NamedId> {
	let mut users = Vec::new();
	for line in text.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		// name:passwd:uid:gid:gecos:home:shell
		let fields: Vec<&str> = line.split(':').collect();
		if fields.len() < 3 {
			continue;
		}
		let uid = match fields[2].parse::<u32>() {
			Ok(uid) => uid,
			Err(_) => continue,
		};
		users.push(NamedId {
			name: fields[0].to_string(),
			id: uid,
		});
	}
	users
}

pub fn parse_group(text: &str) -> Vec<NamedId> {
	let mut groups = Vec::new();
	for line in text.lines() {
		let line = line.trim();
		if line.is_empty() {
			continue;
		}
		// name:passwd:gid:member1,member2,...
		let fields: Vec<&str> = line.split(':').collect();
		if fields.len() < 3 {
			continue;
		}
		let gid = match fields[2].parse::<u32>() {
			Ok(gid) => gid,
			Err(_) => continue,
		};
		groups.push(NamedId {
			name: fields[0].to_string(),
			id: gid,
		});
	}
	groups
}

#[cfg(test)]
mod tests {
	use super::*;

	const PASSWD: &str = "\
root:x:0:0:root:/root:/usr/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
alice:x:1000:1000:Alice Doe,Room 42:/home/alice:/usr/bin/zsh
broken:x:notanumber:2
";

	const GROUPS: &str = "\
root:x:0:
adm:x:4:alice
sudo:x:27:alice,bob
";

	#[test]
	fn parse_passwd_reads_name_and_uid_and_skips_bad_lines() {
		let users = parse_passwd(PASSWD);
		assert_eq!(users.len(), 3);
		assert_eq!(users[0].name, "root");
		assert_eq!(users[0].id, 0);
		assert_eq!(users[2].name, "alice");
		assert_eq!(users[2].id, 1000);
	}

	#[test]
	fn parse_passwd_requires_uid_field() {
		assert!(parse_passwd("no_uid:x").is_empty());
		assert!(parse_passwd("bare:x:11").len() == 1);
	}

	#[test]
	fn parse_group_reads_name_and_gid() {
		let groups = parse_group(GROUPS);
		assert_eq!(groups.len(), 3);
		assert_eq!(groups[0].name, "root");
		assert_eq!(groups[0].id, 0);
		assert_eq!(groups[2].name, "sudo");
		assert_eq!(groups[2].id, 27);
	}

	#[test]
	fn parse_group_requires_gid_field() {
		assert!(parse_group("nonsense:x").is_empty());
	}
}
