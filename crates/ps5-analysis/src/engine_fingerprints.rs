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

pub const ALL: &[EngineFingerprint] = &[UNREAL4, UNREAL5, UNITY, GODOT];

pub fn detect_engine(strings: &[String]) -> Option<Detection> {
    let mut best: Option<Detection> = None;

    for fp in ALL {
        let (score, confidence, evidence) = fp.score(strings);
        if confidence == 0 {
            continue;
        }

        match &best {
            Some(current) if current.confidence > confidence => {}
            Some(current) if current.confidence == confidence => {
                // tie-break: prefer the later entry (newer engine)
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

    best
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
}
