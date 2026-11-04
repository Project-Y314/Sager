# Sager v0.4.0 — The Minimal Release

One pure-Rust executable. One dashboard page. Everything you actually use, nothing you don't.

## What changed

### Removed
- `sager-min` companion binary and the separate `SagerLauncher.exe` installer (download `sager.exe` directly instead).
- Test center, diagnostics suite, Google Drive write benchmark, custom achievement simulator.
- Live event console, "run in background / make appear" console juggling (the app is windowless by design).
- Vault export/import, mobile test payload buttons, network info page.
- Launcher preference pills (Goldberg/Steam/Project Lightning/Custom), tray-side and compact-mode settings.
- Forced auto-registration at startup — Sager no longer silently adds itself to Windows Startup, and no longer litters your Desktop and Start Menu with shortcuts on every run. "Start with Windows" is now a simple, honest toggle in the dashboard.

### Fixed / improved
- **Backup retention**: Sager now keeps only the newest 10 snapshots per game, so Google Drive never fills up with hundreds of zips.
- **Backup folder health**: the dashboard cloud chip now reflects whether the backup folder is actually writable — failures surface instantly instead of hiding.
- **Honest versioning**: manifests carry the real app version; the domain `0.0.0.0` binding is gone (localhost only, as intended).
- Dashboard rewritten from ~2,000 lines to a single focused page (~21 KB served, was ~102 KB): status chips, one hero backup button, saves list with filter, cloud restores, achievements, four settings.
- Tray menu reduced to what it should be: status, "Open Sager", "Back up all games now", "Exit".
- End-to-end Rust tests for the backup → manifest → restore → safety-copy pipeline and pruning.

## Upgrade notes
- Your config (`%APPDATA%\Sager\sager_config.json`) migrates automatically; removed settings are simply ignored.
- The Android companion keeps working over home Wi-Fi sync.
- Sager v0.4.0 only speaks to `http://127.0.0.1:3939`.
