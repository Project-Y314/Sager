use crate::achievements::{load_achievements_history};
use crate::backup::{SaveArchiver, SaveRestore, SaveScanner};
use crate::config::{self, get_drive_status, save_config, SagerConfig};
use crate::web::ui::get_dashboard_html;
use crate::set_windows_startup;
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::{Header, Method, Response, Server, StatusCode};

pub struct WebServer;

impl WebServer {
    pub fn start(config_provider: Arc<Mutex<SagerConfig>>) {
        thread::spawn(move || {
            let port = config_provider.lock().unwrap().web_port;

            let bind_addr = format!("127.0.0.1:{}", port);
            let server = match Server::http(&bind_addr) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to start Sager web server on {}: {}", bind_addr, e);
                    return;
                }
            };

            println!("Sager dashboard running at http://127.0.0.1:{}", port);

            for mut req in server.incoming_requests() {
                let url = req.url().to_string();
                let method = req.method().clone();

                let json_header =
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                let html_header = Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"text/html; charset=utf-8"[..],
                )
                .unwrap();

                if url == "/" && method == Method::Get {
                    let resp = Response::from_string(get_dashboard_html()).with_header(html_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/logo" && method == Method::Get {
                    let svg_header =
                        Header::from_bytes(&b"Content-Type"[..], &b"image/svg+xml"[..]).unwrap();
                    let resp =
                        Response::from_data(include_bytes!("../../assets/sager-icon.svg").to_vec())
                            .with_header(svg_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/status" && method == Method::Get {
                    let cfg = config_provider.lock().unwrap().clone();
                    let games = SaveScanner::scan_all(&cfg);
                    let achievements = load_achievements_history();
                    let drive = get_drive_status(&cfg);
                    // Can we actually write there right now?
                    let backup_dir_accessible = cfg.backup_directory.exists()
                        || std::fs::create_dir_all(&cfg.backup_directory).is_ok();

                    let status = json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "backup_dir": cfg.backup_directory.to_string_lossy(),
                        "games_count": games.len(),
                        "achievements_count": achievements.len(),
                        "auto_backup_enabled": cfg.auto_backup_enabled,
                        "auto_backup_interval_minutes": cfg.auto_backup_interval_minutes,
                        "sound_enabled": cfg.sound_enabled,
                        "notifications_enabled": cfg.notifications_enabled,
                        "backup_dir_accessible": backup_dir_accessible,
                        "google_drive_running": drive.google_drive_running,
                        "google_drive_installed": drive.google_drive_installed,
                        "startup_enabled": is_startup_installed(),
                    });
                    let resp = Response::from_string(status.to_string()).with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/games" && method == Method::Get {
                    let cfg = config_provider.lock().unwrap().clone();
                    let resp = Response::from_string(
                        serde_json::to_string(&list_games_with_backup_info(&cfg)).unwrap_or_default(),
                    )
                    .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/backups" && method == Method::Get {
                    let cfg = config_provider.lock().unwrap().clone();
                    let backups = SaveRestore::list_all_backups(&cfg);
                    let resp = Response::from_string(
                        serde_json::to_string(&backups).unwrap_or_default(),
                    )
                    .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/backup" && method == Method::Post {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    let cfg = config_provider.lock().unwrap().clone();
                    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();

                    let message = if parsed
                        .get("all")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        let res = SaveArchiver::backup_all(&cfg);
                        let success = res.iter().filter(|r| r.is_ok()).count();
                        format!("Backed up {} of {} games to {}", success, res.len(), cfg.backup_directory.display())
                    } else {
                        let name = parsed.get("game_name").and_then(|v| v.as_str()).unwrap_or("Game");
                        let path = parsed.get("source_path").and_then(|v| v.as_str()).unwrap_or("");
                        match SaveArchiver::backup_game(name, &PathBuf::from(path), &cfg) {
                            Ok(info) => format!("Backed up {} ({})", name, info.file_name),
                            Err(e) => format!("Backup failed for {}: {}", name, e),
                        }
                    };

                    let resp = Response::from_string(json!({ "message": message }).to_string())
                        .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/restore" && method == Method::Post {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    let cfg = config_provider.lock().unwrap().clone();
                    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                    let zip_path = parsed
                        .get("zip_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    let message = match SaveRestore::restore_backup(&PathBuf::from(zip_path), None, &cfg) {
                        Ok(res) => format!(
                            "Restored {} files for {} to {}",
                            res.restored_files,
                            res.game_name,
                            res.target_path.display()
                        ),
                        Err(e) => format!("Restore failed: {}", e),
                    };

                    let resp = Response::from_string(json!({ "message": message }).to_string())
                        .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/achievements" && method == Method::Get {
                    let list = load_achievements_history();
                    let resp = Response::from_string(
                        serde_json::to_string(&list).unwrap_or_default(),
                    )
                    .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/config" && method == Method::Post {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);

                    let mut cfg = config_provider.lock().unwrap();
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(dir) = val.get("backup_directory").and_then(|v| v.as_str()) {
                            if !dir.trim().is_empty() {
                                cfg.backup_directory = PathBuf::from(dir.trim());
                            }
                        }
                        if let Some(interval) =
                            val.get("auto_backup_interval_minutes").and_then(|v| v.as_u64())
                        {
                            cfg.auto_backup_interval_minutes = interval.clamp(5, 1440);
                        }
                        if let Some(auto) =
                            val.get("auto_backup_enabled").and_then(|v| v.as_bool())
                        {
                            cfg.auto_backup_enabled = auto;
                        }
                        if let Some(sound) = val.get("sound_enabled").and_then(|v| v.as_bool()) {
                            cfg.sound_enabled = sound;
                        }
                        if let Some(notif) =
                            val.get("notifications_enabled").and_then(|v| v.as_bool())
                        {
                            cfg.notifications_enabled = notif;
                        }
                        let _ = save_config(&cfg);
                    }

