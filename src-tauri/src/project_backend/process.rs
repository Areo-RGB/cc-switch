//! Process execution helpers used by project scripts and CLI-backed commands.
//!
//! Long-running commands are placed in their own process group. This lets the
//! Projects page stop a shell script together with any compiler/dev-server
//! children it started.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn combined(&self) -> String {
        [self.stdout.trim(), self.stderr.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug)]
pub enum StreamEvent {
    Stdout(String),
    Stderr(String),
    Exit(i32),
}

#[derive(Clone)]
pub struct ProcessHandle {
    pid: u32,
    child: Arc<Mutex<Child>>,
}

impl ProcessHandle {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn kill(&self) {
        kill_process_group(self.pid);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

pub struct StreamingChild {
    pub handle: ProcessHandle,
    pub events: Receiver<StreamEvent>,
}

fn path_exts() -> Vec<String> {
    if !cfg!(windows) {
        return Vec::new();
    }
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned())
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(str::to_owned)
        .collect()
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn with_extensions(path: PathBuf) -> Vec<PathBuf> {
    let mut paths = path_exts()
        .into_iter()
        .map(|extension| {
            let mut value = path.clone().into_os_string();
            value.push(extension);
            PathBuf::from(value)
        })
        .collect::<Vec<_>>();
    paths.push(path);
    paths
}

fn enriched_path_value() -> String {
    let mut paths = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let mut extra = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        home.join(".local/bin"),
        home.join(".cargo/bin"),
        home.join(".bun/bin"),
    ];
    if let Ok(entries) = std::fs::read_dir(home.join(".nvm/versions/node")) {
        let mut versions = entries
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        versions.sort();
        if let Some(version) = versions.last() {
            extra.push(version.join("bin"));
        }
    }
    for path in extra {
        if path.is_dir() && !paths.contains(&path) {
            paths.push(path);
        }
    }
    std::env::join_paths(paths)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn enriched_path() -> String {
    enriched_path_value()
}

pub fn which(executable: &str) -> Option<PathBuf> {
    if executable.contains(['/', '\\']) {
        return with_extensions(PathBuf::from(executable))
            .into_iter()
            .find(|path| is_executable(path));
    }
    std::env::split_paths(&enriched_path())
        .flat_map(|directory| with_extensions(directory.join(executable)))
        .find(|path| is_executable(path))
}

fn configure_command(command: &mut Command, own_group: bool) {
    command.env("PATH", enriched_path());
    #[cfg(unix)]
    if own_group {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let flags = if own_group {
            CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
        } else {
            CREATE_NO_WINDOW
        };
        command.creation_flags(flags);
    }
}

pub fn run_capture(
    executable: &str,
    args: &[String],
    cwd: Option<&Path>,
) -> Result<CommandOutput, String> {
    let resolved = which(executable).unwrap_or_else(|| PathBuf::from(executable));
    let mut command = Command::new(resolved);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    configure_command(&mut command, false);
    let output = command
        .output()
        .map_err(|error| format!("Could not start {executable}: {error}"))?;
    let code = output.status.code().unwrap_or(-1);
    Ok(CommandOutput {
        code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn shell_command(script: &str, cwd: &Path) -> Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new("/bin/sh");
        command
            .args(["-lc", script])
            .current_dir(cwd)
            .process_group(0);
        configure_command(&mut command, false);
        command
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/S", "/C", script]).current_dir(cwd);
        configure_command(&mut command, true);
        command
    }
}

pub fn run_shell_capture(script: &str, cwd: &Path) -> Result<CommandOutput, String> {
    let mut command = shell_command(script, cwd);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command
        .output()
        .map_err(|error| format!("Could not start project script: {error}"))?;
    Ok(CommandOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

pub fn spawn_detached(executable: &str, args: &[String], cwd: &Path) -> Result<(), String> {
    let resolved = which(executable).unwrap_or_else(|| PathBuf::from(executable));
    let mut command = Command::new(resolved);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_command(&mut command, false);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not launch {executable}: {error}"))
}

pub fn start_shell(script: &str, cwd: &Path) -> Result<StreamingChild, String> {
    let mut command = shell_command(script, cwd);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start project script: {error}"))?;
    let pid = child.id();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Project script stdout was not captured.".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Project script stderr was not captured.".to_owned())?;
    let (sender, receiver) = mpsc::channel();
    spawn_reader(stdout, sender.clone(), true);
    spawn_reader(stderr, sender.clone(), false);
    let child = Arc::new(Mutex::new(child));
    let waiter = Arc::clone(&child);
    std::thread::spawn(move || loop {
        let result = waiter
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok());
        if let Some(Some(status)) = result {
            let _ = sender.send(StreamEvent::Exit(status.code().unwrap_or(-1)));
            break;
        }
        std::thread::sleep(Duration::from_millis(40));
    });
    Ok(StreamingChild {
        handle: ProcessHandle { pid, child },
        events: receiver,
    })
}

fn spawn_reader<R: Read + Send + 'static>(reader: R, sender: Sender<StreamEvent>, stdout: bool) {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            let event = if stdout {
                StreamEvent::Stdout(line)
            } else {
                StreamEvent::Stderr(line)
            };
            if sender.send(event).is_err() {
                break;
            }
        }
    });
}

pub fn kill_process_group(pid: u32) {
    #[cfg(unix)]
    {
        let mut command = Command::new("kill");
        command.args(["-TERM", "--", &format!("-{pid}")]);
        let _ = command.status();
    }
    #[cfg(windows)]
    {
        let mut command = Command::new("taskkill");
        command.args(["/T", "/F", "/PID", &pid.to_string()]);
        configure_command(&mut command, false);
        let _ = command.status();
    }
}

#[allow(dead_code)]
pub fn wait_for_exit(handle: &ProcessHandle, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if handle
            .child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok())
            .flatten()
            .is_some()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_executable_paths() {
        assert!(which(if cfg!(windows) { "cmd" } else { "sh" }).is_some());
    }

    #[test]
    fn streams_output_and_can_be_terminated() {
        let directory = tempfile::tempdir().unwrap();
        let script = if cfg!(windows) {
            "echo start & ping 127.0.0.1 -n 10 > nul"
        } else {
            "printf 'start\\n'; sleep 30"
        };
        let stream = start_shell(script, directory.path()).unwrap();
        let handle = stream.handle.clone();
        let mut saw_output = false;
        for event in stream.events.iter().take(5) {
            if matches!(event, StreamEvent::Stdout(line) if line == "start") {
                saw_output = true;
                break;
            }
        }
        assert!(saw_output);
        handle.kill();
        assert!(wait_for_exit(&handle, Duration::from_secs(2)));
    }
}
