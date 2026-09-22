//! Display-name hygiene for derived artifacts.
//!
//! Dump directories often carry scene release tags (`[SuperPSX]`, bare
//! `SuperPSX`, …) that must never leak into dataset records, filenames, or
//! the dashboard. These helpers scrub them in one place so every record
//! point (scan, inventory, middleware, loader reports, NID reports) agrees.
//! Filesystem paths are additionally recorded relative to the scanned corpus
//! root so shared datasets stay machine-independent.

use std::path::Path;

fn is_scene_bracket(inner: &str) -> bool {
    inner.to_ascii_lowercase().contains("superpsx")
}

/// Remove scene release tags from a display name.
///
/// Drops `[...]` groups mentioning SuperPSX (case-insensitive) and bare
/// `SuperPSX` tokens, then collapses leftover separator runs. A `[PPSA12345]`
/// title ID is never a scene tag and is always preserved.
pub fn scrub_scene_tags(name: &str) -> String {
    // Drop `[...]` groups mentioning SuperPSX (byte scan is safe: both
    // brackets are ASCII, inner slice boundaries stay on char edges).
    let mut out = String::with_capacity(name.len());
    let mut i = 0;
    while i < name.len() {
        if name.as_bytes()[i] == b'['
            && let Some(rel) = name[i..].find(']')
        {
            let end = i + rel;
            let inner = &name[i + 1..end];
            if is_scene_bracket(inner) {
                i = end + 1;
                continue;
            }
        }
        let ch = name[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    // Drop bare tokens (e.g. "Game SuperPSX Edition").
    let mut words: Vec<&str> = Vec::new();
    for word in out.split_whitespace() {
        let trimmed = word.trim_matches(|c| c == '_' || c == '-' || c == '.');
        if trimmed.eq_ignore_ascii_case("superpsx") {
            continue;
        }
        words.push(word);
    }
    let mut cleaned = words.join(" ");
    // Collapse runs left behind by removals.
    loop {
        let next = cleaned
            .replace("  ", " ")
            .replace("__", "_")
            .replace("--", "-")
            .replace(" -", " ")
            .replace("- ", " ")
            .replace("/-", "/")
            .replace("-/", "/");
        if next.len() == cleaned.len() {
            break;
        }
        cleaned = next;
    }
    cleaned
        .trim()
        .trim_matches(|c| c == '_' || c == '-' || c == '/' || c == '\\')
        .to_string()
}

/// Render `path` for derived artifacts: relative to the scanned corpus
/// `root` (with `/` separators) when possible, scene tags scrubbed.
/// Falls back to the scrubbed file name when `path` is not under `root`.
pub fn relative_display_path(path: &Path, root: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) if !rel.as_os_str().is_empty() => {
            scrub_scene_tags(&rel.to_string_lossy().replace('\\', "/"))
        }
        _ => path
            .file_name()
            .map(|n| scrub_scene_tags(&n.to_string_lossy()))
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_bracketed_superpsx_prefix() {
        assert_eq!(
            scrub_scene_tags("[SuperPSX]-Animal.Well-PPSA22520-JPN-Game (v01.000.011)-PS5"),
            "Animal.Well-PPSA22520-JPN-Game (v01.000.011)-PS5"
        );
    }

    #[test]
    fn strips_bare_superpsx_token() {
        assert_eq!(
            scrub_scene_tags("Cool Game SuperPSX Edition"),
            "Cool Game Edition"
        );
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(scrub_scene_tags("[superpsx] Game"), "Game");
    }

    #[test]
    fn preserves_ppsa_brackets_and_versions() {
        assert_eq!(
            scrub_scene_tags("Gori.Cuddly.Carnage-PPSA07393-USA-Game-(v01.000)-PS5"),
            "Gori.Cuddly.Carnage-PPSA07393-USA-Game-(v01.000)-PS5"
        );
        assert_eq!(
            scrub_scene_tags("Dolphin Spirit Ocean Mission [PPSA13711]"),
            "Dolphin Spirit Ocean Mission [PPSA13711]"
        );
    }

    #[test]
    fn leaves_clean_names_untouched() {
        assert_eq!(
            scrub_scene_tags("Stray-PPSA02100-USA-PS5"),
            "Stray-PPSA02100-USA-PS5"
        );
        assert_eq!(scrub_scene_tags("GRIS"), "GRIS");
    }

    #[test]
    fn keeps_unicode_intact() {
        assert_eq!(
            scrub_scene_tags("[SuperPSX]-Persona.3.Reload-PPSA10872 – USA-Game-v01.000-PS5"),
            "Persona.3.Reload-PPSA10872 – USA-Game-v01.000-PS5"
        );
    }

    #[test]
    fn relative_display_path_under_root() {
        let root = Path::new("/roms/PS5");
        let nested =
            Path::new("/roms/PS5/Animal Well PS5 _PPSA22520_/[SuperPSX]-Animal.Well/eboot.bin");
        assert_eq!(
            relative_display_path(nested, root),
            "Animal Well PS5 _PPSA22520_/Animal.Well/eboot.bin"
        );
    }

    #[test]
    fn relative_display_path_outside_root_falls_back_to_name() {
        let root = Path::new("/roms/PS5");
        assert_eq!(
            relative_display_path(Path::new("/other/[SuperPSX]-Game"), root),
            "Game"
        );
    }
}
