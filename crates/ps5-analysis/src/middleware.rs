use ps5_image::BinaryImageBuilder;
use ps5_nid::Catalog;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleKind {
    ThirdParty,
    Sony,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiddlewareId {
    pub vendor: &'static str,
    pub product: &'static str,
    pub description: &'static str,
}

const FMOD: MiddlewareId = MiddlewareId {
    vendor: "Firelight Technologies",
    product: "FMOD",
    description: "FMOD low-level audio engine",
};

const FMOD_STUDIO: MiddlewareId = MiddlewareId {
    vendor: "Firelight Technologies",
    product: "FMOD Studio",
    description: "FMOD Studio audio engine (banks and events)",
};

const WWISE_EFFECT: MiddlewareId = MiddlewareId {
    vendor: "Audiokinetic",
    product: "Wwise",
    description: "Wwise audio effect plugin",
};

const AURO: MiddlewareId = MiddlewareId {
    vendor: "Auro Technologies",
    product: "Auro-3D",
    description: "Auro-3D audio plugin (Wwise integration)",
};

const IZOTOPE: MiddlewareId = MiddlewareId {
    vendor: "iZotope",
    product: "iZotope",
    description: "iZotope audio processing plugin (Wwise integration)",
};

const MCDSP: MiddlewareId = MiddlewareId {
    vendor: "McDSP",
    product: "McDSP",
    description: "McDSP DSP audio plugin (Wwise integration)",
};

const MASTERING_SUITE: MiddlewareId = MiddlewareId {
    vendor: "iZotope",
    product: "Ozone Mastering Suite",
    description: "iZotope Ozone mastering suite (Wwise integration)",
};

const GAMEFACE: MiddlewareId = MiddlewareId {
    vendor: "Coherent Labs",
    product: "Gameface",
    description: "Gameface UI runtime",
};

const GAMEFACE_DEV: MiddlewareId = MiddlewareId {
    vendor: "Coherent Labs",
    product: "Gameface",
    description: "Gameface development build",
};

const GAMEFACE_CORE: MiddlewareId = MiddlewareId {
    vendor: "Coherent Labs",
    product: "Gameface",
    description: "Gameface core library",
};

const GAMEFACE_JS: MiddlewareId = MiddlewareId {
    vendor: "Coherent Labs",
    product: "Gameface",
    description: "Gameface JavaScript engine",
};

const ICU: MiddlewareId = MiddlewareId {
    vendor: "Unicode Consortium",
    product: "ICU",
    description: "International Components for Unicode",
};

const RENOIR: MiddlewareId = MiddlewareId {
    vendor: "SN Systems",
    product: "RENOIR",
    description: "GPU/CPU performance capture runtime",
};

const WTF: MiddlewareId = MiddlewareId {
    vendor: "Unknown",
    product: "WTF",
    description: "Unidentified third-party library",
};

const UNITY_IL2CPP: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity IL2CPP",
    description: "Unity IL2CPP user assemblies (AOT-compiled C#)",
};

const UNITY_BURST: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity Burst",
    description: "Unity Burst-compiled job system code",
};

const RESONANCE_AUDIO: MiddlewareId = MiddlewareId {
    vendor: "Google",
    product: "Resonance Audio",
    description: "Google Resonance Audio spatializer",
};

const CRIWARE_UNITY: MiddlewareId = MiddlewareId {
    vendor: "CRI Middleware",
    product: "CRIWARE",
    description: "CRIWARE Unity plugin (ADX audio)",
};

const WEBKIT_KITT: MiddlewareId = MiddlewareId {
    vendor: "Apple",
    product: "WebKit (Kitt)",
    description: "WebKit-based embedded browser",
};

const EOS_SDK: MiddlewareId = MiddlewareId {
    vendor: "Epic Games",
    product: "Epic Online Services",
    description: "Epic Online Services SDK",
};

const UNITY_PS5_PLATFORM: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity PS5 Platform",
    description: "Unity PS5 player support module",
};

const UNITY_PSN: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity PSN",
    description: "Unity PSN package (com.unity.psn.ps5)",
};

