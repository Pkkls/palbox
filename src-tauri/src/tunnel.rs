//! Safe networking via a playit.gg agent running in Docker.
//!
//! The agent makes only an *outbound* connection to the playit relay, so no
//! inbound port is ever opened on the host's router and the host's public IP is
//! never revealed to players — they connect to a relay address instead.
//!
//! ponytail: first run requires a one-time (free) account claim at the URL the
//! agent prints. That step is playit-side and can't be automated away; we scrape
//! it out of the logs and hand it to the UI. Upgrade path if this ever chafes:
//! swap in a self-hosted WireGuard relay (also UDP), same interface.

use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const TUNNEL_NAME: &str = "palbox-tunnel";
const TUNNEL_IMAGE: &str = "ghcr.io/playit-cloud/playit-agent:latest";

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub running: bool,
    /// One-time setup link, present until the agent is claimed.
    pub claim_url: String,
    /// Public relay address to share once the tunnel is live.
    pub address: String,
    pub message: String,
}

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

pub fn start(data_dir: &Path) -> Result<(), String> {
    let _ = docker(&["rm", "-f", TUNNEL_NAME]);
    let cfg = format!("{}/playit:/etc/playit", data_dir.to_string_lossy());
    let (ok, _, err) = docker(&[
        "run",
        "-d",
        "--name",
        TUNNEL_NAME,
        "--restart",
        "unless-stopped",
        // Lets the agent reach the game server published on the host, on Linux
        // hosts too (Docker Desktop maps host.docker.internal automatically).
        "--add-host",
        "host.docker.internal:host-gateway",
        "-v",
        &cfg,
        TUNNEL_IMAGE,
    ]);
    if ok {
        Ok(())
    } else {
        Err(err)
    }
}

pub fn stop() -> Result<(), String> {
    let (ok, _, err) = docker(&["rm", "-f", TUNNEL_NAME]);
    if ok {
        Ok(())
    } else {
        Err(err)
    }
}

fn running() -> bool {
    let (ok, out, _) = docker(&["inspect", "-f", "{{.State.Running}}", TUNNEL_NAME]);
    ok && out == "true"
}

pub fn status() -> TunnelStatus {
    if !running() {
        return TunnelStatus {
            running: false,
            message: "Tunnel is off.".into(),
            ..Default::default()
        };
    }
    let (_, out, err) = docker(&["logs", "--tail", "200", TUNNEL_NAME]);
    let text = format!("{out}\n{err}");

    let claim_url = text
        .split_whitespace()
        .find(|w| w.contains("playit.gg/claim"))
        .map(|w| w.trim_matches(|c: char| !c.is_ascii_graphic()).to_string())
        .unwrap_or_default();

    // Relay addresses look like `something.gl.at.ply.gg` / `*.ply.gg`.
    let address = text
        .split_whitespace()
        .find(|w| w.contains(".ply.gg") || w.contains(".playit.gg:"))
        .map(|w| w.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == ':')).to_string())
        .unwrap_or_default();

    let message = if !claim_url.is_empty() {
        "Tunnel needs a one-time setup — open the link to claim it (free).".into()
    } else if !address.is_empty() {
        "Tunnel is live. Share the relay address with your friends.".into()
    } else {
        "Tunnel is starting…".into()
    };

    TunnelStatus {
        running: true,
        claim_url,
        address,
        message,
    }
}
