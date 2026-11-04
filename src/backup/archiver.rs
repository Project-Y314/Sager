use crate::config::SagerConfig;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub game_name: String,
    pub original_path: String,
    pub backup_timestamp: i64,
    pub backup_date_str: String,
    pub total_files: usize,
    pub total_bytes: u64,
    pub sager_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub file_name: String,
    pub full_path: PathBuf,
    pub size_bytes: u64,
    pub manifest: Option<BackupManifest>,
}

pub struct SaveArchiver;

impl SaveArchiver {
    pub fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }

    pub fn backup_game(
        game_name: &str,
        source_path: &Path,
        config: &SagerConfig,
    ) -> Result<BackupInfo, String> {
        if !source_path.exists() {
            return Err(format!("Source path does not exist: {}", source_path.display()));
        }

        let clean_name = Self::sanitize_filename(game_name);
        let game_backup_dir = config.backup_directory.join(&clean_name);
        fs::create_dir_all(&game_backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        // Optimization: Skip re-compressing if save has not been modified
        let latest_path = game_backup_dir.join("latest.json");
        if latest_path.exists() {
            if let Ok(content) = fs::read_to_string(&latest_path) {
                if let Ok(prev_manifest) = serde_json::from_str::<BackupManifest>(&content) {
                    let newest_source_mtime = Self::get_newest_mtime(source_path);
                    if newest_source_mtime > 0 && newest_source_mtime <= prev_manifest.backup_timestamp {
                        let size = prev_manifest.total_bytes;
                        return Ok(BackupInfo {
                            file_name: format!("(unchanged since {})", prev_manifest.backup_date_str),
                            full_path: game_backup_dir,
                            size_bytes: size,
                            manifest: Some(prev_manifest),
                        });
                    }
                }
            }
        }

        let now = chrono::Local::now();
        let timestamp_str = now.format("%Y%m%d_%H%M%S").to_string();
        let zip_filename = format!("{}_{}.zip", clean_name, timestamp_str);
        let zip_path = game_backup_dir.join(&zip_filename);

        let zip_file = File::create(&zip_path)
            .map_err(|e| format!("Failed to create zip file: {}", e))?;
        let mut zip = ZipWriter::new(zip_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut total_files = 0;
        let mut total_bytes = 0;

        for entry in WalkDir::new(source_path).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(rel_path) = path.strip_prefix(source_path) {
                    let rel_str = rel_path.to_string_lossy().replace('\\', "/");
                    
                    zip.start_file(&rel_str, options)
                        .map_err(|e| format!("Failed to add file to zip: {}", e))?;
                    
                    let mut file = File::open(path)
                        .map_err(|e| format!("Failed to open file: {}", e))?;
                    let bytes = std::io::copy(&mut file, &mut zip)
                        .map_err(|e| format!("Failed to write file to zip: {}", e))?;
                    
                    total_bytes += bytes;
                    total_files += 1;
                }
            }
        }

        // Add manifest inside the zip
        let manifest = BackupManifest {
            game_name: game_name.to_string(),
            original_path: source_path.to_string_lossy().to_string(),
            backup_timestamp: now.timestamp(),
            backup_date_str: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            total_files,
            total_bytes,
            sager_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

        zip.start_file("sager_manifest.json", options)
            .map_err(|e| format!("Failed to add manifest to zip: {}", e))?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(|e| format!("Failed to write manifest: {}", e))?;

        zip.finish()
            .map_err(|e| format!("Failed to finalize zip: {}", e))?;

        // Save manifest alongside zip as .json for quick inspection
        let json_path = game_backup_dir.join(format!("{}_{}.json", clean_name, timestamp_str));
        let _ = fs::write(&json_path, &manifest_json);

        // Update latest.json
        let latest_path = game_backup_dir.join("latest.json");
        let _ = fs::write(&latest_path, &manifest_json);

        // Keep only the newest archives per game so the cloud folder stays tidy
        Self::prune_old_backups(&game_backup_dir, 10);

        let size_bytes = fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(total_bytes);

        Ok(BackupInfo {
            file_name: zip_filename,
            full_path: zip_path,
            size_bytes,
            manifest: Some(manifest),
        })
    }

    pub fn backup_all(config: &SagerConfig) -> Vec<Result<BackupInfo, String>> {
        use super::scanner::SaveScanner;
        let games = SaveScanner::scan_all(config);
        let mut results = Vec::new();

        for game in games {
            let res = Self::backup_game(&game.name, &game.source_path, config);
            results.push(res);
        }

        results
    }

    fn get_newest_mtime(dir: &Path) -> i64 {
        let mut max_mtime = 0;
        for entry in WalkDir::new(dir).into_iter().flatten() {
            if entry.path().is_file() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
                            let epoch = duration.as_secs() as i64;
                            if epoch > max_mtime {
                                max_mtime = epoch;
                            }
                        }
                    }
                }
            }
        }
        max_mtime
    }

    /// Keep only the newest `keep` timestamped zip archives per game.
    /// Older zip + manifest json pairs are removed so Google Drive never
    /// fills up with hundreds of snapshots. Safety backups (separate
    /// `*_safety_before_restore` folders) are never touched.
    fn prune_old_backups(game_backup_dir: &Path, keep: usize) {
        let Ok(entries) = fs::read_dir(game_backup_dir) else {
            return;
        };

        let mut archives: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "zip") {
                archives.push(path);
            }
        }

        // Newest first (timestamped filenames sort lexicographically)
        archives.sort();
        archives.reverse();

        for old_zip in archives.into_iter().skip(keep) {
            let _ = fs::remove_file(&old_zip);
            let old_json = old_zip.with_extension("json");
            if old_json.exists() {
                let _ = fs::remove_file(old_json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::restore::SaveRestore;
    use std::io::Write as _;

    fn temp_root(sub: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sager_test_{}", sub));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn backup_then_restore_roundtrip() {
        // Fake game save folder
        let root = temp_root("src");
        let save = root.join("MyGame");
        fs::create_dir_all(&save).unwrap();
        fs::write(save.join("progress.dat"), b"level 7 boss down").unwrap();

        // Fake backup target (Google Drive)
        let mut cfg = SagerConfig::default();
        cfg.backup_directory = root.join("Backups");

        let info = SaveArchiver::backup_game("My Game", &save, &cfg).unwrap();
        assert!(info.full_path.is_file());

        // Manifest must round-trip through the zip
        let manifest = SaveRestore::read_manifest_from_zip(&info.full_path).unwrap();
        assert_eq!(manifest.game_name, "My Game");

        // Modify the save after the backup (like the game would)
        fs::write(save.join("progress.dat"), b"level 9 new run").unwrap();

        // Restore the newest archive — must overwrite the newer local save
        let backups = SaveRestore::list_backups_for_game("My Game", &cfg);
        assert_eq!(backups.len(), 1);
        let result = SaveRestore::restore_backup(&backups[0].full_path, None, &cfg).unwrap();
        assert_eq!(result.game_name, "My Game");
        assert_eq!(fs::read(save.join("progress.dat")).unwrap(), b"level 7 boss down");

        // Safety copy for the replaced save must exist
        assert!(result.safety_backup_path.is_some());

        let _ = fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn pruning_keeps_only_newest_archives() {
        let root = temp_root("prune");
        let save = root.join("GamePrune");
        fs::create_dir_all(&save).unwrap();
        fs::write(save.join("a.dat"), "data").unwrap();

        let mut cfg = SagerConfig::default();
        cfg.backup_directory = root.join("Backups");

        // Make several archives: each pass writes new content so the
        // differential check re-archives instead of skipping.
        for i in 0..14 {
            fs::write(save.join("a.dat"), format!("data {i}")).unwrap();
            SaveArchiver::backup_game("GamePrune", &save, &cfg).unwrap();
            if i < 13 {
                std::thread::sleep(std::time::Duration::from_millis(1100));
            }
        }

        let remaining = SaveRestore::list_backups_for_game("GamePrune", &cfg).len();
        assert!(remaining <= 10, "expected <=10 archives after pruning, got {remaining}");

        let _ = fs::remove_dir_all(&root).ok();
    }
}
