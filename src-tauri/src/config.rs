//! Server settings and their mapping to the palworld-server-docker image.
//!
//! We drive the game server entirely through the image's documented environment
//! variables, so PalBox never has to parse or template PalWorldSettings.ini
//! itself. See https://github.com/thijsvanloef/palworld-server-docker

use serde::{Deserialize, Serialize};

pub const CONTAINER_NAME: &str = "palbox-palworld";
pub const IMAGE: &str = "thijsvanloef/palworld-server-docker:latest";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    pub server_name: String,
    pub server_description: String,
    pub server_password: String,
    pub admin_password: String,
    pub players: u32,
    pub difficulty: String,
    pub death_penalty: String,
    pub exp_rate: f32,
    pub pal_capture_rate: f32,
    pub day_time_speed: f32,
    pub night_time_speed: f32,
    pub pvp: bool,
    pub multithreading: bool,
    pub auto_update: bool,
    pub backup_enabled: bool,
    pub port: u16,
    pub rcon_port: u16,
    /// "tunnel" (safe, via relay) or "direct" (port-forward, IP exposed).
    pub tunnel_mode: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            server_name: "My PalBox Server".into(),
            server_description: "Hosted with PalBox".into(),
            server_password: String::new(),
            admin_password: String::new(),
            players: 8,
            difficulty: "None".into(),
            death_penalty: "Item".into(),
            exp_rate: 1.0,
            pal_capture_rate: 1.0,
            day_time_speed: 1.0,
            night_time_speed: 1.0,
            pvp: false,
            multithreading: true,
            auto_update: true,
            backup_enabled: true,
            port: 8211,
            rcon_port: 25575,
            tunnel_mode: "tunnel".into(),
        }
    }
}

impl ServerSettings {
    /// `-e KEY=VALUE` pairs handed to `docker run`.
    ///
    /// COMMUNITY is forced to false so the server never advertises itself on the
    /// public in-game browser — private by default, matching the safe posture.
    pub fn env_args(&self) -> Vec<String> {
        let mut env: Vec<(String, String)> = vec![
            ("PUID".into(), "1000".into()),
            ("PGID".into(), "1000".into()),
            ("PORT".into(), self.port.to_string()),
            ("PLAYERS".into(), self.players.to_string()),
            ("SERVER_NAME".into(), self.server_name.clone()),
            ("SERVER_DESCRIPTION".into(), self.server_description.clone()),
            ("SERVER_PASSWORD".into(), self.server_password.clone()),
            ("ADMIN_PASSWORD".into(), self.admin_password.clone()),
            ("COMMUNITY".into(), "false".into()),
            ("MULTITHREADING".into(), bool_str(self.multithreading)),
            ("RCON_ENABLED".into(), "true".into()),
            ("RCON_PORT".into(), self.rcon_port.to_string()),
            ("UPDATE_ON_BOOT".into(), bool_str(self.auto_update)),
            ("AUTO_UPDATE_ENABLED".into(), bool_str(self.auto_update)),
            ("BACKUP_ENABLED".into(), bool_str(self.backup_enabled)),
            ("DIFFICULTY".into(), self.difficulty.clone()),
            ("DEATH_PENALTY".into(), self.death_penalty.clone()),
            ("EXP_RATE".into(), fnum(self.exp_rate)),
            ("PAL_CAPTURE_RATE".into(), fnum(self.pal_capture_rate)),
            ("DAYTIME_SPEEDRATE".into(), fnum(self.day_time_speed)),
            ("NIGHTTIME_SPEEDRATE".into(), fnum(self.night_time_speed)),
            (
                "ENABLE_PLAYER_TO_PLAYER_DAMAGE".into(),
                bool_str(self.pvp),
            ),
            ("PVP".into(), bool_str(self.pvp)),
        ];
        // Flatten into ["-e", "KEY=VALUE", ...].
        let mut args = Vec::with_capacity(env.len() * 2);
        for (k, v) in env.drain(..) {
            args.push("-e".to_string());
            args.push(format!("{k}={v}"));
        }
        args
    }
}

fn bool_str(b: bool) -> String {
    if b {
        "true".into()
    } else {
        "false".into()
    }
}

/// Palworld expects rates as plain decimals (e.g. "1.000000").
fn fnum(v: f32) -> String {
    format!("{v:.6}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_args_are_key_value_pairs() {
        let s = ServerSettings::default();
        let args = s.env_args();
        assert_eq!(args.len() % 2, 0, "each -e must be followed by a value");
        assert!(args.iter().any(|a| a == "COMMUNITY=false"), "must stay private");
        assert!(args.iter().any(|a| a == "PLAYERS=8"));
        assert!(args.iter().any(|a| a.starts_with("EXP_RATE=1.0")));
    }
}
