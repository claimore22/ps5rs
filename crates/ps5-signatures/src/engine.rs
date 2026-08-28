use ps5_image::Detection;

pub struct EngineFingerprint {
    pub name: &'static str,
    pub patterns: &'static [(&'static str, u8)],
}

impl EngineFingerprint {
    pub fn score(&self, strings: &[String]) -> (u32, u8, Vec<String>) {
        let mut evidence = Vec::new();
        let mut total: u32 = 0;

        for &(pattern, weight) in self.patterns {
            for s in strings {
                if s.contains(pattern) {
                    evidence.push(s.clone());
                    total += weight as u32;
                    break;
                }
            }
        }

        let confidence = if evidence.is_empty() {
            0
        } else {
            total.min(100) as u8
        };

        (total, confidence, evidence)
    }
}

pub const UNREAL4: EngineFingerprint = EngineFingerprint {
    name: "Unreal Engine 4",
    patterns: &[
        ("UnrealEngine4Runtime", 100),
        ("UnrealEngine4Editor", 100),
        ("P4Damascus", 95),
        ("UObject", 80),
        ("FName", 70),
        ("GEngine", 70),
        ("GWorld", 70),
        ("FPlatformProcess", 60),
        ("UE4Runtime", 50),
        ("UE4Game", 50),
        ("FShaderPipelineCache", 8),
        ("SlateRHIRenderer", 5),
        ("QuickHullConvexHullLib", 3),
        ("Engine/Source/Runtime", 3),
        ("Engine/Plugins", 3),
        ("PhysXCooking", 2),
    ],
};

pub const UNREAL5: EngineFingerprint = EngineFingerprint {
    name: "Unreal Engine 5",
    patterns: &[
        ("UnrealEngine5Runtime", 100),
        ("UnrealEngine5Editor", 100),
        ("Nanite", 10),
        ("Lumen", 10),
        ("UE5Runtime", 50),
        ("UE5Game", 50),
    ],
};

pub const UNITY: EngineFingerprint = EngineFingerprint {
    name: "Unity",
    patterns: &[
        ("UnityEngine", 90),
        ("UnityPlayer", 5),
        ("UnityMain", 2),
        ("global-metadata.dat", 5),
        ("il2cpp", 5),
        ("Assembly-CSharp", 3),
    ],
};

pub const GODOT: EngineFingerprint = EngineFingerprint {
    name: "Godot",
    patterns: &[("Godot Engine", 10), ("GDNative", 8)],
};

pub const RE_ENGINE: EngineFingerprint = EngineFingerprint {
    name: "RE Engine",
    patterns: &[
        ("RE Engine", 90),
        ("re_chunk", 80),
        ("REFramework", 70),
        ("via.motion", 40),
        ("via.render", 40),
        ("RE2_Prop", 30),
    ],
};

pub const DECIMA: EngineFingerprint = EngineFingerprint {
    name: "Decima",
    patterns: &[
        ("Decima", 90),
        ("DecimaEngine", 90),
        (".core", 30),
        ("HorizonZeroDawn", 20),
        ("DeathStranding", 20),
    ],
};

pub const DRAGON_ENGINE: EngineFingerprint = EngineFingerprint {
    name: "Dragon Engine",
    patterns: &[
        ("DragonEngine", 90),
        ("RyuGaGotoku", 90),
        (".par", 40),
        (".pxd", 40),
        (".gmd", 30),
        (".dds", 10),
        ("Yakuza", 20),
        ("LikeADragon", 30),
    ],
};

pub const FROMSOFTWARE: EngineFingerprint = EngineFingerprint {
    name: "FromSoftware",
    patterns: &[
        ("FromSoftware", 90),
        (".dcx", 40),
        (".bnd", 40),
        ("FLVER", 50),
        ("SoulsFormats", 30),
        ("EldenRing", 30),
    ],
};

pub const INSOMNIAC: EngineFingerprint = EngineFingerprint {
    name: "Insomniac",
    patterns: &[
        ("Insomniac", 90),
        ("Spider-Man", 30),
        ("Ratchet", 20),
        (".i30", 30),
    ],
};

pub const NAUGHTY_DOG: EngineFingerprint = EngineFingerprint {
    name: "Naughty Dog",
    patterns: &[("Naughty Dog", 90), ("ndlib", 50), ("The Last of Us", 30)],
};

pub const SANTA_MONICA: EngineFingerprint = EngineFingerprint {
    name: "Santa Monica Studio",
    patterns: &[("Santa Monica", 90), ("God of War", 40), ("Jotnar", 30)],
};

