#![windows_subsystem = "windows"]

//! Sager Minimal Dashboard App.
//!
//! Pure Rust, no console window, no tray icon. Starts the same embedded web
//! server that powers the localhost dashboard (all `/api/` endpoints) and opens
//! the dashboard in the default browser. Delivers the exact same style and
//! buttons as `http://127.0.0.1:3939`, served by a single lightweight binary.

use sager::achievements::AchievementNotification;
use sager::config::load_config;
use sager::web::WebServer;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let config = load_config();
    let shared_config = Arc::new(Mutex::new(config));

    let port = {
        let cfg = shared_config.lock().unwrap();
        cfg.web_port
    };

    // The dashboard's /api/ endpoints post achievement notifications through
    // this channel (wired into the tray/overlay backend in `sager.exe`; here
    // it is accepted so the same WebServer code path works unchanged).
    let (tx, _rx) = std::sync::mpsc::channel::<AchievementNotification>();

    // Start embedded web server — identical backend to the localhost dashboard.
    WebServer::start(Arc::clone(&shared_config), tx);

    let url = format!("http://127.0.0.1:{}", port);
    println!(">> Sager Minimal Dashboard on {}", url);
    let _ = open::that(&url);

    // Keep the process alive serving the dashboard until the user closes it.
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