                    let resp = Response::from_string(json!({ "success": true }).to_string())
                        .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/startup" && method == Method::Post {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
                    let enable = parsed.get("enabled").and_then(|v| v.as_bool());
                    let (enabled, message) = toggle_startup(enable);
                    let resp = Response::from_string(
                        json!({ "enabled": enabled, "message": message }).to_string(),
                    )
                    .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/open-folder" && method == Method::Post {
                    let cfg = config_provider.lock().unwrap().clone();
                    let _ = open::that(&cfg.backup_directory);
                    let resp = Response::from_string(json!({ "success": true }).to_string())
                        .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/launch-drive" && method == Method::Post {
                    let cfg = config_provider.lock().unwrap().clone();
                    let message = match config::launch_google_drive(&cfg) {
                        Ok(()) => "Google Drive is starting up...".to_string(),
                        Err(e) => e,
                    };
                    let resp = Response::from_string(json!({ "message": message }).to_string())
                        .with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                if url == "/api/mobile/sync" && method == Method::Get {
                    // Kept for the Android companion app (local home Wi-Fi sync).
                    let cfg = config_provider.lock().unwrap().clone();
                    let achievements = load_achievements_history();
                    let games = SaveScanner::scan_all(&cfg);

                    let resp_body = json!({
                        "sager_version": env!("CARGO_PKG_VERSION"),
                        "pc_name": std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Sager-PC".to_string()),
                        "sync_timestamp": chrono::Utc::now().timestamp(),
                        "total_games": games.len(),
                        "unlocked_achievements_count": achievements.len(),
                        "preferred_launcher": "Steam",
                        "unlocked_achievements": achievements,
                    });
                    let resp = Response::from_string(resp_body.to_string()).with_header(json_header);
                    let _ = req.respond(resp);
                    continue;
                }

                let resp =
                    Response::from_string("Not Found").with_status_code(StatusCode(404));
                let _ = req.respond(resp);
            }
        });
    }
}

#[derive(serde::Serialize)]
struct GameView {
    name: String,
    source_path: String,
    save_category: String,
    total_size_bytes: u64,
    files_count: usize,
    last_modified: Option<i64>,
    last_backup: Option<String>,
    backups_count: usize,
}

fn list_games_with_backup_info(config: &SagerConfig) -> Vec<GameView> {
    SaveScanner::scan_all(config)
        .into_iter()
        .map(|g| {
            let backups = SaveRestore::list_backups_for_game(&g.name, config);
            let last_backup = backups
                .first()
                .and_then(|b| {
                    b.manifest
                        .as_ref()
                        .map(|m| m.backup_date_str.clone())
                });
            GameView {
                name: g.name,
                source_path: g.source_path.to_string_lossy().to_string(),
                save_category: g.save_category,
                total_size_bytes: g.total_size_bytes,
                files_count: g.files_count,
                last_modified: g.last_modified,
                last_backup,
                backups_count: backups.len(),
            }
        })
        .collect()
}

fn is_startup_installed() -> bool {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
            .join(r"Microsoft\Windows\Start Menu\Programs\Startup\Sager.lnk")
            .exists()
    } else {
        false
    }
}

/// Toggle (or force-set) the "Start with Windows" shortcut.
fn toggle_startup(enable: Option<bool>) -> (bool, String) {
    let installed = is_startup_installed();
    let target = match enable {
        Some(v) => v,
        None => !installed,
    };

    match set_windows_startup(target) {
        Ok((state, message)) => (state, message),
        Err(message) => (installed, message),
    }
}
