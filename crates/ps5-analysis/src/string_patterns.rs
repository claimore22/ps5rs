use ps5_image::{Detection, StringAnalysis};
use std::collections::BTreeSet;

const MAX_STRING_LENGTH: usize = 4096;

pub fn extract_strings(data: &[u8], min_length: usize) -> Vec<String> {
    let mut strings = Vec::new();
    let mut start: Option<usize> = None;

    for (i, &byte) in data.iter().enumerate() {
        let printable = byte.is_ascii_graphic() || byte == b' ';
        if printable {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start {
            let len = i - s;
            if len >= min_length
                && len <= MAX_STRING_LENGTH
                && let Ok(string) = std::str::from_utf8(&data[s..i])
            {
                strings.push(string.to_string());
            }
            start = None;
        }
    }

    if let Some(s) = start
        && data.len() - s >= min_length
        && data.len() - s <= MAX_STRING_LENGTH
        && let Ok(string) = std::str::from_utf8(&data[s..])
    {
        strings.push(string.to_string());
    }

    strings
}

pub fn detect_sce_libraries(strings: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    for s in strings {
        if s.contains("libSce")
            && (s.ends_with(".prx") || s.ends_with(".sprx") || s.ends_with(".so"))
        {
            found.insert(s.clone());
        }
    }
    found.into_iter().collect()
}

pub fn detect_third_party(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();

    detect_pattern(
        strings,
        &["PhysX", "PX_", "PhysXScene", "PxRigidActor", "PhysXCooking"],
        "PhysX",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["libVorbis", "Xiph.Org libVorbis"],
        "libVorbis",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["libpng version", "libpng "],
        "libpng",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["libopus", "Opus audio codec"],
        "libopus",
        &mut detections,
    );
    detect_pattern(strings, &["OpenSSL"], "OpenSSL", &mut detections);
    detect_pattern(strings, &["Bink Video", "Bink2"], "Bink", &mut detections);
    detect_pattern(
        strings,
        &["libsamplerate", "Secret Rabbit Code"],
        "libsamplerate",
        &mut detections,
    );
    detect_pattern(strings, &["zlib"], "zlib", &mut detections);
    detect_pattern(strings, &["libcrunch", "Crunch"], "Crunch", &mut detections);
    detect_pattern(
        strings,
        &["Oodle", "Kraken", "Leviathan"],
        "Oodle",
        &mut detections,
    );

    detections
}

pub fn detect_engine(strings: &[String]) -> Option<Detection> {
    crate::engine_fingerprints::detect_engine(strings)
}

pub fn detect_custom_forks(strings: &[String]) -> Vec<Detection> {
    crate::engine_fingerprints::detect_custom_forks(strings)
}

pub fn detect_build_system(strings: &[String]) -> Option<Detection> {
    let mut evidence = Vec::new();

    for s in strings {
        if s.contains("Jenkins") || s.contains("jenkins") {
            evidence.push(s.clone());
        }
    }
    if !evidence.is_empty() {
        return Some(Detection {
            value: "Jenkins".to_string(),
            score: 0,
            confidence: 0,
            evidence,
        });
    }

    for s in strings {
        if s.contains("BuildServer") || s.contains("build_server") {
            evidence.push(s.clone());
        }
    }
    if !evidence.is_empty() {
        return Some(Detection {
            value: "BuildServer".to_string(),
            score: 0,
            confidence: 0,
            evidence,
        });
    }

    None
}

pub fn detect_depot(strings: &[String]) -> Option<Detection> {
    let mut evidence = Vec::new();
    let mut depot_name = None;

    for s in strings {
        // Windows drive-letter paths: U:/P4Damascus/..., X:/Jenkins/..., etc.
        if s.len() >= 4
            && s.as_bytes()[0].is_ascii_alphabetic()
            && s.as_bytes()[1] == b':'
            && (s.as_bytes()[2] == b'/' || s.as_bytes()[2] == b'\\')
            && let Some(rest) = s.get(3..)
            && let Some(first_component) = rest.split(['/', '\\']).next()
            && first_component.len() >= 2
            && first_component
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            evidence.push(s.clone());
            if depot_name.is_none() {
                depot_name = Some(first_component.to_string());
            }
        }
    }

    if !evidence.is_empty() {
        evidence.truncate(10);
        return Some(Detection {
            value: depot_name.unwrap_or_else(|| "Unknown".to_string()),
            score: 0,
            confidence: 0,
            evidence,
        });
    }

    None
}

