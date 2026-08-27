use crate::device_value::DeviceKind;
use crate::fs_options::{self, FsOption, OptionSpec, OptionValue};
use crate::fs_value::FsType;
use crate::i18n::{i18n, i18n_fmt};
use crate::stab_yurself::StabEntry;
use crate::subvolume::{find_mount_point, list_subvolumes_at};
use crate::user_group;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProblemLevel {
	Ok,
	Warning,
	Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Problem {
	pub level: ProblemLevel,
	pub message: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CheckValue {
	Device(String),
	MountPoint(String),
	Option(FsOption),
	Subvolume { value: String, by_id: bool, no_permission_ask: bool },
}

pub fn check(value: &CheckValue, entry: &StabEntry) -> Option<Problem> {
	match value {
		CheckValue::Device(device) => device_problem(device, &entry.fs_type),
		CheckValue::MountPoint(point) => error_problem(mount_point_problem(point, &entry.fs_type)),
		CheckValue::Option(option) => option_problem(entry, option),
		CheckValue::Subvolume {
			value,
			by_id,
			no_permission_ask,
		} => subvol_problem(entry, value, *by_id, *no_permission_ask),
	}
}

fn error_problem(message: Option<String>) -> Option<Problem> {
	message.map(|message| Problem {
		level: ProblemLevel::Error,
		message,
	})
}

pub fn detect_issues(line: usize, text: &str) -> Vec<Problem> {
	let entry = match StabEntry::from(line, text) {
		Ok(entry) => entry,
		Err(err) => {
			return vec![Problem {
				level: ProblemLevel::Error,
				message: format!("{}: {err}", i18n("Could not parse entry")),
			}];
		}
	};

	let mut problems = Vec::new();

	if let Some(problem) = check(&CheckValue::MountPoint(entry.mount_point.clone()), &entry) {
		problems.push(problem);
	}

	if let Some(problem) = check(&CheckValue::Device(entry.device.render()), &entry) {
		problems.push(problem);
	}

	if matches!(entry.fs_type, FsType::Other(_)) {
		problems.push(Problem {
			level: ProblemLevel::Error,
			message: i18n_fmt("Unknown file system: {fs_type}", &[("{fs_type}", &entry.fs_type.to_string())]),
		});
	} else {
		for option in &entry.options {
			if let Some(problem) = check(&CheckValue::Option(option.clone()), &entry) {
				problems.push(problem);
			}
		}
	}

	if problems.is_empty() {
		problems.push(Problem {
			level: ProblemLevel::Ok,
			message: i18n("No problems detected"),
		});
	}
	problems
}

fn mount_point_problem(point: &str, fs_type: &FsType) -> Option<String> {
	let point = point.trim();
	if *fs_type == FsType::Swap || point.is_empty() || Path::new(point).is_dir() {
		return None;
	}
	Some(i18n_fmt("Mount point does not exist: {point}", &[("{point}", point)]))
}

fn device_problem(device: &str, fs_type: &FsType) -> Option<Problem> {
	let value = DeviceKind::classify(device, DeviceKind::for_fs_type(fs_type));
	if value.value.trim().is_empty() {
		return None;
	}
	if value.kind == DeviceKind::Other && !matches!(fs_type, FsType::Other(_)) && !DeviceKind::for_fs_type(fs_type).is_empty() {
		return Some(Problem {
			level: ProblemLevel::Error,
			message: i18n_fmt("Unknown device type: {device}", &[("{device}", device)]),
		});
	}
	let checkable = value.kind.is_local();
	if !checkable || value.resolve_node().is_some() {
		return None;
	}
	Some(Problem {
		level: ProblemLevel::Error,
		message: i18n_fmt("Device does not exist: {device}", &[("{device}", device)]),
	})
}

fn option_problem(entry: &StabEntry, option: &FsOption) -> Option<Problem> {
	if matches!(entry.fs_type, FsType::Other(_)) {
		return None;
	}
	let Some(spec) = fs_options::lookup(&entry.fs_type, option.name()) else {
		return Some(Problem {
			level: ProblemLevel::Error,
			message: i18n_fmt("Unknown option: {option}", &[("{option}", option.name())]),
		});
	};
	error_problem(option_value_problem(spec, option))
}

fn option_value_problem(spec: OptionSpec, option: &FsOption) -> Option<String> {
	let name = option.name();
	let value = match option {
		FsOption::Named(_) => None,
		FsOption::KeyValue(_, value) => Some(value),
	};
	match spec.value {
		OptionValue::Toggle => {
			if value.is_some() {
				Some(i18n_fmt("{option} should not have a value", &[("{option}", name)]))
			} else {
				None
			}
		}
		OptionValue::Enum(values) => value.filter(|value| values.contains(&value.as_str())).map_or_else(
			|| {
				Some(i18n_fmt(
					"{option} should be one of: {values}",
					&[("{option}", name), ("{values}", &values.join(", "))],
				))
			},
			|_| None,
		),
		OptionValue::Compression(algorithms) => {
			let valid = match value {
				None => true,
				Some(value) => algorithms.iter().any(|spec| spec.is_valid(value)),
			};
			if valid {
				None
			} else {
				let values: Vec<&str> = algorithms.iter().map(|spec| spec.name).collect();
				Some(i18n_fmt(
					"{option} should be one of: {values}, optionally followed by a level",
					&[("{option}", name), ("{values}", &values.join(", "))],
				))
			}
		}
		OptionValue::Integer => value
			.filter(|value| value.parse::<i64>().is_ok())
			.map_or_else(|| Some(i18n_fmt("{option} should be a number", &[("{option}", name)])), |_| None),
		OptionValue::User => user_group_value_problem(name, value, true),
		OptionValue::Group => user_group_value_problem(name, value, false),
		OptionValue::IntegerRange(min, max) => value
			.and_then(|value| value.parse::<i64>().ok())
			.filter(|value| (min..=max).contains(value))
			.map_or_else(
				|| {
					Some(i18n_fmt(
						"{option} should be a number between {min} and {max}",
						&[("{option}", name), ("{min}", &min.to_string()), ("{max}", &max.to_string())],
					))
				},
				|_| None,
			),
		OptionValue::Octal => value
			.filter(|value| !value.is_empty() && value.chars().all(|c| matches!(c, '0'..='7')))
			.map_or_else(|| Some(i18n_fmt("{option} should be an octal number", &[("{option}", name)])), |_| None),
		OptionValue::Size => value.filter(|value| is_size_value(value)).map_or_else(
			|| {
				Some(i18n_fmt(
					"{option} should be a number with a size unit, for example 16K",
					&[("{option}", name)],
				))
			},
			|_| None,
		),
		OptionValue::Bool(bool_type) => {
			let (on, off) = bool_type.values();
			value
				.filter(|value| {
					let lowered = value.to_ascii_lowercase();
					lowered == on || lowered == off
				})
				.map_or_else(
					|| {
						Some(i18n_fmt(
							"{option} should be {on} or {off}",
							&[("{option}", name), ("{on}", on), ("{off}", off)],
						))
					},
					|_| None,
				)
		}
		OptionValue::Subvol => None,
		OptionValue::String => credentials_option_problem(name, value.map(|value| value.as_str())),
	}
}

fn user_group_value_problem(name: &str, value: Option<&String>, is_user: bool) -> Option<String> {
	let not_a_number = i18n_fmt("{option} should be a number", &[("{option}", name)]);
	let id = match value {
		Some(value) => match value.parse::<u32>() {
			Ok(id) => id,
			Err(_) => return Some(not_a_number),
		},
		None => return Some(not_a_number),
	};
	let list = if is_user { user_group::users() } else { user_group::groups() };
	let Ok(list) = list else { return None };
	if list.iter().any(|entry| entry.id == id) {
		return None;
	}
	let (message, id) = if is_user {
		("User does not exist: {id}", id.to_string())
	} else {
		("Group does not exist: {id}", id.to_string())
	};
	Some(i18n_fmt(message, &[("{id}", &id)]))
}

fn credentials_option_problem(name: &str, value: Option<&str>) -> Option<String> {
	if matches!(name, "credentials" | "cred")
		&& let Some(value) = value
		&& credentials_file_missing(value)
	{
		return Some(i18n_fmt("Credentials file does not exist: {path}", &[("{path}", value)]));
	}
	None
}

fn credentials_file_missing(value: &str) -> bool {
	let path = if value.starts_with('/') {
		PathBuf::from(value)
	} else {
		match std::env::var_os("HOME") {
			Some(home) => Path::new(&home).join(".config/libcifs").join(value),
			None => PathBuf::from(value),
		}
	};
	!path.is_file()
}

fn is_size_value(value: &str) -> bool {
	let num_len = value
		.char_indices()
		.find(|(_, c)| !c.is_ascii_digit() && *c != '.')
		.map(|(idx, _)| idx)
		.unwrap_or(value.len());
	let (number, unit) = value.split_at(num_len);
	!number.is_empty()
		&& number.chars().all(|c| c.is_ascii_digit() || c == '.')
		&& (unit.is_empty() || (unit.len() == 1 && "BKMGTPE%".contains(unit.to_ascii_uppercase().as_str())))
}

enum SubvolCheck {
	Found,
	Missing,
	NotMounted,
	Skipped,
	Uncheckable(String),
}

fn check_subvol(entry: &StabEntry, value: &str, by_id: bool, no_permission_ask: bool) -> SubvolCheck {
	if value.trim().is_empty() {
		return SubvolCheck::Found;
	}
	let Some(node) = entry.device.resolve_node() else {
		return SubvolCheck::Uncheckable(String::new());
	};
	let source = node.to_string_lossy();
	let mount_point = match find_mount_point(source.as_ref()) {
		Ok(Some(mount_point)) => mount_point,
		Ok(None) => return SubvolCheck::NotMounted,
		Err(err) => return SubvolCheck::Uncheckable(err.to_string()),
	};
	let subvols = match list_subvolumes_at(&mount_point, !no_permission_ask) {
		Ok(subvols) => subvols,
		Err(err) => {
			if no_permission_ask {
				return SubvolCheck::Skipped;
			}
			return SubvolCheck::Uncheckable(err.to_string());
		}
	};
	let found = subvols
		.iter()
		.any(|subvol| if by_id { subvol.id.to_string() == value } else { subvol.path == value });
	if found { SubvolCheck::Found } else { SubvolCheck::Missing }
}

pub fn check_subvols_in_text(line: usize, text: &str) -> Vec<Problem> {
	let Ok(entry) = StabEntry::from(line, text) else {
		return Vec::new();
	};
	subvol_option_problems(&entry)
}

fn subvol_option_problems(entry: &StabEntry) -> Vec<Problem> {
	let mut problems = Vec::new();
	for option in &entry.options {
		let Some(spec) = fs_options::lookup(&entry.fs_type, option.name()) else {
			continue;
		};
		if !matches!(spec.value, OptionValue::Subvol) {
			continue;
		}
		let FsOption::KeyValue(_, value) = option else {
			continue;
		};
		if let Some(problem) = subvol_problem(entry, value, option.name() == "subvolid", false) {
			problems.push(problem);
		}
	}
	problems
}

fn subvol_problem(entry: &StabEntry, value: &str, by_id: bool, no_permission_ask: bool) -> Option<Problem> {
	match check_subvol(entry, value, by_id, no_permission_ask) {
		SubvolCheck::Found => None,
		SubvolCheck::Skipped => Some(Problem {
			level: ProblemLevel::Warning,
			message: i18n("Could not check subvolume"),
		}),
		SubvolCheck::Missing => Some(Problem {
			level: ProblemLevel::Error,
			message: i18n_fmt("Subvolume does not exist: {subvol}", &[("{subvol}", value)]),
		}),
		SubvolCheck::NotMounted => Some(Problem {
			level: ProblemLevel::Warning,
			message: i18n("Could not check subvolume: the device is not mounted"),
		}),
		SubvolCheck::Uncheckable(reason) => Some(Problem {
			level: ProblemLevel::Warning,
			message: if reason.is_empty() {
				i18n("Could not check subvolume")
			} else {
				i18n_fmt("Could not check subvolume: {error}", &[("{error}", &reason)])
			},
		}),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unparseable_text_is_an_error() {
		let issues = detect_issues(3, "not a valid entry");
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].level, ProblemLevel::Error);
	}

	#[test]
	fn clean_entry_reports_ok() {
		let issues = detect_issues(0, "none / tmpfs defaults 0 0");
		assert_eq!(issues.len(), 1);
		assert_eq!(issues[0].level, ProblemLevel::Ok);
	}

	#[test]
	fn missing_mount_point_is_an_error() {
		let issues = detect_issues(0, "none /no/such/mount/point tmpfs defaults 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("/no/such/mount/point"))
		);
	}

	#[test]
	fn swap_entry_skips_mount_point_check() {
		let issues = detect_issues(0, "/dev/zram0 none swap defaults 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("Mount point does not exist")));
	}

	#[test]
	fn swap_file_is_a_recognized_device_type() {
		let issues = detect_issues(0, "/swapfile none swap defaults 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("Unknown device type")));
	}

	#[test]
	fn missing_swap_file_is_an_error() {
		let issues = detect_issues(0, "/no/such/swapfile none swap defaults 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("Device does not exist"))
		);
	}

	#[test]
	fn missing_device_and_unknown_option_are_errors() {
		let issues = detect_issues(0, "UUID=12345678-1234-1234-1234-123456789abc /mnt/nope ext4 bogus_option 0 2");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("UUID=12345678"))
		);
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("bogus_option"))
		);
	}

	#[test]
	fn unknown_file_system_is_an_error_and_skips_option_checks() {
		let issues = detect_issues(0, "none /mnt/x borkfs bogus_option 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("borkfs"))
		);
		assert!(!issues.iter().any(|issue| issue.message.contains("bogus_option")));
	}

	#[test]
	fn unknown_device_type_warns() {
		let issues = detect_issues(0, "LABEL=boot /mnt/x cifs defaults 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("LABEL=boot"))
		);
	}

	#[test]
	fn invalid_option_values_are_errors() {
		let issues = detect_issues(0, "none / tmpfs nofail=1,X-mount.nocanonicalize=wrong 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("nofail"))
		);
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("X-mount.nocanonicalize"))
		);
	}

	#[test]
	fn non_numeric_integer_option_is_an_error() {
		let issues = detect_issues(0, "none /mnt/x ext2 resgid=abc 0 2");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("resgid"))
		);
	}

	#[test]
	fn unknown_uid_is_an_error() {
		let issues = detect_issues(0, "none /mnt/x udf uid=4294967295 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("User does not exist"))
		);
	}

	#[test]
	fn unknown_gid_is_an_error() {
		let issues = detect_issues(0, "none /mnt/x udf gid=4294967295 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("Group does not exist"))
		);
	}

	#[test]
	fn known_uid_and_gid_are_ok() {
		let issues = detect_issues(0, "none /mnt/x udf uid=0,gid=0 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("User does not exist")));
		assert!(!issues.iter().any(|issue| issue.message.contains("Group does not exist")));
	}

	#[test]
	fn compression_option_accepts_bare_algorithm_and_level() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:3 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:0 0 0");
		assert!(!issues.iter().any(|issue| issue.message.contains("compress")));
	}

	#[test]
	fn compression_option_rejects_out_of_range_level() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zstd:16 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("compress"))
		);
		let issues = detect_issues(0, "none /mnt/x btrfs compress=zlib:10 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("compress"))
		);
	}

	#[test]
	fn compression_option_rejects_level_for_levelless_algorithm() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=lzo:3 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("compress"))
		);
	}

	#[test]
	fn compression_option_rejects_unknown_algorithm() {
		let issues = detect_issues(0, "none /mnt/x btrfs compress=bork:3 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("compress"))
		);
	}

	#[test]
	fn missing_credentials_file_is_an_error() {
		let issues = detect_issues(0, "//server/share /mnt/x cifs credentials=/nonexistent/fstabulator-test-cred 0 0");
		assert!(
			issues
				.iter()
				.any(|issue| issue.level == ProblemLevel::Error && issue.message.contains("fstabulator-test-cred"))
		);
	}

	#[test]
	fn existing_credentials_file_is_ok() {
		let dir = std::env::temp_dir().join(format!("fstabulator-cred-test-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let file = dir.join("cred");
		std::fs::write(&file, "username=user\n").unwrap();
		let line = format!("//server/share /mnt/x cifs credentials={} 0 0", file.display());
		let issues = detect_issues(0, &line);
		assert!(!issues.iter().any(|issue| issue.message.contains("Credentials file does not exist")));
		let _ = std::fs::remove_dir_all(&dir);
	}

	#[test]
	fn empty_subvol_value_is_found() {
		let entry = StabEntry::blank(0);
		assert!(matches!(check_subvol(&entry, "   ", false, true), SubvolCheck::Found));
	}

	#[test]
	fn check_subvols_in_text_without_subvol_is_empty() {
		assert!(check_subvols_in_text(0, "none / tmpfs defaults 0 0").is_empty());
		assert!(check_subvols_in_text(0, "not a valid entry").is_empty());
	}

	#[test]
	fn check_subvols_in_text_uncheckable_device_warns() {
		let problems = check_subvols_in_text(0, "none /mnt/x btrfs subvol=@home 0 0");
		assert!(
			problems
				.iter()
				.any(|problem| problem.level == ProblemLevel::Warning && problem.message.contains("subvolume"))
		);
	}
}