pub const SUCKER_PUNCH: EngineFingerprint = EngineFingerprint {
    name: "Sucker Punch",
    patterns: &[
        ("Sucker Punch", 90),
        ("Ghost of Tsushima", 40),
        ("inFAMOUS", 30),
    ],
};

pub const POLYPHONY: EngineFingerprint = EngineFingerprint {
    name: "Polyphony Digital",
    patterns: &[("Polyphony", 90), ("Gran Turismo", 50), ("GT7", 30)],
};

pub const SQUARE_ENIX: EngineFingerprint = EngineFingerprint {
    name: "Square Enix",
    patterns: &[
        ("Square Enix", 90),
        ("Luminous", 50),
        ("Crystal Tools", 40),
        ("FFXVI", 30),
        (".pac", 20),
    ],
};

pub const UBISOFT: EngineFingerprint = EngineFingerprint {
    name: "Ubisoft",
    patterns: &[
        ("Ubisoft", 90),
        ("Anvil", 50),
        ("Snowdrop", 50),
        ("Dunia", 50),
        ("FarCry", 30),
    ],
};

pub const CRYENGINE: EngineFingerprint = EngineFingerprint {
    name: "CryEngine",
    patterns: &[("CryEngine", 90), ("CryTek", 80), ("Crysis", 30)],
};

pub const HEDGEHOG: EngineFingerprint = EngineFingerprint {
    name: "Hedgehog Engine",
    patterns: &[("Hedgehog", 90), ("Sonic", 30), ("HedgehogEngine", 90)],
};

pub const NATIVE: EngineFingerprint = EngineFingerprint {
    name: "Native",
    patterns: &[("eboot.bin", 5), ("prx", 5)],
};

pub const SCE: EngineFingerprint = EngineFingerprint {
    name: "SCE",
    patterns: &[("libSce", 30), ("Prospero", 30), ("Orbis", 30), ("SCE", 10)],
};

pub const ALL: &[EngineFingerprint] = &[
    UNREAL4,
    UNREAL5,
    UNITY,
    GODOT,
    RE_ENGINE,
    DECIMA,
    DRAGON_ENGINE,
    FROMSOFTWARE,
    INSOMNIAC,
    NAUGHTY_DOG,
    SANTA_MONICA,
    SUCKER_PUNCH,
    POLYPHONY,
    SQUARE_ENIX,
    UBISOFT,
    CRYENGINE,
    HEDGEHOG,
    NATIVE,
    SCE,
];

pub fn enhance_with_artifacts(
    mut detection: Detection,
    shader_count: usize,
    texture_count: usize,
    audio_count: usize,
) -> Detection {
    if shader_count > 0 || texture_count > 0 || audio_count > 0 {
        let evidence = format!(
            "bare artifacts: shader={} (pssl/sb), texture={} (gnf/dds), audio={} (at9/bank) — relative paths preserved, .pak not required",
            shader_count, texture_count, audio_count
        );
        detection.evidence.push(evidence);
        if shader_count > 100 {
            detection.confidence = detection.confidence.saturating_add(2).min(100);
            detection.score = detection.score.saturating_add(5);
        }
    }
    detection
}

pub fn detect_unreal_version(strings: &[String], base: &Detection) -> Option<(String, u8)> {
    if !base.value.contains("Unreal") {
        return None;
    }
    for s in strings {
        if s.contains("5.3") && s.contains("Unreal") {
            return Some(("Unreal Engine 5.3".to_string(), 85));
        }
        if s.contains("5.2") && s.contains("Unreal") {
            return Some(("Unreal Engine 5.2".to_string(), 85));
        }
        if s.contains("5.1") && s.contains("Unreal") {
            return Some(("Unreal Engine 5.1".to_string(), 85));
        }
        if s.contains("5.0") && s.contains("Unreal") {
            return Some(("Unreal Engine 5.0".to_string(), 80));
        }
        if s.contains("4.27") && s.contains("Unreal") {
            return Some(("Unreal Engine 4.27".to_string(), 80));
        }
    }
    None
}

