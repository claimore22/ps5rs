use serde::Serialize;

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

pub const CATALOG: &[(&str, &MiddlewareId)] = &[
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
