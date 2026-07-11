//! Thin wrapper over the `docker` CLI.
//!
//! We shell out instead of pulling in a Docker API crate: it is the laziest
//! thing that works on every Docker Desktop / Engine install without extra
//! runtime deps, and the commands we need are a handful of one-liners.

use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::config::{CONTAINER_NAME, IMAGE};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub installed: bool,
    pub running: bool,
    pub version: String,
    pub message: String,
}

/// Run `docker <args>` and return (success, stdout, stderr).
fn docker(args: &[&str]) -> (bool, String, String) {
    let mut cmd = Command::new("docker");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    match cmd.output() {
        Ok(out) => (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).trim().to_string(),
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

/// Same, but for a longer arg vector (owned strings).
fn docker_owned(args: &[String]) -> (bool, String, String) {
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    docker(&refs)
}

pub fn check() -> DockerStatus {
    // `docker --version` works even when the daemon is down.
    let (ok_cli, ver, _) = docker(&["--version"]);
    if !ok_cli {
        return DockerStatus {
            installed: false,
            running: false,
            version: String::new(),
            message: "Docker is not installed. Install Docker Desktop and try again.".into(),
        };
    }
    // Talking to the daemon confirms it is actually running.
    let (ok_daemon, server_ver, err) = docker(&["version", "--format", "{{.Server.Version}}"]);
    if ok_daemon && !server_ver.is_empty() {
        DockerStatus {
            installed: true,
            running: true,
            version: server_ver,
            message: "Docker is ready.".into(),
        }
    } else {
        let hint = if err.to_lowercase().contains("pipe")
            || err.to_lowercase().contains("cannot connect")
            || err.to_lowercase().contains("daemon")
        {
            "Docker is installed but not running. Start Docker Desktop and wait for the whale icon.".to_string()
        } else if !err.is_empty() {
            err
        } else {
            "Docker is installed but the daemon did not respond.".to_string()
        };
        DockerStatus {
            installed: true,
            running: false,
            version: ver,
            message: hint,
        }
    }
}

pub fn image_present() -> bool {
    let (ok, out, _) = docker(&["images", "-q", IMAGE]);
    ok && !out.is_empty()
}

pub fn pull_image() -> Result<(), String> {
    let (ok, _, err) = docker(&["pull", IMAGE]);
    if ok {
        Ok(())
    } else {
        Err(err)
    }
}

pub fn container_exists() -> bool {
    let (ok, out, _) = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("name=^{CONTAINER_NAME}$"),
        "--format",
        "{{.Names}}",
    ]);
    ok && out.lines().any(|l| l == CONTAINER_NAME)
}

pub fn container_running() -> bool {
    let (ok, out, _) = docker(&[
        "inspect",
        "-f",
        "{{.State.Running}}",
        CONTAINER_NAME,
    ]);
    ok && out == "true"
}

/// (Re)create and start the game server container.
pub fn create_and_run(env_args: &[String], data_dir: &Path, port: u16, rcon_port: u16) -> Result<(), String> {
    if container_exists() {
        let _ = docker(&["rm", "-f", CONTAINER_NAME]);
    }
    let data = data_dir.to_string_lossy().to_string();
    let game_port = format!("{port}:{port}/udp");
    // RCON is bound to localhost only — it must never be reachable from outside.
    let rcon_map = format!("127.0.0.1:{rcon_port}:{rcon_port}/tcp");

    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--name".into(),
        CONTAINER_NAME.into(),
        "--restart".into(),
        "unless-stopped".into(),
        "-p".into(),
        game_port,
        "-p".into(),
        rcon_map,
        "-v".into(),
        format!("{data}:/palworld"),
    ];
    args.extend_from_slice(env_args);
    args.push(IMAGE.into());

    let (ok, _, err) = docker_owned(&args);
    if ok {
        Ok(())
    } else {
        Err(err)
    }
}

pub fn start() -> Result<(), String> {
    let (ok, _, err) = docker(&["start", CONTAINER_NAME]);
    if ok { Ok(()) } else { Err(err) }
}

pub fn stop() -> Result<(), String> {
    let (ok, _, err) = docker(&["stop", CONTAINER_NAME]);
    if ok { Ok(()) } else { Err(err) }
}

pub fn restart() -> Result<(), String> {
    let (ok, _, err) = docker(&["restart", CONTAINER_NAME]);
    if ok { Ok(()) } else { Err(err) }
}

pub fn remove() -> Result<(), String> {
    let (ok, _, err) = docker(&["rm", "-f", CONTAINER_NAME]);
    if ok { Ok(()) } else { Err(err) }
}

pub fn logs(lines: u32) -> String {
    let (_, out, err) = docker(&[
        "logs",
        "--tail",
        &lines.to_string(),
        CONTAINER_NAME,
    ]);
    // Docker writes container stdout to our stderr for `logs`; merge both.
    if out.is_empty() {
        err
    } else if err.is_empty() {
        out
    } else {
        format!("{out}\n{err}")
    }
}

/// Container memory usage as reported by `docker stats`, e.g. "3.2GiB / 15GiB".
pub fn mem_usage() -> Option<String> {
    let (ok, out, _) = docker(&[
        "stats",
        "--no-stream",
        "--format",
        "{{.MemUsage}}",
        CONTAINER_NAME,
    ]);
    if ok && !out.is_empty() {
        Some(out)
    } else {
        None
    }
}
