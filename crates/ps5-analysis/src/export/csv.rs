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