pub fn detect_project_paths(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();
    let mut evidence = Vec::new();
    let mut project_name = None;

    for s in strings {
        if s.contains("sharedspace/") || s.contains("sharedspace\\") {
            evidence.push(s.clone());
            if project_name.is_none() {
                if let Some(idx) = s.find("sharedspace/") {
                    let rest = &s[idx + "sharedspace/".len()..];
                    let name = rest.split(['/', '\\']).next().unwrap_or("");
                    if !name.is_empty() {
                        project_name = Some(name.to_string());
                    }
                } else if let Some(idx) = s.find("sharedspace\\") {
                    let rest = &s[idx + "sharedspace\\".len()..];
                    let name = rest.split(['/', '\\']).next().unwrap_or("");
                    if !name.is_empty() {
                        project_name = Some(name.to_string());
                    }
                }
            }
        }
    }

    if !evidence.is_empty() {
        evidence.truncate(10);
        detections.push(Detection {
            value: project_name.unwrap_or_else(|| "Unknown".to_string()),
            score: 0,
            confidence: 0,
            evidence,
        });
    }

    detections
}

pub fn detect_sdk_hints(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();

    detect_pattern(
        strings,
        &["Prospero SDK", "prospero"],
        "Prospero SDK",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["ORBIS SDK", "orbis"],
        "ORBIS SDK",
        &mut detections,
    );
    detect_pattern(strings, &["SCE SDK", "sce_"], "SCE SDK", &mut detections);

    detections
}

pub fn detect_versions(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();

    detect_version_pattern(strings, "PhysX", &["PhysX ", "PhysX"], &mut detections);
    detect_version_pattern(
        strings,
        "libVorbis",
        &["Xiph.Org libVorbis "],
        &mut detections,
    );
    detect_version_pattern(
        strings,
        "libpng",
        &["libpng version ", "libpng "],
        &mut detections,
    );
    detect_version_pattern(strings, "OpenSSL", &["OpenSSL "], &mut detections);
    detect_version_pattern(strings, "zlib", &["zlib "], &mut detections);
    detect_version_pattern(strings, "libopus", &["libopus "], &mut detections);
    detect_version_pattern(
        strings,
        "libsamplerate",
        &["libsamplerate-"],
        &mut detections,
    );

    detections
}

pub fn detect_source_paths(strings: &[String]) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for s in strings {
        if s.contains("Engine/Source/")
            || s.contains("Engine\\Source\\")
            || s.contains("Engine/Plugins/")
            || s.contains("Engine\\Plugins\\")
        {
            paths.insert(s.clone());
        }
    }
    paths.into_iter().take(20).collect()
}

pub fn analyze_strings(data: &[u8]) -> StringAnalysis {
    let strings = extract_strings(data, 4);

    let sce_libraries = detect_sce_libraries(&strings);
    let third_party_libs = detect_third_party(&strings);
    let engine = detect_engine(&strings);
    let build_system = detect_build_system(&strings);
    let source_depot = detect_depot(&strings);
    let sdk_hints = detect_sdk_hints(&strings);
    let detected_versions = detect_versions(&strings);
    let source_paths = detect_source_paths(&strings);
    let project_paths = detect_project_paths(&strings);
    let custom_forks = detect_custom_forks(&strings);

    StringAnalysis {
        sce_libraries,
        third_party_libs,
        engine,
        build_system,
        source_depot,
        sdk_hints,
        detected_versions,
        source_paths,
        project_paths,
        custom_forks,
    }
}

fn detect_pattern(
    strings: &[String],
    patterns: &[&str],
    name: &str,
    detections: &mut Vec<Detection>,
) {
    let mut evidence = Vec::new();
    for s in strings {
        if patterns.iter().any(|p| s.contains(p)) {
            evidence.push(s.clone());
        }
    }
    if !evidence.is_empty() {
        evidence.truncate(10);
        detections.push(Detection {
            value: name.to_string(),
            score: 0,
            confidence: 0,
            evidence,
        });
    }
}

