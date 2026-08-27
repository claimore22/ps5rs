use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct FirmwareVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl FirmwareVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(Self { major, minor, patch })
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.patch == 0 {
            write!(f, "{}.{:02}", self.major, self.minor)
        } else {
            write!(f, "{}.{:02}.{}", self.major, self.minor, self.patch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_display() {
        let v = FirmwareVersion::parse("10.01").unwrap();
        assert_eq!(v.major, 10);
        assert_eq!(v.minor, 1);
        assert_eq!(format!("{}", v), "10.01");
    }

    #[test]
    fn ordering() {
        let v9 = FirmwareVersion::new(9, 0, 0);
        let v10 = FirmwareVersion::new(10, 0, 0);
        assert!(v9 < v10);
    }
}
