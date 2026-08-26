//! Game directory discovery: locate the app0 directory, eboot.bin and the
//! optional param.sfo / playgo-chunk / icon0.png assets.

use std::fs;
use std::path::Path;

use crate::error::{BleError, BleResult};

#[derive(Debug, Clone)]
pub struct GameDirectory {
    pub app0_dir: String,
    pub eboot_path: String,
    pub param_sfo: Option<String>,
    pub playgo_path: Option<String>,
    pub icon_path: Option<String>,
}

impl GameDirectory {
    pub fn from_path(path: &str) -> BleResult<Self> {
        let path = Path::new(path);
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            let parent = path
                .parent()
                .ok_or_else(|| BleError::Loader("cannot determine parent directory".to_string()))?;

            let app0_dir = parent.to_string_lossy().to_string();
            let eboot_full = path.to_string_lossy().to_string();

            let sce_sys_dir = format!("{}/sce_sys", app0_dir);
            let param_sfo_path = format!("{}/param.sfo", sce_sys_dir);
            let playgo_path_val = format!("{}/playgo-chunk", sce_sys_dir);
            let icon_path_val = format!("{}/icon0.png", sce_sys_dir);

            Ok(Self {
                app0_dir,
                eboot_path: eboot_full,
                param_sfo: if Path::new(&param_sfo_path).exists() {
                    Some(param_sfo_path)
                } else {
                    None
                },
                playgo_path: if Path::new(&playgo_path_val).exists() {
                    Some(playgo_path_val)
                } else {
                    None
                },
                icon_path: if Path::new(&icon_path_val).exists() {
                    Some(icon_path_val)
                } else {
                    None
                },
            })
        } else if metadata.is_dir() {
            let app0_dir = path.to_string_lossy().to_string();

            let eboot_candidates = [
                format!("{}/eboot.bin", app0_dir),
                format!("{}/app0/eboot.bin", app0_dir),
                format!("{}/APP0/eboot.bin", app0_dir),
                format!("{}/App0/eboot.bin", app0_dir),
                format!("{}/EBOOT.BIN", app0_dir),
            ];

            let mut eboot_full = String::new();
            let mut found = false;
            for candidate in &eboot_candidates {
                if Path::new(candidate).exists() {
                    eboot_full = candidate.clone();
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(BleError::Loader(format!(
                    "eboot.bin not found in {} (checked root, app0/, APP0/, App0/)",
                    app0_dir
                )));
            }

            let sce_sys_candidates = [
                format!("{}/sce_sys", app0_dir),
                format!("{}/app0/sce_sys", app0_dir),
                format!("{}/sce_sys/param.sfo", app0_dir),
            ];

            let mut sce_sys_dir = String::new();
            for candidate in &sce_sys_candidates {
                let p = Path::new(candidate);
                if p.exists() && p.is_dir() {
                    sce_sys_dir = candidate.clone();
                    break;
                }
            }

            if sce_sys_dir.is_empty() {
                sce_sys_dir = format!("{}/sce_sys", app0_dir);
            }

            let param_sfo_path = format!("{}/param.sfo", sce_sys_dir);
            let playgo_path_val = format!("{}/playgo-chunk", sce_sys_dir);
            let icon_path_val = format!("{}/icon0.png", sce_sys_dir);

            Ok(Self {
                app0_dir: app0_dir.clone(),
                eboot_path: eboot_full,
                param_sfo: if Path::new(&param_sfo_path).exists() {
                    Some(param_sfo_path)
                } else {
                    None
                },
                playgo_path: if Path::new(&playgo_path_val).exists() {
                    Some(playgo_path_val)
                } else {
                    None
                },
                icon_path: if Path::new(&icon_path_val).exists() {
                    Some(icon_path_val)
                } else {
                    None
                },
            })
        } else {
            Err(BleError::Loader("invalid game path".to_string()))
        }
    }
}
