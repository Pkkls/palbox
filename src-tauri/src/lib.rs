mod config;
mod docker;
mod rcon;
mod tunnel;

use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use tauri::async_runtime::spawn_blocking;
use tauri::{Manager, State};

use config::ServerSettings;

/// Resolved on startup; every command reads paths from here.
struct AppPaths {
    /// Bind-mounted into the container as /palworld (world saves, config).
    server_dir: PathBuf,
    settings_file: PathBuf,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ServerRuntime {
    exists: bool,
    running: bool,
    /// Container is up AND the game answers RCON (actually joinable).
    ready: bool,
    image_ready: bool,
    memory: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Player {
    name: String,
    playeruid: String,
    steamid: String,
}

fn read_settings(path: &std::path::Path) -> ServerSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_settings(path: &std::path::Path, settings: &ServerSettings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

// --- prerequisites & settings ---------------------------------------------

#[tauri::command]
fn check_docker() -> docker::DockerStatus {
    docker::check()
}

#[tauri::command]
fn load_settings(paths: State<AppPaths>) -> ServerSettings {
    read_settings(&paths.settings_file)
}

#[tauri::command]
fn save_settings(paths: State<AppPaths>, settings: ServerSettings) -> Result<(), String> {
    write_settings(&paths.settings_file, &settings)
}

// --- image & container lifecycle ------------------------------------------

#[tauri::command]
async fn pull_image() -> Result<(), String> {
    spawn_blocking(docker::pull_image)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn create_server(paths: State<'_, AppPaths>, settings: ServerSettings) -> Result<(), String> {
    write_settings(&paths.settings_file, &settings)?;
    fs::create_dir_all(&paths.server_dir).map_err(|e| e.to_string())?;
    let env = settings.env_args();
    let dir = paths.server_dir.clone();
    let port = settings.port;
    let rcon_port = settings.rcon_port;
    spawn_blocking(move || docker::create_and_run(&env, &dir, port, rcon_port))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn start_server() -> Result<(), String> {
    spawn_blocking(docker::start).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn stop_server() -> Result<(), String> {
    spawn_blocking(docker::stop).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn restart_server() -> Result<(), String> {
    spawn_blocking(docker::restart).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_server() -> Result<(), String> {
    spawn_blocking(docker::remove).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn server_status(paths: State<'_, AppPaths>) -> Result<ServerRuntime, String> {
    let (port, pw) = rcon_conn(&paths);
    let rt = spawn_blocking(move || {
        let running = docker::container_running();
        ServerRuntime {
            exists: docker::container_exists(),
            running,
            ready: running && rcon::ready("127.0.0.1", port, &pw),
            image_ready: docker::image_present(),
            memory: docker::mem_usage().unwrap_or_default(),
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(rt)
}

#[tauri::command]
async fn get_logs(lines: u32) -> String {
    spawn_blocking(move || docker::logs(lines))
        .await
        .unwrap_or_default()
}

// --- in-game administration via RCON --------------------------------------

fn rcon_conn(paths: &State<AppPaths>) -> (u16, String) {
    let s = read_settings(&paths.settings_file);
    (s.rcon_port, s.admin_password)
}

#[tauri::command]
async fn list_players(paths: State<'_, AppPaths>) -> Result<Vec<Player>, String> {
    let (port, pw) = rcon_conn(&paths);
    let raw = spawn_blocking(move || rcon::command("127.0.0.1", port, &pw, "ShowPlayers"))
        .await
        .map_err(|e| e.to_string())??;
    let mut players = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("name,") {
            continue; // skip blanks and the CSV header
        }
        let cols: Vec<&str> = line.splitn(3, ',').collect();
        if cols.len() == 3 {
            players.push(Player {
                name: cols[0].to_string(),
                playeruid: cols[1].to_string(),
                steamid: cols[2].to_string(),
            });
        }
    }
    Ok(players)
}

async fn rcon_run(paths: State<'_, AppPaths>, cmd: String) -> Result<String, String> {
    let (port, pw) = rcon_conn(&paths);
    spawn_blocking(move || rcon::command("127.0.0.1", port, &pw, &cmd))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn kick_player(paths: State<'_, AppPaths>, steamid: String) -> Result<String, String> {
    rcon_run(paths, format!("KickPlayer {steamid}")).await
}

#[tauri::command]
async fn ban_player(paths: State<'_, AppPaths>, steamid: String) -> Result<String, String> {
    rcon_run(paths, format!("BanPlayer {steamid}")).await
}

#[tauri::command]
async fn broadcast(paths: State<'_, AppPaths>, message: String) -> Result<String, String> {
    // Palworld's Broadcast splits on spaces; underscores render as spaces.
    let safe = message.replace(' ', "_");
    rcon_run(paths, format!("Broadcast {safe}")).await
}

#[tauri::command]
async fn save_world(paths: State<'_, AppPaths>) -> Result<String, String> {
    rcon_run(paths, "Save".to_string()).await
}

// --- safe networking (tunnel) ---------------------------------------------

#[tauri::command]
async fn tunnel_start(paths: State<'_, AppPaths>) -> Result<(), String> {
    let secret = read_settings(&paths.settings_file).playit_secret;
    spawn_blocking(move || tunnel::start(&secret))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn tunnel_stop() -> Result<(), String> {
    spawn_blocking(tunnel::stop).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn tunnel_status() -> tunnel::TunnelStatus {
    spawn_blocking(tunnel::status).await.unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir");
            fs::create_dir_all(&data_dir).ok();
            app.manage(AppPaths {
                server_dir: data_dir.join("server"),
                settings_file: data_dir.join("settings.json"),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_docker,
            load_settings,
            save_settings,
            pull_image,
            create_server,
            start_server,
            stop_server,
            restart_server,
            delete_server,
            server_status,
            get_logs,
            list_players,
            kick_player,
            ban_player,
            broadcast,
            save_world,
            tunnel_start,
            tunnel_stop,
            tunnel_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
