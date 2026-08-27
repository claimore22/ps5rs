use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EvidenceKind {
    StringMatch,
    Symbol,
    Nid,
    Library,
    ModuleName,
    SourcePath,
    ExportTable,
    CatalogSource,
    SdkMetadata,
    FirmwareMetadata,
    ShaderMetadata,
    BuildId,
    FileLocation,
    CrossReference,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl Evidence {
    pub fn new(kind: EvidenceKind, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
            source: None,
            details: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Detection {
    pub value: String,
    #[serde(default)]
    pub confidence: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

impl Detection {
    pub fn new(value: impl Into<String>, confidence: u8, evidence: Vec<Evidence>) -> Self {
        Self {
            value: value.into(),
            confidence,
            evidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_serializes() {
        let ev = Evidence::new(EvidenceKind::StringMatch, "Engine/Source/Runtime")
            .with_source("strings")
            .with_details("found in .rodata");
        let json = serde_json::to_string(&ev).unwrap();
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, "Engine/Source/Runtime");
        assert_eq!(back.kind, EvidenceKind::StringMatch);
    }

    #[test]
    fn detection_serializes() {
        let d = Detection::new(
            "Unreal Engine 5",
            89,
            vec![Evidence::new(EvidenceKind::StringMatch, "FName")],
        );
        let json = serde_json::to_string(&d).unwrap();
        let back: Detection = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, "Unreal Engine 5");
        assert_eq!(back.confidence, 89);
        assert_eq!(back.evidence.len(), 1);
    }
}
