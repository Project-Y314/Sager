// Sager — Windows tray app + embedded dashboard, one pure-Rust binary.

pub mod achievements;
pub mod audio;
pub mod backup;
pub mod config;
pub mod overlay;
pub mod tray;
pub mod web;

use achievements::{AchievementNotification, AchievementWatcher};
use backup::SaveArchiver;
use config::load_config;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray::{start_notification_listener, SystemTray};
use web::WebServer;

#[link(name = "kernel32")]
extern "system" {
    fn AttachConsole(dwProcessId: u32) -> i32;
}
const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;

/// Entry point: runs silently in the Windows system tray with the dashboard on localhost.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    // Show logs when launched from a terminal with --console / --debug
    if args.iter().any(|a| a == "--console" || a == "--debug") {
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    let config = load_config();
    // `--ui` opens the dashboard immediately (handy when launching manually)
    let open_ui = args.iter().any(|a| a == "--ui" || a == "--dashboard");
    let port = config.web_port;

    let shared_config = Arc::new(Mutex::new(config));
    let (tx, rx) = channel::<AchievementNotification>();

    // 1. Achievement watcher → tray notifications
    let watcher = AchievementWatcher::new(tx.clone());
    watcher.start(Arc::clone(&shared_config));
    println!("Achievement watcher running.");

    // 2. Dashboard on localhost
    WebServer::start(Arc::clone(&shared_config));
    println!("Dashboard: http://127.0.0.1:{}", port);
    if open_ui {
        let dashboard = format!("http://127.0.0.1:{}", port);
        let _ = open::that(&dashboard);
    }

    // 3. Background auto-backup loop
    let backup_config = Arc::clone(&shared_config);
    thread::spawn(move || loop {
        let (enabled, interval_mins) = {
            let cfg = backup_config.lock().unwrap();
            (cfg.auto_backup_enabled, cfg.auto_backup_interval_minutes)
        };

        if enabled {
            thread::sleep(Duration::from_secs(interval_mins.max(5) * 60));
            let cfg = backup_config.lock().unwrap().clone();
            let results = SaveArchiver::backup_all(&cfg);
            let successes = results.iter().filter(|r| r.is_ok()).count();
            println!(
                "Auto-backup: {} of {} games saved to {}",
                successes,
                results.len(),
                cfg.backup_directory.display()
            );
        } else {
            thread::sleep(Duration::from_secs(60));
        }
    });

    // 4. System tray icon (message loop runs until the user exits)
    println!("Adding tray icon…");
    match SystemTray::new(port) {
        Ok(tray) => {
            let handle = tray.handle();
            start_notification_listener(handle, rx, Arc::clone(&shared_config));

            handle.show_balloon_notification(
                "Sager is watching",
                "Achievement alerts and save backups are on. Click the icon to open your dashboard.",
                false,
            );

            println!("Tray icon added. Running…");
            tray.run_message_loop();
            println!("Sager closed.");
        }
        Err(e) => {
            eprintln!("Tray unavailable: {}. Dashboard stays open on http://127.0.0.1:{}", e, port);
            // Keep the backend alive so the dashboard still works
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        }
    }
}

const STARTUP_LNK: &str = r"Microsoft\Windows\Start Menu\Programs\Startup\Sager.lnk";

/// Create or remove the "Start with Windows" shortcut. Returns (installed, message).
pub fn set_windows_startup(enable: bool) -> Result<(bool, String), String> {
    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not available".to_string())?;
    let lnk = std::path::PathBuf::from(&appdata).join(STARTUP_LNK);

    let exe_path = std::env::current_exe().map_err(|_| "cannot locate sager.exe".to_string())?;
    let exe_name = exe_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !exe_name.starts_with("sager") {
        return Err(format!("{} is not Sager itself; start with Windows not configured", exe_path.display()));
    }

    let installed = lnk.exists();
    if enable && installed {
        return Ok((true, "Sager already starts with Windows".to_string()));
    }
    if !enable && !installed {
        return Ok((false, "Sager does not start with Windows".to_string()));
    }

    if enable {
        create_shortcut(&exe_path, &lnk)?;
        Ok((true, "Sager will start with Windows (silent, in the tray)".to_string()))
    } else {
        std::fs::remove_file(&lnk)
            .map_err(|e| format!("Could not remove startup shortcut: {}", e))?;
        Ok((false, "Sager will not start with Windows anymore".to_string()))
    }
}

fn create_shortcut(target: &std::path::Path, lnk: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = lnk.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not open the Startup folder: {}", e))?;
    }
    let cur_dir = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let ps_cmd = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WorkingDirectory = '{}'; $s.Save()",
        lnk.to_string_lossy().replace('\'', "''"),
        target.to_string_lossy().replace('\'', "''"),
        cur_dir.to_string_lossy().replace('\'', "''"),
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
        .output()
        .map_err(|e| format!("Could not create the startup shortcut: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Could not create the startup shortcut: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}
