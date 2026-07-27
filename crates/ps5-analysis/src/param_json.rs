use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameParam {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub title_id: Option<String>,
    pub content_version: Option<String>,
    pub master_version: Option<String>,
    pub sdk_version: Option<String>,
    pub title_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub drm_type: Option<String>,
    pub content_id: Option<String>,
    pub creation_date: Option<String>,
}

impl GameParam {
    pub fn compute_display_name(&self) -> Option<String> {
        match (&self.title_name, &self.title_id) {
            (Some(name), Some(id)) => Some(format!("{name} - [{id}]")),
            (None, Some(id)) => Some(format!("[{id}]")),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawParamJson {
    #[serde(default)]
    title_id: Option<String>,
    #[serde(default)]
    content_version: Option<String>,
    #[serde(default)]
    master_version: Option<String>,
    #[serde(default)]
    sdk_version: Option<String>,
    #[serde(default)]
    localized_parameters: Option<serde_json::Value>,
    #[serde(default)]
    application_drm_type: Option<String>,
    #[serde(default)]
    content_id: Option<String>,
    #[serde(default)]
    pubtools: Option<serde_json::Value>,
}

pub fn read_param(game_dir: &Path) -> Option<GameParam> {
    let param_path = find_param_json(game_dir)?;
    let data = std::fs::read_to_string(&param_path).ok()?;
    let raw: RawParamJson = serde_json::from_str(&data).ok()?;

    let title_name = raw
        .localized_parameters
        .as_ref()
        .and_then(|lp| lp.get("defaultLanguage"))
        .and_then(|dl| dl.as_str())
        .and_then(|lang| {
            raw.localized_parameters
                .as_ref()?
                .get(lang)?
                .get("titleName")?
                .as_str()
                .map(|s| s.to_string())
        });

    let creation_date = raw
        .pubtools
        .as_ref()
        .and_then(|pt| pt.get("creationDate"))
        .and_then(|cd| cd.as_str())
        .map(|s| s.to_string());

    let mut param = GameParam {
        name: None,
        title_id: raw.title_id,
        content_version: raw.content_version,
        master_version: raw.master_version,
        sdk_version: raw.sdk_version,
        title_name,
        display_name: None,
        drm_type: raw.application_drm_type,
        content_id: raw.content_id,
        creation_date,
    };
    param.display_name = param.compute_display_name();
    Some(param)
}

fn find_param_json(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("sce_sys").join("param.json");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    find_param_json_recursive(start, 3)
}

fn find_param_json_recursive(dir: &Path, depth: usize) -> Option<std::path::PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let candidate = path.join("sce_sys").join("param.json");
            if candidate.exists() {
                return Some(candidate);
            }
            if let Some(found) = find_param_json_recursive(&path, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tempdir_for_test() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ps5rs_param_json_test_{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn read_param_valid() {
        let tmp = tempdir_for_test().join("valid_game");
        let sce_sys = tmp.join("sce_sys");
        fs::create_dir_all(&sce_sys).unwrap();
        fs::write(
            sce_sys.join("param.json"),
            r#"{
                "titleId": "PPSA10264",
                "contentVersion": "02.000.000",
                "masterVersion": "02.00",
                "sdkVersion": "0x0700000000000000",
                "applicationDrmType": "standard",
                "contentId": "EP8091-PPSA10264_00-4648066903011525",
                "localizedParameters": {
                    "defaultLanguage": "en-US",
                    "en-US": {"titleName": "Jusant"}
                },
                "pubtools": {
                    "creationDate": "2024-07-04 10:53:05"
                }
            }"#,
        )
        .unwrap();

        let param = read_param(&tmp).unwrap();
        assert_eq!(param.title_id.as_deref(), Some("PPSA10264"));
        assert_eq!(param.content_version.as_deref(), Some("02.000.000"));
        assert_eq!(param.master_version.as_deref(), Some("02.00"));
        assert_eq!(param.sdk_version.as_deref(), Some("0x0700000000000000"));
        assert_eq!(param.title_name.as_deref(), Some("Jusant"));
        assert_eq!(param.display_name.as_deref(), Some("Jusant - [PPSA10264]"));
        assert_eq!(param.drm_type.as_deref(), Some("standard"));
        assert_eq!(
            param.content_id.as_deref(),
            Some("EP8091-PPSA10264_00-4648066903011525")
        );
        assert_eq!(
            param.creation_date.as_deref(),
            Some("2024-07-04 10:53:05")
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_param_missing() {
        let tmp = tempdir_for_test().join("no_param");
        fs::create_dir_all(&tmp).unwrap();
        assert!(read_param(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_param_malformed_json() {
        let tmp = tempdir_for_test().join("malformed");
        let sce_sys = tmp.join("sce_sys");
        fs::create_dir_all(&sce_sys).unwrap();
        fs::write(sce_sys.join("param.json"), "not json").unwrap();
        assert!(read_param(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_param_partial_fields() {
        let tmp = tempdir_for_test().join("partial");
        let sce_sys = tmp.join("sce_sys");
        fs::create_dir_all(&sce_sys).unwrap();
        fs::write(
            sce_sys.join("param.json"),
            r#"{"titleId": "PPSA01502"}"#,
        )
        .unwrap();

        let param = read_param(&tmp).unwrap();
        assert_eq!(param.title_id.as_deref(), Some("PPSA01502"));
        assert!(param.content_version.is_none());
        assert!(param.title_name.is_none());
        assert_eq!(param.display_name.as_deref(), Some("[PPSA01502]"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_param_walks_parent() {
        let tmp = tempdir_for_test().join("parent_search");
        let game_dir = tmp.join("nested").join("deep");
        let sce_sys = tmp.join("sce_sys");
        fs::create_dir_all(&game_dir).unwrap();
        fs::create_dir_all(&sce_sys).unwrap();
        fs::write(
            sce_sys.join("param.json"),
            r#"{"titleId": "PPSA00001"}"#,
        )
        .unwrap();

        let param = read_param(&game_dir).unwrap();
        assert_eq!(param.title_id.as_deref(), Some("PPSA00001"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn serde_roundtrip() {
        let param = GameParam {
            name: None,
            title_id: Some("PPSA10264".to_string()),
            content_version: Some("02.000.000".to_string()),
            master_version: None,
            sdk_version: None,
            title_name: Some("Jusant".to_string()),
            display_name: Some("Jusant - [PPSA10264]".to_string()),
            drm_type: Some("standard".to_string()),
            content_id: None,
            creation_date: None,
        };
        let json = serde_json::to_string(&param).unwrap();
        let back: GameParam = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title_id.as_deref(), Some("PPSA10264"));
        assert_eq!(back.display_name.as_deref(), Some("Jusant - [PPSA10264]"));
        assert!(back.master_version.is_none());
    }

    #[test]
    fn default_game_param_is_all_none() {
        let param = GameParam::default();
        assert!(param.title_id.is_none());
        assert!(param.content_version.is_none());
    }

    #[test]
    fn compute_display_name_both() {
        let p = GameParam { title_name: Some("Bugsnax".into()), title_id: Some("PPSA01502".into()), ..Default::default() };
        assert_eq!(p.compute_display_name().as_deref(), Some("Bugsnax - [PPSA01502]"));
    }

    #[test]
    fn compute_display_name_id_only() {
        let p = GameParam { title_id: Some("PPSA01502".into()), ..Default::default() };
        assert_eq!(p.compute_display_name().as_deref(), Some("[PPSA01502]"));
    }

    #[test]
    fn compute_display_name_neither() {
        let p = GameParam::default();
        assert!(p.compute_display_name().is_none());
    }

    #[test]
    fn read_param_nested_subdir() {
        let tmp = tempdir_for_test().join("nested_subdir");
        let game_dir = tmp.join("outer-game-dir");
        let inner_dir = game_dir.join("Inner Game Name PPSA00001");
        let sce_sys = inner_dir.join("sce_sys");
        fs::create_dir_all(&sce_sys).unwrap();
        fs::write(
            sce_sys.join("param.json"),
            r#"{"titleId": "PPSA00001", "contentVersion": "01.000.000"}"#,
        )
        .unwrap();

        let param = read_param(&game_dir).unwrap();
        assert_eq!(param.title_id.as_deref(), Some("PPSA00001"));
        assert_eq!(param.content_version.as_deref(), Some("01.000.000"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
