use crate::model::*;
use std::io::Write;

pub fn export_analysis(db: &AnalysisDatabase, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(db)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_stats(stats: &AnalysisStats, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(stats)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_heatmap(heatmap: &LibraryHeatmap, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(heatmap)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_nid_frequency(freq: &NidFrequency, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(freq)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_unresolved(entries: &[UnresolvedEntry], writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(entries)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn export_graph(graph: &DependencyGraph, writer: &mut dyn Write) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(graph)
        .map_err(std::io::Error::other)?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}
