//! Safe networking via a playit.gg agent running in Docker.
//!
//! The agent makes only an *outbound* connection to the playit relay, so no
//! inbound port is ever opened on the host's router and the host's public IP is
//! never revealed to players — they connect to a relay address instead.
//!
//! The official playit agent image needs a `SECRET_KEY` from a (free) playit
//! account; there is no zero-account auto-claim. So PalBox runs the agent with
//! the user's key and points a UDP tunnel at the game server reached through
//! `host.docker.internal:<port>`. Getting the key + creating the tunnel is a
//! one-time step on playit.gg — friends still need nothing.
//!
//! ponytail: naive log scraping for the relay address; upgrade path is the
//! playit control API if we ever need the address without parsing logs.

use serde::Serialize;
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
    /// True once a secret key has been provided and the agent is up.
    pub configured: bool,
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

/// Start the agent with the user's playit secret key.
pub fn start(secret: &str) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Err(
            "No playit key yet. Open the Safe access panel to get a free key and paste it in."
                .into(),
        );
    }
    let _ = docker(&["rm", "-f", TUNNEL_NAME]);
    let secret_env = format!("SECRET_KEY={}", secret.trim());
    let (ok, _, err) = docker(&[
        "run",
        "-d",
        "--name",
        TUNNEL_NAME,
        "--restart",
        "unless-stopped",
        // Lets the agent reach the game server published on the host (Docker
        // Desktop maps host.docker.internal automatically; this covers Linux).
        "--add-host",
        "host.docker.internal:host-gateway",
        "-e",
        &secret_env,
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
            message: "Tunnel is off.".into(),
            ..Default::default()
        };
    }
    let (_, out, err) = docker(&["logs", "--tail", "200", TUNNEL_NAME]);
    let text = format!("{out}\n{err}");
    let low = text.to_lowercase();

    // Relay addresses look like `something.gl.at.ply.gg` / `*.ply.gg`.
    let address = text
        .split_whitespace()
        .find(|w| w.contains(".ply.gg") || w.contains(".playit.gg:"))
        .map(|w| {
            w.trim_matches(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '.' || c == ':')
            })
            .to_string()
        })
        .unwrap_or_default();

    let (configured, message) = if low.contains("secret key is required") {
        (false, "No key set — add your playit key in the Safe access panel.".to_string())
    } else if low.contains("invalid") && low.contains("secret") {
        (false, "That playit key was rejected — double-check it.".to_string())
    } else if !address.is_empty() {
        (true, "Tunnel is live. Share the relay address with your friends.".to_string())
    } else {
        (true, "Tunnel is connecting…".to_string())
    };

    TunnelStatus {
        running: true,
        configured,
        address,
        message,
    }
}