const UNITY_SAVE_DATA: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity PS5 Save Data",
    description: "Unity SaveData package (com.unity.savedata.ps5)",
};

const UNITY_COMMON_DIALOG: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity PS5 Common Dialog",
    description: "Unity PS5 common dialog module",
};

const UNITY_PS5_SPATIALIZER: MiddlewareId = MiddlewareId {
    vendor: "Unity Technologies",
    product: "Unity PS5 Audio Spatializer",
    description: "Unity PS5 audio spatializer plugin",
};

const CATALOG: &[(&str, &MiddlewareId)] = &[
    ("libfmodstudio", &FMOD_STUDIO),
    ("libfmod", &FMOD),
    ("libcoherentgtcore", &GAMEFACE_CORE),
    ("libcoherentgtjs", &GAMEFACE_JS),
    ("libcoherentuigt", &GAMEFACE),
    ("coherentuigtdevelopment", &GAMEFACE_DEV),
    ("coherentuigt", &GAMEFACE),
    ("libicuin", &ICU),
    ("librenoircore", &RENOIR),
    ("libwtf", &WTF),
    ("masteringsuite", &MASTERING_SUITE),
    ("mcdsp", &MCDSP),
    ("izotope", &IZOTOPE),
    ("auro", &AURO),
    ("ak", &WWISE_EFFECT),
    ("il2cppuserassemblies", &UNITY_IL2CPP),
    ("lib_burst_generated", &UNITY_BURST),
    ("libresonanceaudio", &RESONANCE_AUDIO),
    ("cri_ware_unity", &CRIWARE_UNITY),
    ("kitt", &WEBKIT_KITT),
    ("libeossdk", &EOS_SDK),
    ("eossdk", &EOS_SDK),
    ("eosnatlib", &EOS_SDK),
    ("ps5util", &UNITY_PS5_PLATFORM),
    ("psncore", &UNITY_PSN),
    ("psncommon", &UNITY_PSN),
    ("psn", &UNITY_PSN),
    ("savedata", &UNITY_SAVE_DATA),
    ("commondialog", &UNITY_COMMON_DIALOG),
    ("ps5audiospatializer", &UNITY_PS5_SPATIALIZER),
];

fn starts_with_ignore_ascii_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn is_sony_stem(stem: &str) -> bool {
    starts_with_ignore_ascii_case(stem, "libSce")
        || starts_with_ignore_ascii_case(stem, "libkernel")
        || matches!(
            stem.to_ascii_lowercase().as_str(),
            "libc" | "libc++" | "libc++abi" | "libm" | "sceaudio3d"
        )
}

pub fn match_middleware(stem: &str) -> Option<&'static MiddlewareId> {
    CATALOG
        .iter()
        .filter(|(p, _)| starts_with_ignore_ascii_case(stem, p))
        .max_by_key(|(p, _)| p.len())
        .map(|(_, id)| *id)
}

