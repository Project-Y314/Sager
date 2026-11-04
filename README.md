<p align="center">
  <img src="assets/sager-icon.svg" alt="Sager" width="120" height="120">
</p>
<h1 align="center">Sager</h1>
<p align="center">
  Achievements pop up like in Steam. Your game saves back up to Google Drive by themselves.<br>
  One tiny exe, no window, no login, no telemetry.
</p>
<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Pure%20Rust-orange.svg" alt="Rust"></a>
  <a href="https://www.microsoft.com/windows"><img src="https://img.shields.io/badge/Windows%2010%20%7C%2011-blue.svg" alt="Windows"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License"></a>
</p>

---

## Download & use (1 minute)

1. Grab **[sager.exe](https://github.com/ybreakless/Sager/releases/latest)** (`sager.exe` from the latest release — ~1.5 MB, also available as a zip).
2. Double-click it. It lives silently in your system tray — that's it, you're installed.

Ready-to-go binaries for every release are published on the [Releases tab](https://github.com/ybreakless/Sager/releases).

> Prefer to build it yourself?
> ```powershell
> git clone https://github.com/ybreakless/Sager.git
> cd Sager
> cargo build --release
> # -> target\release\sager.exe
> ```

## What it does

- **Achievement notifications** — Watches Goldberg / Steam emulator save folders and shows a Steam-style in-game toast with the authentic unlock chime the moment you unlock something.
- **Automatic save backups** — Finds save folders for 23+ games (and Documents / My Games / Saved Games / emulator paths), zips up anything that changed, and syncs it into your Google Drive folder.
- **1-click restore** — Moved to a new PC or lost a save? Open the dashboard, click *Restore*, done. A safety copy of your current save is made first, automatically.
- **That's it.** No test suites, no diagnostics dashboards, no vaults — just the two things this app is for.

## Everyday use

| What | How |
|---|---|
| Open your dashboard | Click the tray icon (or right-click → *Open Sager*) |
| Back up everything | *Back up all games* button, or right-click tray → *Back up all games now* |
| Restore a save | Dashboard → *Cloud backups* → *Restore* |
| Turn achievements sound off / change backup frequency | Dashboard → *Settings* |
| Start it with Windows | Dashboard → *Settings* → *Start with Windows* |
| Quit | Right-click tray → *Exit* |

The dashboard lives at `http://127.0.0.1:3939` — one page: backups, restores, achievements, four settings.

## Where things live

| Thing | Location |
|---|---|
| Backup archives | `G:\My Drive\SagerBackups` (auto-detected Google Drive folder, falls back to `%USERPROFILE%\SagerBackups`) |
| Settings & achievement history | `%APPDATA%\Sager\` |
| Dashboard console logs (debug) | `sager.exe --console`, launched from a terminal |

## Android companion (optional)

A minimal offline trophy vault: [`source project`](android/) + APK on the Releases tab. It talks to your PC only on your home Wi-Fi (`/api/mobile/sync`) — no cloud middleman.

---

Sager is free and open source ([MIT](LICENSE)). If it's useful to you, consider [sponsoring](https://github.com/sponsors/ybreakless) — but it costs nothing to use.
