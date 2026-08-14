use crate::privileged_actions::{PrivilegedAction, PrivilegedResponse};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

#[derive(Serialize, Deserialize)]
struct Greeting {
	ready: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
enum ServiceResponse {
	Ok(PrivilegedResponse),
	Err(String),
}

struct PrivilegedService {
	_child: Child,
	stdin: BufWriter<ChildStdin>,
	stdout: BufReader<ChildStdout>,
	dead: bool,
}

impl PrivilegedService {
	fn spawn() -> Result<Self> {
		let exe = std::env::current_exe().context("Could not determine the current executable")?;
		let mut child = Command::new("pkexec")
			.arg(&exe)
			.arg("--root-helper")
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::inherit())
			.spawn()
			.context("Could not launch the privileged helper")?;

		let stdin = BufWriter::new(child.stdin.take().context("Could not open helper stdin")?);
		let mut stdout = BufReader::new(child.stdout.take().context("Could not open helper stdout")?);

		let mut greeting = String::new();
		let n = stdout.read_line(&mut greeting).context("Could not read the helper greeting")?;
		if n == 0 {
			bail!("The privileged helper exited during startup.");
		}
		let greeting: Greeting = serde_json::from_str(&greeting).context("Could not parse the helper greeting")?;
		if !greeting.ready {
			bail!("The privileged helper is not ready: {}", greeting.error.unwrap_or_default());
		}

		Ok(Self { _child: child, stdin, stdout, dead: false })
	}

	fn request(&mut self, action: PrivilegedAction) -> Result<PrivilegedResponse> {
		if self.dead {
			bail!("The privileged helper is no longer running.");
		}

		let request = serde_json::to_string(&action).context("Could not serialize the request")?;
		if self.stdin.write_all(request.as_bytes()).and_then(|_| self.stdin.write_all(b"\n")).and_then(|_| self.stdin.flush()).is_err() {
			self.dead = true;
			bail!("Could not send the request to the privileged helper.");
		}

		let mut line = String::new();
		let n = self.stdout.read_line(&mut line).context("Could not read the helper response")?;
		if n == 0 {
			self.dead = true;
			bail!("The privileged helper exited unexpectedly.");
		}

		match serde_json::from_str::<ServiceResponse>(&line).context("Could not parse the helper response")? {
			ServiceResponse::Ok(response) => Ok(response),
			ServiceResponse::Err(message) => bail!(message),
		}
	}

	fn child_exited(&mut self) -> bool {
		if self.dead {
			return true;
		}
		if self._child.try_wait().map(|status| status.is_some()).unwrap_or(false) {
			self.dead = true;
			return true;
		}
		false
	}
}

pub fn request(action: PrivilegedAction) -> Result<PrivilegedResponse> {
	static SERVICE: OnceLock<Mutex<Option<PrivilegedService>>> = OnceLock::new();

	let mut slot = SERVICE.get_or_init(|| Mutex::new(None)).lock().unwrap();

	ensure_alive(&mut slot)?;

	let result = {
		let service = slot.as_mut().expect("service was ensured alive");
		service.request(action.clone())
	};

	if !slot.as_ref().is_some_and(|service| service.dead) {
		return result;
	}

	*slot = None;
	ensure_alive(&mut slot)?;
	let service = slot.as_mut().expect("service was respawned");
	service.request(action)
}

fn ensure_alive(slot: &mut Option<PrivilegedService>) -> Result<()> {
	let alive = slot.as_mut().is_some_and(|service| !service.dead && !service.child_exited());
	if !alive {
		*slot = Some(PrivilegedService::spawn()?);
	}
	Ok(())
}

pub fn run_root_helper() -> Result<()> {
	let ready = is_root();
	let greeting = Greeting {
		ready,
		error: (!ready).then(|| "The helper must run as root".to_string()),
	};

	let stdout = std::io::stdout();
	let mut stdout = stdout.lock();
	serde_json::to_writer(&mut stdout, &greeting).context("Could not write the greeting")?;
	stdout.write_all(b"\n")?;
	stdout.flush()?;

	if !ready {
		bail!("The helper must run as root.");
	}

	for line in std::io::stdin().lock().lines() {
		let line = line.context("Could not read a request from stdin")?;
		let action: PrivilegedAction = serde_json::from_str(&line).context("Could not parse the request")?;

		let response = match crate::privileged_actions::execute(action) {
			Ok(response) => ServiceResponse::Ok(response),
			Err(err) => ServiceResponse::Err(format!("{err:#}")),
		};

		serde_json::to_writer(&mut stdout, &response).context("Could not write the response")?;
		stdout.write_all(b"\n")?;
		stdout.flush()?;
	}

	Ok(())
}

fn is_root() -> bool {
	std::fs::read_to_string("/proc/self/status")
		.ok()
		.and_then(|status| status.lines().find(|line| line.starts_with("Uid:")).map(str::to_string))
		.and_then(|line| line.split_whitespace().nth(2).map(str::to_string))
		.is_some_and(|effective_uid| effective_uid == "0")
}