fn detect_version_pattern(
    strings: &[String],
    name: &str,
    patterns: &[&str],
    detections: &mut Vec<Detection>,
) {
    let mut evidence = Vec::new();
    for s in strings {
        if patterns.iter().any(|p| s.contains(p)) {
            evidence.push(s.clone());
        }
    }
    if !evidence.is_empty() {
        evidence.truncate(5);
        detections.push(Detection {
            value: name.to_string(),
            score: 0,
            confidence: 0,
            evidence,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_strings_basic() {
        let data = b"hello world\x00foo bar\x00";
        let strings = extract_strings(data, 3);
        assert!(strings.contains(&"hello world".to_string()));
        assert!(strings.contains(&"foo bar".to_string()));
    }

    #[test]
    fn extract_strings_min_length() {
        let data = b"ab\x00abc\x00abcd\x00";
        let strings = extract_strings(data, 4);
        assert!(!strings.contains(&"ab".to_string()));
        assert!(!strings.contains(&"abc".to_string()));
        assert!(strings.contains(&"abcd".to_string()));
    }

    #[test]
    fn extract_strings_max_length() {
        let long = vec![b'A'; 5000];
        let mut data = long.clone();
        data.push(0);
        let strings = extract_strings(&data, 4);
        assert!(strings.is_empty());
    }

    #[test]
    fn extract_strings_at_max_length() {
        let exact = vec![b'A'; MAX_STRING_LENGTH];
        let mut data = exact;
        data.push(0);
        let strings = extract_strings(&data, 4);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].len(), MAX_STRING_LENGTH);
    }

    #[test]
    fn detect_sce_libraries_prx() {
        let strings = vec![
            "libScePad.prx".to_string(),
            "libSceKernel.sprx".to_string(),
            "libSceGnmDriver.so".to_string(),
            "libNotASce.prx".to_string(),
            "libSce".to_string(),
        ];
        let libs = detect_sce_libraries(&strings);
        assert!(libs.contains(&"libScePad.prx".to_string()));
        assert!(libs.contains(&"libSceKernel.sprx".to_string()));
        assert!(libs.contains(&"libSceGnmDriver.so".to_string()));
        assert!(!libs.contains(&"libNotASce.prx".to_string()));
        assert!(!libs.contains(&"libSce".to_string()));
    }

    #[test]
    fn detect_sce_libraries_embedded_path() {
        let strings = vec![
            "/system/common/lib/libScePad.prx".to_string(),
            "/app0/sce_module/libSceUserService.sprx".to_string(),
        ];
        let libs = detect_sce_libraries(&strings);
        assert!(libs.contains(&"/system/common/lib/libScePad.prx".to_string()));
        assert!(libs.contains(&"/app0/sce_module/libSceUserService.sprx".to_string()));
    }

    #[test]
    fn detect_engine_unreal() {
        let strings = vec![
            "UnrealEngine4Runtime".to_string(),
            "FShaderPipelineCache".to_string(),
            "Engine/Source/Runtime/Core/Private/".to_string(),
        ];
        let engine = detect_engine(&strings);
        assert!(engine.is_some());
        let engine = engine.unwrap();
        assert_eq!(engine.value, "Unreal Engine 4");
        assert_eq!(engine.evidence.len(), 3);
    }

    #[test]
    fn detect_engine_ue5_nanite() {
        let strings = vec!["UnrealEngine4Runtime".to_string(), "Nanite".to_string()];
        let engine = detect_engine(&strings);
        assert!(engine.is_some());
        // UE4 (100) beats UE5 Nanite (10) — Nanite alone isn't enough
        assert_eq!(engine.unwrap().value, "Unreal Engine 4");
    }

    #[test]
    fn detect_engine_ue4_not_false_positive() {
        // "UE5Something" alone should NOT trigger UE5
        let strings = vec![
            "UnrealEngine4Runtime".to_string(),
            "UE5SomethingUnrelated".to_string(),
        ];
        let engine = detect_engine(&strings);
        assert!(engine.is_some());
        let engine = engine.unwrap();
        // Should be UE4 because we only match strong UE5 patterns
        assert_eq!(engine.value, "Unreal Engine 4");
    }

    #[test]
    fn detect_engine_unity() {
        let strings = vec![
            "UnityEngine.dll".to_string(),
            "global-metadata.dat".to_string(),
        ];
        let engine = detect_engine(&strings);
        assert!(engine.is_some());
        assert_eq!(engine.unwrap().value, "Unity");
    }

    #[test]
    fn detect_engine_godot() {
        let strings = vec!["Godot Engine".to_string()];
        let engine = detect_engine(&strings);
        assert!(engine.is_some());
        assert_eq!(engine.unwrap().value, "Godot");
    }

    #[test]
    fn detect_engine_none() {
        let strings = vec!["hello world".to_string()];
        assert!(detect_engine(&strings).is_none());
    }

    #[test]
    fn detect_build_system_jenkins() {
        let strings = vec![
            "X:/Jenkins/sharedspace/Build/".to_string(),
            "Build from Jenkins".to_string(),
        ];
        let bs = detect_build_system(&strings);
        assert!(bs.is_some());
        assert_eq!(bs.unwrap().value, "Jenkins");
    }

    #[test]
    fn detect_build_system_none() {
        let strings = vec!["hello world".to_string()];
        assert!(detect_build_system(&strings).is_none());
    }

    #[test]
    fn detect_depot_paths() {
        let strings = vec![
            "U:/P4Damascus/Main/Engine/Source/".to_string(),
            "X:/Jenkins/sharedspace/Build/".to_string(),
        ];
        let depot = detect_depot(&strings);
        assert!(depot.is_some());
        let depot = depot.unwrap();
        assert_eq!(depot.evidence.len(), 2);
    }

    #[test]
    fn detect_depot_backslash() {
        let strings = vec!["U:\\P4Damascus\\Main\\Engine".to_string()];
        let depot = detect_depot(&strings);
        assert!(depot.is_some());
        let depot = depot.unwrap();
        assert_eq!(depot.value, "P4Damascus");
    }

    #[test]
    fn detect_project_paths_works() {
        let strings = vec![
            "X:/Jenkins/sharedspace/HK_Project_Delivery/Build/".to_string(),
            "X:/Jenkins/sharedspace/HK_EngineSources/".to_string(),
        ];
        let projects = detect_project_paths(&strings);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].value, "HK_Project_Delivery");
        assert_eq!(projects[0].evidence.len(), 2);
    }

    #[test]
    fn detect_third_party_physx() {
        let strings = vec![
            "PhysX 3.4".to_string(),
            "PhysXCooking".to_string(),
            "Failed to create the PhysX shape.".to_string(),
        ];
        let tp = detect_third_party(&strings);
        assert!(!tp.is_empty());
        assert!(tp.iter().any(|d| d.value == "PhysX"));
    }

    #[test]
    fn detect_third_party_libpng() {
        let strings = vec!["libpng version 1.5.2".to_string()];
        let tp = detect_third_party(&strings);
        assert!(tp.iter().any(|d| d.value == "libpng"));
    }

    #[test]
    fn detect_sdk_hints_prospero() {
        let strings = vec!["Prospero SDK v10".to_string()];
        let sdk = detect_sdk_hints(&strings);
        assert!(!sdk.is_empty());
        assert!(sdk.iter().any(|d| d.value == "Prospero SDK"));
    }

    #[test]
    fn detect_versions_physx() {
        let strings = vec!["PhysX 3.4.1".to_string()];
        let versions = detect_versions(&strings);
        assert!(!versions.is_empty());
        assert!(versions.iter().any(|d| d.value == "PhysX"));
    }

    #[test]
    fn detect_source_paths_works() {
        let strings = vec![
            "U:/Engine/Source/Runtime/Core/Private/".to_string(),
            "Engine/Plugins/Online/".to_string(),
            "not a path".to_string(),
        ];
        let paths = detect_source_paths(&strings);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn analyze_strings_full() {
        let data = b"libScePad.prx\x00UnrealEngine4Runtime\x00U:/P4Damascus/Main/\x00PhysX 3.4\x00X:/Jenkins/sharedspace/\x00";
        let result = analyze_strings(data);
        assert!(!result.sce_libraries.is_empty());
        assert!(result.engine.is_some());
        assert!(result.build_system.is_some());
        assert!(result.source_depot.is_some());
        assert!(!result.third_party_libs.is_empty());
        assert!(!result.detected_versions.is_empty());
    }
}
