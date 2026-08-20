use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct BlockDeviceInfo {
	pub name: String,
	pub path: String,
	pub uuid: Option<String>,
	pub partuuid: Option<String>,
	pub label: Option<String>,
	pub partlabel: Option<String>,
	pub fstype: Option<String>,
	pub size: Option<String>,
	pub model: Option<String>,
	pub mountpoints: Vec<String>,
}

#[derive(Deserialize)]
struct RawBlockDevice {
	name: String,
	path: String,
	#[serde(default)]
	uuid: Option<String>,
	#[serde(default)]
	partuuid: Option<String>,
	#[serde(default)]
	label: Option<String>,
	#[serde(default)]
	partlabel: Option<String>,
	#[serde(default)]
	fstype: Option<String>,
	#[serde(default)]
	size: Option<String>,
	#[serde(default)]
	model: Option<String>,
	#[serde(default)]
	mountpoints: Vec<String>,
	#[serde(default)]
	children: Vec<RawBlockDevice>,
}

impl From<RawBlockDevice> for BlockDeviceInfo {
	fn from(raw: RawBlockDevice) -> Self {
		BlockDeviceInfo {
			name: raw.name,
			path: raw.path,
			uuid: raw.uuid,
			partuuid: raw.partuuid,
			label: raw.label,
			partlabel: raw.partlabel,
			fstype: raw.fstype,
			size: raw.size,
			model: raw.model,
			mountpoints: raw.mountpoints,
		}
	}
}

#[derive(Deserialize)]
struct LsblkOutput {
	blockdevices: Vec<RawBlockDevice>,
}

pub fn list_block_devices() -> Result<Vec<BlockDeviceInfo>> {
	let output = Command::new("lsblk")
		.args(["-J", "-o", "NAME,PATH,UUID,PARTUUID,LABEL,PARTLABEL,FSTYPE,SIZE,MODEL,MOUNTPOINTS"])
		.output()
		.context("Could not run 'lsblk'. Is util-linux installed?")?;
	if !output.status.success() {
		bail!("lsblk exited with: {}", String::from_utf8_lossy(&output.stderr).trim());
	}
	let stdout = String::from_utf8_lossy(&output.stdout);
	parse_lsblk(&stdout)
}

fn parse_lsblk(stdout: &str) -> Result<Vec<BlockDeviceInfo>> {
	let output = serde_json::from_str::<LsblkOutput>(stdout).context("Could not parse lsblk output")?;
	let mut devices = Vec::new();
	collect_devices(output.blockdevices, None, &mut devices);
	Ok(devices)
}

fn collect_devices(raw: Vec<RawBlockDevice>, parent_model: Option<&str>, out: &mut Vec<BlockDeviceInfo>) {
	for mut device in raw {
		if device.model.is_none() {
			device.model = parent_model.map(str::to_string);
		}
		let model = device.model.clone();
		let children = std::mem::take(&mut device.children);
		out.push(device.into());
		collect_devices(children, model.as_deref(), out);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_devices_with_children() {
		let json = r#"{
			"blockdevices": [
				{
					"name": "nvme0n1",
					"path": "/dev/nvme0n1",
					"uuid": null,
					"partuuid": null,
					"label": null,
					"partlabel": null,
					"fstype": null,
					"size": "953.9G",
					"model": "HFS001TEJ9X110N",
					"mountpoints": [],
					"children": [
						{
							"name": "nvme0n1p1",
							"path": "/dev/nvme0n1p1",
							"uuid": "65D3-6417",
							"partuuid": "5344a3fd-c59c-4dcd-a809-7371291cb33f",
							"label": null,
							"partlabel": "EFI System Partition",
							"fstype": "vfat",
							"size": "600M",
							"model": null,
							"mountpoints": ["/boot/efi"]
						},
						{
							"name": "zram0",
							"path": "/dev/zram0",
							"uuid": "7f31c9e7-0576-4bfb-bf21-3970423d8398",
							"partuuid": null,
							"label": "zram0",
							"partlabel": null,
							"fstype": "swap",
							"size": "16G",
							"model": null,
							"mountpoints": ["[SWAP]"]
						}
					]
				}
			]
		}"#;
		let devices = parse_lsblk(json).unwrap();
		assert_eq!(devices.len(), 3);
		assert_eq!(devices[0].name, "nvme0n1");
		assert_eq!(devices[0].uuid, None);
		assert_eq!(devices[0].model.as_deref(), Some("HFS001TEJ9X110N"));
		assert_eq!(devices[1].path, "/dev/nvme0n1p1");
		assert_eq!(devices[1].partlabel.as_deref(), Some("EFI System Partition"));
		assert_eq!(devices[1].mountpoints, vec!["/boot/efi".to_string()]);
		assert_eq!(devices[1].model.as_deref(), Some("HFS001TEJ9X110N"));
		assert_eq!(devices[2].label.as_deref(), Some("zram0"));
		assert_eq!(devices[2].mountpoints, vec!["[SWAP]".to_string()]);
		assert_eq!(devices[2].model.as_deref(), Some("HFS001TEJ9X110N"));
	}

	#[test]
	fn invalid_json_yields_error() {
		assert!(parse_lsblk("not json").is_err());
	}

	#[test]
	fn missing_fields_default_to_empty() {
		let json = r#"{"blockdevices":[{"name":"sda","path":"/dev/sda"}]}"#;
		let devices = parse_lsblk(json).unwrap();
		assert_eq!(devices.len(), 1);
		assert_eq!(devices[0].uuid, None);
		assert!(devices[0].mountpoints.is_empty());
	}
}