pub fn detect_engine(strings: &[String]) -> Option<Detection> {
    let mut best: Option<Detection> = None;

    for fp in ALL {
        if fp.name == "Native" || fp.name == "SCE" {
            continue;
        }
        let (score, confidence, evidence) = fp.score(strings);
        if confidence == 0 {
            continue;
        }

        match &best {
            Some(current) if current.confidence > confidence => {}
            Some(current) if current.confidence == confidence => {
                best = Some(Detection {
                    value: fp.name.to_string(),
                    score,
                    confidence,
                    evidence,
                });
            }
            Some(_) => {
                best = Some(Detection {
                    value: fp.name.to_string(),
                    score,
                    confidence,
                    evidence,
                });
            }
            None => {
                best = Some(Detection {
                    value: fp.name.to_string(),
                    score,
                    confidence,
                    evidence,
                });
            }
        }
    }

    if best.is_some() {
        return best;
    }

    for fallback in [NATIVE, SCE] {
        let (score, confidence, evidence) = fallback.score(strings);
        if confidence == 0 {
            continue;
        }
        return Some(Detection {
            value: fallback.name.to_string(),
            score,
            confidence,
            evidence,
        });
    }

    None
}

pub fn classify_unknown(strings: &[String]) -> Detection {
    if let Some(d) = detect_engine(strings) {
        return d;
    }
    Detection {
        value: "Unknown".to_string(),
        score: 0,
        confidence: 0,
        evidence: vec![],
    }
}

