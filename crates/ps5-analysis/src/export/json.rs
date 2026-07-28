use crate::model::*;
use std::io::Write;

pub fn export_analysis(db: &AnalysisDatabase, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(db).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_stats(stats: &AnalysisStats, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(stats).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_heatmap(heatmap: &LibraryHeatmap, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(heatmap).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_nid_frequency(freq: &NidFrequency, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(freq).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_unresolved(
    entries: &[UnresolvedEntry],
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(entries).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_graph(graph: &DependencyGraph, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(graph).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_validation(
    report: &crate::reports::ValidationReport,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_library_versions(
    report: &crate::reports::LibraryVersionReport,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_engine_hints(
    report: &crate::reports::EngineHintReport,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report).map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_db, make_game, make_import};

    fn test_db() -> AnalysisDatabase {
        make_db(vec![make_game(
            "GameA",
            vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "?", 1, "libA"),
            ],
        )])
    }

    #[test]
    fn json_export_analysis_valid() {
        let db = test_db();
        let mut buf = Vec::new();
        export_analysis(&db, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with('{'));
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["tool"], "ps5rs-test");
        assert_eq!(parsed["games"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn json_export_stats_valid() {
        use crate::reports::compute_stats;
        let db = test_db();
        let stats = compute_stats(&db);
        let mut buf = Vec::new();
        export_stats(&stats, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["total_games"], 1);
        assert_eq!(parsed["total_imports"], 2);
    }

    #[test]
    fn json_export_heatmap_valid() {
        use crate::reports::build_heatmap;
        let db = test_db();
        let heatmap = build_heatmap(&db);
        let mut buf = Vec::new();
        export_heatmap(&heatmap, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(!parsed["libraries"].as_array().unwrap().is_empty());
        assert!(!parsed["matrix"].as_array().unwrap().is_empty());
    }

    #[test]
    fn json_export_frequency_valid() {
        use crate::reports::build_frequency;
        let db = test_db();
        let freq = build_frequency(&db);
        let mut buf = Vec::new();
        export_nid_frequency(&freq, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["total_imports"], 2);
        assert!(!parsed["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn json_export_unresolved_valid() {
        use crate::reports::find_unresolved;
        let db = test_db();
        let entries = find_unresolved(&db);
        let mut buf = Vec::new();
        export_unresolved(&entries, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 1);
        assert_eq!(parsed[0]["nid_hash"], "bbb");
    }

    #[test]
    fn json_export_graph_valid() {
        use crate::reports::build_graph;
        let db = test_db();
        let graph = build_graph(&db, false);
        let mut buf = Vec::new();
        export_graph(&graph, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["game_nodes"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["lib_nodes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn json_export_ends_with_newline() {
        let db = test_db();
        let mut buf = Vec::new();
        export_analysis(&db, &mut buf).unwrap();
        assert_eq!(*buf.last().unwrap(), b'\n');
    }
}
