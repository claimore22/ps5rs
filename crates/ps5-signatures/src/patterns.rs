use ps5_image::Detection;

pub fn detect_sdk_hints(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();
    detect_pattern(
        strings,
        &["Prospero SDK", "prospero"],
        "Platform SDK",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["ORBIS SDK", "orbis"],
        "Platform SDK",
        &mut detections,
    );
    detect_pattern(strings, &["SCE SDK", "sce_"], "System SDK", &mut detections);
    detections
}

pub fn detect_third_party(strings: &[String]) -> Vec<Detection> {
    let mut detections = Vec::new();
    detect_pattern(
        strings,
        &["PhysX", "PX_", "PhysXScene"],
        "PhysX",
        &mut detections,
    );
    detect_pattern(
        strings,
        &["libVorbis", "Xiph.Org libVorbis"],
        "libVorbis",
        &mut detections,
    );
    detect_pattern(strings, &["FMOD", "Firelight"], "FMOD", &mut detections);
    detect_pattern(
        strings,
        &["Wwise", "Audiokinetic"],
        "Wwise",
        &mut detections,
    );
    detections
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
