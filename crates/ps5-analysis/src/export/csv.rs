use crate::model::*;
use std::io::Write;

pub fn export_analysis(db: &AnalysisDatabase, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "game,platform,file_size,imports,relocations,symbols,has_tls")?;
    for game in &db.games {
        writeln!(writer, "{},{},{},{},{},{},{}",
            game.name,
            game.platform,
            game.file_size,
            game.imports.len(),
            game.num_relocations,
            game.num_symbols,
            game.has_tls,
        )?;
    }
    Ok(())
}

pub fn export_heatmap(heatmap: &LibraryHeatmap, writer: &mut dyn Write) -> std::io::Result<()> {
    write!(writer, "Library")?;
    for game in &heatmap.games {
        write!(writer, ",{}", game)?;
    }
    writeln!(writer)?;

    for (i, lib) in heatmap.libraries.iter().enumerate() {
        write!(writer, "{}", lib)?;
        for count in &heatmap.matrix[i] {
            write!(writer, ",{}", count)?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

pub fn export_nid_frequency(freq: &NidFrequency, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "nid,name,count,games")?;
    for entry in &freq.entries {
        let game_list = entry.games.join(";");
        writeln!(writer, "{},{},{},{}",
            entry.nid_hash,
            entry.name,
            entry.count,
            game_list,
        )?;
    }
    Ok(())
}

pub fn export_unresolved(entries: &[UnresolvedEntry], writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "game,library,nid")?;
    for entry in entries {
        writeln!(writer, "{},{},{}",
            entry.game,
            entry.library,
            entry.nid_hash,
        )?;
    }
    Ok(())
}

pub fn export_imports(db: &AnalysisDatabase, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "game,library,nid,name")?;
    for game in &db.games {
        for imp in &game.imports {
            writeln!(writer, "{},{},{},{}",
                game.name,
                imp.library_name,
                imp.nid_hash,
                imp.resolved_name,
            )?;
        }
    }
    Ok(())
}

pub fn export_library_versions(report: &crate::reports::LibraryVersionReport, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "library,version_raw,version_string,game_count,games")?;
    for entry in &report.entries {
        writeln!(
            writer,
            "{},{},{},{},{}",
            entry.library,
            entry.version_raw,
            entry.version_string,
            entry.game_count,
            entry.games.join(";"),
        )?;
    }
    Ok(())
}

pub fn export_unknown_nids(report: &crate::reports::unknown_nids::UnknownNidReport, writer: &mut dyn Write) -> std::io::Result<()> {
    writeln!(writer, "library,nid,count,games")?;
    for entry in &report.entries {
        for lib in &entry.libraries {
            writeln!(
                writer,
                "{},{},{},{}",
                lib,
                entry.nid_hash,
                entry.count,
                entry.games.join(";"),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_game, make_db, make_import};

    fn test_db() -> AnalysisDatabase {
        make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "funcB", 2, "libB"),
            ]),
            make_game("GameB", vec![
                make_import("aaa", "funcA", 1, "libA"),
            ]),
        ])
    }

    #[test]
    fn csv_analysis_header_and_rows() {
        let db = test_db();
        let mut buf = Vec::new();
        export_analysis(&db, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[0], "game,platform,file_size,imports,relocations,symbols,has_tls");
        assert_eq!(lines.len(), 3); // header + 2 games
        assert!(lines[1].starts_with("GameA,PS5,"));
    }

    #[test]
    fn csv_heatmap_header_and_rows() {
        use crate::reports::build_heatmap;
        let db = test_db();
        let heatmap = build_heatmap(&db);
        let mut buf = Vec::new();
        export_heatmap(&heatmap, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        // Header: Library,GameA,GameB
        assert!(lines[0].starts_with("Library,"));
        assert!(lines[0].contains("GameA"));
        assert!(lines[0].contains("GameB"));
        assert!(lines.len() > 1);
    }

    #[test]
    fn csv_frequency_header_and_rows() {
        use crate::reports::build_frequency;
        let db = test_db();
        let freq = build_frequency(&db);
        let mut buf = Vec::new();
        export_nid_frequency(&freq, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[0], "nid,name,count,games");
        assert_eq!(lines.len(), 3); // header + 2 NIDs
    }

    #[test]
    fn csv_unresolved_header_and_rows() {
        use crate::reports::find_unresolved;
        let mut db = test_db();
        // Add an unresolved import so find_unresolved has something to find
        db.games[0].imports.push(crate::model::make_import("zzz", "?", 3, "libC"));
        let entries = find_unresolved(&db);
        let mut buf = Vec::new();
        export_unresolved(&entries, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[0], "game,library,nid");
        assert!(lines.len() > 1);
    }

    #[test]
    fn csv_imports_header_and_rows() {
        let db = test_db();
        let mut buf = Vec::new();
        export_imports(&db, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[0], "game,library,nid,name");
        assert_eq!(lines.len(), 4); // header + 3 imports
    }

    #[test]
    fn csv_imports_correct_content() {
        let db = test_db();
        let mut buf = Vec::new();
        export_imports(&db, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("GameA,libA,aaa,funcA"));
        assert!(s.contains("GameA,libB,bbb,funcB"));
        assert!(s.contains("GameB,libA,aaa,funcA"));
    }
}