pub fn classify_stem(stem: &str) -> (ModuleKind, Option<&'static MiddlewareId>) {
    if is_sony_stem(stem) {
        (ModuleKind::Sony, None)
    } else if let Some(id) = match_middleware(stem) {
        (ModuleKind::ThirdParty, Some(id))
    } else {
        (ModuleKind::Unknown, None)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MiddlewareModule {
    pub file_name: String,
    pub module_name: String,
    pub kind: ModuleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parseable: bool,
    pub imports: usize,
    pub exports: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub import_libs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameMiddlewareReport {
    pub game: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    pub third_party: Vec<MiddlewareModule>,
    pub sony: Vec<MiddlewareModule>,
    pub unknown: Vec<MiddlewareModule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MiddlewareReport {
    pub games: Vec<GameMiddlewareReport>,
    pub total_prx: usize,
    pub third_party_modules: usize,
    pub sony_modules: usize,
    pub unknown_modules: usize,
}

pub fn build_middleware_report(root: &Path, catalog: &Catalog) -> MiddlewareReport {
    let mut games = Vec::new();
    let mut total_prx = 0;
    let mut third_party_modules = 0;
    let mut sony_modules = 0;
    let mut unknown_modules = 0;

    for game_dir in find_game_dirs(root) {
        let game = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let title_id = title_id_from_name(&game);
        let engine = detect_engine(&game_dir);

        let mut third_party = Vec::new();
        let mut sony = Vec::new();
        let mut unknown = Vec::new();
        let mut seen_sha = std::collections::HashSet::new();

        for path in find_module_files(&game_dir) {
            let module = analyze_module(&path, catalog);
            if module
                .sha256
                .as_ref()
                .is_some_and(|sha| !seen_sha.insert(sha.clone()))
            {
                continue;
            }
            total_prx += 1;
            match module.kind {
                ModuleKind::ThirdParty => third_party.push(module),
                ModuleKind::Sony => sony.push(module),
                ModuleKind::Unknown => unknown.push(module),
            }
        }

        third_party.sort_by(|a, b| a.module_name.cmp(&b.module_name));
        sony.sort_by(|a, b| a.module_name.cmp(&b.module_name));
        unknown.sort_by(|a, b| a.module_name.cmp(&b.module_name));

        third_party_modules += third_party.len();
        sony_modules += sony.len();
        unknown_modules += unknown.len();

        games.push(GameMiddlewareReport {
            game,
            title_id,
            engine,
            third_party,
            sony,
            unknown,
        });
    }

    games.sort_by(|a, b| a.game.cmp(&b.game));

    MiddlewareReport {
        games,
        total_prx,
        third_party_modules,
        sony_modules,
        unknown_modules,
    }
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    resolve_game_dir(root, &mut dirs);
    if !dirs.is_empty() {
        return dirs;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                resolve_game_dir(&entry.path(), &mut dirs);
            }
        }
    }
    dirs
}

fn resolve_game_dir(dir: &Path, result: &mut Vec<PathBuf>) {
    if dir.join("eboot.bin").exists() {
        result.push(dir.to_path_buf());
        return;
    }
    let children: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect(),
        Err(_) => return,
    };
    if children.len() == 1 {
        resolve_game_dir(&children[0], result);
    }
}

fn is_module_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        e.eq_ignore_ascii_case("prx")
            || e.eq_ignore_ascii_case("sprx")
            || e.eq_ignore_ascii_case("so")
    })
}

fn find_module_files(game_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_modules(game_dir, &mut result, 0);
    result.sort();
    result
}

fn walk_modules(dir: &Path, result: &mut Vec<PathBuf>, depth: usize) {
    if depth > 4 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str());
                if name.is_some_and(|n| {
                    n.eq_ignore_ascii_case("sce_sys") || n.eq_ignore_ascii_case("decrypted")
                }) {
                    continue;
                }
                walk_modules(&path, result, depth + 1);
            } else if path.is_file() && is_module_file(&path) {
                result.push(path);
            }
        }
    }
}

fn module_stem(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    for ext in [".prx", ".sprx", ".so"] {
        if lower.ends_with(ext) {
            return file_name[..file_name.len() - ext.len()].to_string();
        }
    }
    file_name.to_string()
}

fn analyze_module(path: &Path, catalog: &Catalog) -> MiddlewareModule {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let module_name = module_stem(&file_name);
    let (kind, id) = classify_stem(&module_name);

    let mut parseable = false;
    let mut imports = 0;
    let mut exports = 0;
    let mut import_libs = Vec::new();
    let mut needed_files = Vec::new();
    let mut module_sha256 = None;

    if let Ok(data) = std::fs::read(path) {
        let sha256 = ps5_format::sha256_hex(&data);
        let image = BinaryImageBuilder::build_from_file(&data, &sha256, catalog);
        parseable = image.is_self
            || image.metadata.elf_type != 0
            || !image.imports.is_empty()
            || !image.exports.is_empty();
        imports = image.imports.len();
        exports = image.exports.len();
        let mut libs: BTreeSet<String> = image.import_libs.values().cloned().collect();
        libs.extend(image.imports.iter().map(|i| i.library_name.clone()));
        import_libs = libs.into_iter().collect();
        needed_files = image.needed_files.iter().take(40).cloned().collect();
        module_sha256 = Some(sha256);
    }

    MiddlewareModule {
        file_name,
        module_name,
        kind,
        sha256: module_sha256,
        vendor: id.map(|i| i.vendor.to_string()),
        product: id.map(|i| i.product.to_string()),
        description: id.map(|i| i.description.to_string()),
        parseable,
        imports,
        exports,
        import_libs,
        needed_files,
    }
}

