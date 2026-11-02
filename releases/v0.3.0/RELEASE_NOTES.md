# Sager v0.3.0 — Minimal Vertical Tray, Single Exe & Hardened LAN Sync

<p align="center">
  <img src="../../assets/sager-icon.svg" alt="Sager Logo" width="160" height="160">
</p>

Sager v0.3.0 is a cleanup, reliability, and desktop/mobile-parity release. It ships as **one single `.exe`** (`sager.exe`) that installs itself on first run, lives in the Windows system tray / taskbar, and can be shown or hidden from the tray.

---

## Highlights

### 1. Single `.exe` product (`sager.exe`)
- The separate `SagerLauncher.exe` is folded into `sager.exe`.
- On first run, `sager.exe` registers itself in the Windows application list (Desktop + Start Menu shortcuts) and enables silent Windows Startup.
- Runs windowless in the background, minimized to the Windows system tray / taskbar.
- Right-click tray icon: **Show Window**, **Minimize to Tray**, **Backup All**, **Open Dashboard**, **Exit**.
- Toggle the console window from the tray or the dashboard.

### 2. Minimal vertical tray on the website (no shadows)
- New vertical rail on the **left or right** side of the dashboard, set by user preference (default: right) and saved across sessions.
- Compact icon buttons for Backup All, Background/Appear, Settings, Test Overlay, and Move Side — with labels that appear on hover.
- **No shadows anywhere** — flat borders and background tints only; minimal and professional.
- Same style and functions as the localhost dashboard.

### 3. Minimal pure-Rust desktop app (`sager-min`)
- New `sager-min` build target: a pure-Rust, console-free app that starts the same embedded web server and opens the dashboard with the identical style and buttons.

### 4. Hardened Android ↔ Windows LAN sync
- Android no longer hardcodes a specific PC IP; it shows a placeholder and can **discover Sager PCs on the Wi-Fi** (`/api/network-info` scan) and let you pick one.
- Quick **Ping PC** reachability check.
- Graceful offline mode with cached vault.

### 5. Build fixes
- Fixed broken compile in `src/achievements/catalog.rs` and `src/backup/scanner.rs`.
- `cargo build --release` and `cargo test` are green.

---

## Downloads

| Asset | Platform | Description |
|---|---|---|
| `sager.exe` | Windows 10 / 11 (x64) | **Single standalone executable** — self-installs, tray app, zero console window |
| `sager-companion-v0.3.0.apk` | Android 7.0+ (API 24+) | Android companion with offline vault + local Home Wi-Fi sync |

## How to use
1. Run `sager.exe` once — it adds itself to Startup, Desktop, and Start Menu, then hides to the tray.
2. Click the tray icon to open the dashboard (`http://127.0.0.1:3939`).
3. On your phone, open the Sager Android app, tap **Discover PC** (or enter the PC's IP:port shown on the dashboard), then **Sync Now**.