pub fn detect_custom_forks(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();

    let fork_patterns: &[(&str, &str, u8)] = &[
        (
            "P4Damascus",
            "Unreal Engine 4 custom fork (P4Damascus depot)",
            90,
        ),
        (
            "HK_Project_Delivery",
            "Custom project build (HK_Project_Delivery)",
            85,
        ),
        (
            "HK_EngineSources",
            "Custom engine sources (HK_EngineSources)",
            85,
        ),
    ];

    for &(pattern, label, confidence) in fork_patterns {
        let mut evidence = Vec::new();
        for s in strings {
            if s.contains(pattern) {
                evidence.push(s.clone());
            }
        }
        if !evidence.is_empty() {
            evidence.truncate(10);
            detections.push(Detection {
                value: label.to_string(),
                score: confidence as u32,
                confidence,
                evidence,
            });
        }
    }

    detections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_unreal4_strong() {
        let strings = vec![
            "UnrealEngine4Runtime".to_string(),
            "FShaderPipelineCache".to_string(),
            "UObject".to_string(),
            "GWorld".to_string(),
            "FName".to_string(),
            "SlateRHIRenderer".to_string(),
        ];
        let (score, confidence, evidence) = UNREAL4.score(&strings);
        assert_eq!(confidence, 100);
        assert!(score >= 200);
        assert!(evidence.len() >= 5);
    }

    #[test]
    fn score_unreal4_weak() {
        let strings = vec!["Engine/Source/Runtime/Core/".to_string()];
        let (score, confidence, evidence) = UNREAL4.score(&strings);
        assert!(confidence > 0);
        assert!(score > 0);
        assert_eq!(evidence.len(), 1);
    }

    #[test]
    fn score_no_match() {
        let strings = vec!["hello world".to_string()];
        let (score, confidence, evidence) = UNREAL4.score(&strings);
        assert_eq!(confidence, 0);
        assert_eq!(score, 0);
        assert!(evidence.is_empty());
    }

    #[test]
    fn detect_engine_prefers_higher_score() {
        let strings = vec![
            "UnrealEngine4Runtime".to_string(),
            "FShaderPipelineCache".to_string(),
            "UObject".to_string(),
            "GWorld".to_string(),
        ];
        let result = detect_engine(&strings).unwrap();
        assert_eq!(result.value, "Unreal Engine 4");
        assert_eq!(result.confidence, 100);
    }

    #[test]
    fn detect_engine_unreal_and_unity_both_definitive() {
        let strings = vec![
            "UnrealEngine4Runtime".to_string(),
            "UnityEngine.dll".to_string(),
        ];
        let result = detect_engine(&strings).unwrap();
        // UE4 (100) > Unity (90)
        assert_eq!(result.value, "Unreal Engine 4");
        assert_eq!(result.confidence, 100);
    }

    #[test]
    fn detect_engine_ue5() {
        let strings = vec!["UnrealEngine5Runtime".to_string(), "Nanite".to_string()];
        let result = detect_engine(&strings).unwrap();
        assert_eq!(result.value, "Unreal Engine 5");
        assert_eq!(result.confidence, 100);
    }

    #[test]
    fn detect_engine_unity() {
        let strings = vec!["UnityEngine.dll".to_string(), "il2cpp".to_string()];
        let result = detect_engine(&strings).unwrap();
        assert_eq!(result.value, "Unity");
        assert!(result.confidence >= 95);
    }

    #[test]
    fn detect_engine_none() {
        let strings = vec!["hello world".to_string()];
        assert!(detect_engine(&strings).is_none());
        assert_eq!(classify_unknown(&strings).value, "Unknown");
    }

    #[test]
    fn detect_native_fallback() {
        let strings = vec!["eboot.bin".to_string(), "prx".to_string()];
        let d = classify_unknown(&strings);
        assert!(d.value == "Native" || d.value == "Unknown");
    }

    #[test]
    fn detect_sce_fallback() {
        let strings = vec!["libScePad".to_string(), "Prospero".to_string()];
        let d = detect_engine(&strings).unwrap();
        assert_eq!(d.value, "SCE");
    }

    #[test]
    fn detect_re_engine() {
        let strings = vec!["RE Engine".to_string(), "re_chunk_000.pak".to_string()];
        let d = detect_engine(&strings).unwrap();
        assert_eq!(d.value, "RE Engine");
    }

    #[test]
    fn detect_decima() {
        let strings = vec!["DecimaEngine".to_string(), "HorizonZeroDawn".to_string()];
        let d = detect_engine(&strings).unwrap();
        assert_eq!(d.value, "Decima");
    }

    #[test]
    fn detect_fromsoftware() {
        let strings = vec!["FromSoftware".to_string(), "FLVER".to_string()];
        let d = detect_engine(&strings).unwrap();
        assert_eq!(d.value, "FromSoftware");
    }

    #[test]
    fn detect_custom_forks_p4damascus() {
        let strings = vec![
            "U:/P4Damascus/Main/Engine/Source/".to_string(),
            "X:/Jenkins/sharedspace/P4Damascus/".to_string(),
        ];
        let forks = detect_custom_forks(&strings);
        assert!(!forks.is_empty());
        assert!(forks.iter().any(|d| d.value.contains("P4Damascus")));
        assert!(forks.iter().any(|d| d.confidence == 90));
    }

    #[test]
    fn detect_custom_forks_hk_project() {
        let strings = vec!["X:/Jenkins/sharedspace/HK_Project_Delivery/Build/".to_string()];
        let forks = detect_custom_forks(&strings);
        assert!(!forks.is_empty());
        assert!(
            forks
                .iter()
                .any(|d| d.value.contains("HK_Project_Delivery"))
        );
    }

    #[test]
    fn detect_custom_forks_empty() {
        let strings = vec!["hello world".to_string()];
        assert!(detect_custom_forks(&strings).is_empty());
    }

    #[test]
    fn enhance_with_artifacts_adds_evidence() {
        let strings = vec!["UnrealEngine4Runtime".to_string(), "UObject".to_string()];
        let det = detect_engine(&strings).unwrap();
        let enhanced = enhance_with_artifacts(det, 514, 4812, 36);
        assert!(
            enhanced
                .evidence
                .iter()
                .any(|e| e.contains("bare artifacts"))
        );
        assert!(enhanced.confidence >= 90);
    }

    #[test]
    fn enhance_does_not_invent_engine() {
        let det = Detection {
            value: "Unknown".to_string(),
            score: 0,
            confidence: 0,
            evidence: vec![],
        };
        let enhanced = enhance_with_artifacts(det, 500, 4000, 10);
        assert_eq!(enhanced.value, "Unknown");
    }

    #[test]
    fn unreal_version_only_with_evidence() {
        let strings = vec![
            "UnrealEngine5Runtime".to_string(),
            "SomeGame 5.3".to_string(),
        ];
        let base = detect_engine(&strings).unwrap();
        assert_eq!(base.value, "Unreal Engine 5");
        assert!(detect_unreal_version(&strings, &base).is_none());
        let strings2 = vec!["Unreal 5.3 patch".to_string()];
        let base2 = Detection {
            value: "Unreal Engine 5".to_string(),
            score: 100,
            confidence: 90,
            evidence: vec![],
        };
        let ver = detect_unreal_version(&strings2, &base2).unwrap();
        assert_eq!(ver.0, "Unreal Engine 5.3");
    }

    #[test]
    fn version_uncertainty() {
        let strings = vec!["UnrealEngine5Runtime".to_string()];
        let base = detect_engine(&strings).unwrap();
        assert_eq!(base.value, "Unreal Engine 5");
        assert!(base.confidence >= 80);
        assert!(detect_unreal_version(&strings, &base).is_none());
    }
}