fn title_id_from_name(name: &str) -> Option<String> {
    if let Some(idx) = name.find("PPSA") {
        let digits: String = name[idx + 4..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.len() == 5 {
            return Some(format!("PPSA{digits}"));
        }
    }
    None
}

fn detect_engine(game_dir: &Path) -> Option<String> {
    if game_dir.join("engine").is_dir() {
        Some("Unreal Engine".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_fmod() {
        let (kind, id) = classify_stem("libfmod");
        assert_eq!(kind, ModuleKind::ThirdParty);
        assert_eq!(id.unwrap().vendor, "Firelight Technologies");
        assert_eq!(id.unwrap().product, "FMOD");
    }

    #[test]
    fn classify_fmod_studio_precedes_fmod() {
        let (_, id) = classify_stem("libfmodstudio");
        assert_eq!(id.unwrap().product, "FMOD Studio");
    }

    #[test]
    fn classify_wwise_effect() {
        for stem in [
            "akroomverb",
            "akaudioinput",
            "akconvolutionreverb",
            "aksoundseedimpact",
        ] {
            let (kind, id) = classify_stem(stem);
            assert_eq!(kind, ModuleKind::ThirdParty);
            assert_eq!(id.unwrap().product, "Wwise");
        }
    }

    #[test]
    fn classify_coherent_longest_match() {
        let (_, id) = classify_stem("coherentuigtdevelopment");
        assert_eq!(id.unwrap().description, "Gameface development build");
        let (_, id) = classify_stem("coherentuigt");
        assert_eq!(id.unwrap().description, "Gameface UI runtime");
    }

    #[test]
    fn classify_sony_libs() {
        for stem in [
            "libSceFace",
            "libSceJobManager",
            "libSceNpCppWebApi",
            "libkernel",
            "libc",
            "sceaudio3d",
        ] {
            assert_eq!(classify_stem(stem).0, ModuleKind::Sony);
        }
    }

    #[test]
    fn classify_unknown_lib() {
        assert_eq!(classify_stem("mysterylib").0, ModuleKind::Unknown);
    }

    #[test]
    fn classify_unity_il2cpp_case_variants() {
        for stem in ["Il2CppUserAssemblies", "Il2cppUserAssemblies"] {
            let (kind, id) = classify_stem(stem);
            assert_eq!(kind, ModuleKind::ThirdParty);
            assert_eq!(id.unwrap().product, "Unity IL2CPP");
        }
    }

    #[test]
    fn classify_unity_burst() {
        let (kind, id) = classify_stem("lib_burst_generated");
        assert_eq!(kind, ModuleKind::ThirdParty);
        assert_eq!(id.unwrap().product, "Unity Burst");
    }

    #[test]
    fn classify_resonance_audio() {
        let (kind, id) = classify_stem("libresonanceaudio");
        assert_eq!(kind, ModuleKind::ThirdParty);
        assert_eq!(id.unwrap().vendor, "Google");
        assert_eq!(id.unwrap().product, "Resonance Audio");
    }

    #[test]
    fn classify_criware_unity() {
        let (kind, id) = classify_stem("cri_ware_unity");
        assert_eq!(kind, ModuleKind::ThirdParty);
        assert_eq!(id.unwrap().vendor, "CRI Middleware");
        assert_eq!(id.unwrap().product, "CRIWARE");
    }

    #[test]
    fn classify_webkit_kitt_variants() {
        for stem in [
            "kitt-ps5-shipping",
            "kitt_webkit-ps5-shipping",
            "kitt_support-ps5-shipping",
        ] {
            let (kind, id) = classify_stem(stem);
            assert_eq!(kind, ModuleKind::ThirdParty);
            assert_eq!(id.unwrap().product, "WebKit (Kitt)");
        }
    }

    #[test]
    fn classify_eos_sdk() {
        for stem in ["EOSSDK", "libEOSSDK", "EOSNatLib"] {
            let (kind, id) = classify_stem(stem);
            assert_eq!(kind, ModuleKind::ThirdParty);
            assert_eq!(id.unwrap().vendor, "Epic Games");
            assert_eq!(id.unwrap().product, "Epic Online Services");
        }
    }

    #[test]
    fn classify_unity_ps5_platform_modules() {
        for (stem, product) in [
            ("PS5Util", "Unity PS5 Platform"),
            ("PSN", "Unity PSN"),
            ("PSNCore", "Unity PSN"),
            ("PSNCommon", "Unity PSN"),
            ("SaveData", "Unity PS5 Save Data"),
            ("CommonDialog", "Unity PS5 Common Dialog"),
            ("PS5AudioSpatializer", "Unity PS5 Audio Spatializer"),
        ] {
            let (kind, id) = classify_stem(stem);
            assert_eq!(kind, ModuleKind::ThirdParty);
            assert_eq!(id.unwrap().vendor, "Unity Technologies");
            assert_eq!(id.unwrap().product, product);
        }
    }

    #[test]
    fn classify_renoircore_ps5() {
        let (kind, id) = classify_stem("librenoircore.ps5");
        assert_eq!(kind, ModuleKind::ThirdParty);
        assert_eq!(id.unwrap().product, "RENOIR");
    }

    #[test]
    fn module_stem_strips_extension() {
        assert_eq!(module_stem("libfmod.prx"), "libfmod");
        assert_eq!(module_stem("librenoircore.ps5.prx"), "librenoircore.ps5");
        assert_eq!(module_stem("libSceFace.prx"), "libSceFace");
    }

    #[test]
    fn title_id_from_game_name() {
        assert_eq!(
            title_id_from_name("Hello.Neighbor.2-PPSA07426-EUR-Game-v01.000-PS5").as_deref(),
            Some("PPSA07426")
        );
        assert_eq!(title_id_from_name("Asterigos"), None);
    }

    #[test]
    fn report_groups_modules_by_kind() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("TestGame-PPSA12345-PS5");
        std::fs::create_dir_all(game.join("prx")).unwrap();
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::create_dir_all(game.join("engine")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"not an elf").unwrap();
        std::fs::write(game.join("prx/libfmod.prx"), b"fmod bytes").unwrap();
        std::fs::write(game.join("prx/akroomverb.prx"), b"ak bytes").unwrap();
        std::fs::write(game.join("sce_module/libSceFace.prx"), b"face bytes").unwrap();
        std::fs::write(game.join("prx/mystery.prx"), b"mystery bytes").unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.games.len(), 1);
        let game_report = &report.games[0];
        assert_eq!(game_report.title_id.as_deref(), Some("PPSA12345"));
        assert_eq!(game_report.engine.as_deref(), Some("Unreal Engine"));
        assert_eq!(game_report.third_party.len(), 2);
        assert_eq!(game_report.sony.len(), 1);
        assert_eq!(game_report.unknown.len(), 1);
        assert_eq!(game_report.third_party[0].product.as_deref(), Some("Wwise"));
        assert_eq!(game_report.third_party[1].product.as_deref(), Some("FMOD"));
        assert_eq!(report.total_prx, 4);
        assert_eq!(report.third_party_modules, 2);
        assert_eq!(report.sony_modules, 1);
        assert_eq!(report.unknown_modules, 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_scans_single_game_dir_directly() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_single_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("DeSR PS5");
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(game.join("sce_module/libc.prx"), b"libc bytes").unwrap();
        std::fs::write(game.join("sce_module/libSceNpCppWebApi.prx"), b"np bytes").unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.games.len(), 1);
        assert_eq!(report.total_prx, 2);
        assert_eq!(report.sony_modules, 2);
        assert_eq!(report.games[0].third_party.len(), 0);
        assert_eq!(report.games[0].sony[0].module_name, "libSceNpCppWebApi");
        assert_eq!(report.games[0].sony[1].module_name, "libc");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_finds_modules_outside_prx_dir_when_prx_dir_is_populated() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_deep_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("MixedGame-PPSA99999-PS5");
        std::fs::create_dir_all(game.join("prx")).unwrap();
        std::fs::create_dir_all(game.join("Media/Plugins")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(game.join("prx/libSceFace.prx"), b"face bytes").unwrap();
        std::fs::write(game.join("Media/Plugins/libfmod.prx"), b"fmod bytes").unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.games.len(), 1);
        assert_eq!(report.total_prx, 2);
        assert_eq!(report.sony_modules, 1);
        assert_eq!(report.third_party_modules, 1);
        assert_eq!(report.games[0].third_party[0].module_name, "libfmod");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_finds_modules_in_nested_dump_subdirs() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_nested_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("Game-PPSA00001-PS5");
        std::fs::create_dir_all(game.join("PPSA00001-app/Media/Plugins")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(
            game.join("PPSA00001-app/Media/Plugins/libfmod.prx"),
            b"junk",
        )
        .unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.total_prx, 1);
        assert_eq!(report.third_party_modules, 1);
        assert_eq!(report.games[0].third_party[0].module_name, "libfmod");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_ignores_sce_sys_about_images() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_scesys_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("Game-PPSA00002-PS5");
        std::fs::create_dir_all(game.join("sce_sys/about")).unwrap();
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(game.join("sce_sys/about/right.sprx"), b"junk").unwrap();
        std::fs::write(game.join("sce_module/libSceFace.prx"), b"junk").unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.total_prx, 1);
        assert_eq!(report.sony_modules, 1);
        assert!(
            !report.games[0]
                .unknown
                .iter()
                .any(|m| m.file_name == "right.sprx")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_skips_decrypted_copies() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_dec_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("Game-PPSA00005-PS5");
        std::fs::create_dir_all(game.join("decrypted/Media/Modules")).unwrap();
        std::fs::create_dir_all(game.join("Media/Modules")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(
            game.join("decrypted/Media/Modules/Il2CppUserAssemblies.prx"),
            b"decrypted elf bytes",
        )
        .unwrap();
        std::fs::write(
            game.join("Media/Modules/Il2CppUserAssemblies.prx"),
            b"self bytes",
        )
        .unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.total_prx, 1);
        assert_eq!(report.third_party_modules, 1);
        assert_eq!(report.games[0].third_party.len(), 1);
        assert_eq!(
            report.games[0].third_party[0].file_name,
            "Il2CppUserAssemblies.prx"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_dedupes_identical_module_copies_within_a_game() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_dedup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("Game-PPSA00003-PS5");
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::create_dir_all(game.join("PPSA00003-app/sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(game.join("sce_module/libfmod.prx"), b"same bytes").unwrap();
        std::fs::write(
            game.join("PPSA00003-app/sce_module/libfmod.prx"),
            b"same bytes",
        )
        .unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.total_prx, 1);
        assert_eq!(report.third_party_modules, 1);
        assert_eq!(report.games[0].third_party.len(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn report_keeps_distinct_versions_of_same_named_module() {
        let root = std::env::temp_dir().join(format!("ps5rs_mw_vers_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let game = root.join("Game-PPSA00004-PS5");
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::create_dir_all(game.join("PPSA00004-app/sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), b"junk").unwrap();
        std::fs::write(game.join("sce_module/libfmod.prx"), b"version one").unwrap();
        std::fs::write(
            game.join("PPSA00004-app/sce_module/libfmod.prx"),
            b"version two",
        )
        .unwrap();

        let catalog = Catalog::default();
        let report = build_middleware_report(&root, &catalog);

        assert_eq!(report.total_prx, 2);
        assert_eq!(report.third_party_modules, 2);
        assert_eq!(report.games[0].third_party.len(), 2);

        let _ = std::fs::remove_dir_all(&root);
    }
}
