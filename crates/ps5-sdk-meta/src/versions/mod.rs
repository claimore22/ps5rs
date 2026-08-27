use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionRange {
    pub from: String,
    pub to: Option<String>,
}

impl VersionRange {
    pub fn new(from: impl Into<String>, to: Option<impl Into<String>>) -> Self {
        Self {
            from: from.into(),
            to: to.map(|s| s.into()),
        }
    }

    fn parse_ver(s: &str) -> (u32, u32) {
        let mut parts = s.split('.');
        let major = parts.next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        (major, minor)
    }

    pub fn contains(&self, ver: &str) -> bool {
        let v = Self::parse_ver(ver);
        let from = Self::parse_ver(&self.from);
        if v < from {
            return false;
        }
        if let Some(to) = &self.to {
            let to_v = Self::parse_ver(to);
            if v > to_v {
                return false;
            }
        }
        true
    }

    pub fn single(from: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_basic() {
        let r = VersionRange::new("9.00", Some("11.00"));
        assert!(r.contains("10.00"));
        assert!(!r.contains("8.99"));
        assert!(!r.contains("11.01"));
    }

    #[test]
    fn single_open_end() {
        let r = VersionRange::single("10.00");
        assert!(r.contains("10.00"));
        assert!(r.contains("99.00"));
        assert!(!r.contains("9.99"));
    }
}
