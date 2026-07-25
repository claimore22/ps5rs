use crate::model::*;

pub fn compute_stats(db: &AnalysisDatabase) -> AnalysisStats {
    let total_games = db.games.len();
    let total_imports: usize = db.games.iter().map(|g| g.imports.len()).sum();
    let unique_nids: std::collections::HashSet<&str> = db.games.iter()
        .flat_map(|g| g.imports.iter().map(|i| i.nid_hash.as_str()))
        .collect();
    let unique_libs: std::collections::HashSet<&str> = db.games.iter()
        .flat_map(|g| g.imports.iter().map(|i| i.library_name.as_str()))
        .collect();

    let resolved = db.games.iter()
        .flat_map(|g| g.imports.iter())
        .filter(|i| i.resolved_name != "?")
        .count();

    let resolution_rate = if total_imports > 0 {
        resolved as f64 / total_imports as f64 * 100.0
    } else {
        0.0
    };

    // Most common NID
    let mut nid_counts: std::collections::HashMap<&str, (usize, &str)> = std::collections::HashMap::new();
    for game in &db.games {
        for imp in &game.imports {
            let entry = nid_counts.entry(&imp.nid_hash).or_insert((0, &imp.resolved_name));
            entry.0 += 1;
        }
    }
    let (most_nid_hash, most_nid_name, most_nid_count) = nid_counts.iter()
        .max_by_key(|(_, (count, _))| count)
        .map(|(hash, (count, name))| (hash.to_string(), name.to_string(), *count))
        .unwrap_or_default();

    // Most used lib
    let mut lib_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for game in &db.games {
        for imp in &game.imports {
            *lib_counts.entry(&imp.library_name).or_insert(0) += 1;
        }
    }
    let (most_lib_name, most_lib_count) = lib_counts.iter()
        .max_by_key(|(_, count)| **count)
        .map(|(name, count)| (name.to_string(), *count))
        .unwrap_or_default();

    AnalysisStats {
        total_games,
        total_imports,
        unique_nids: unique_nids.len(),
        unique_libs: unique_libs.len(),
        resolution_rate,
        most_common_nid: Some(most_nid_hash),
        most_common_nid_name: Some(most_nid_name),
        most_common_nid_count: most_nid_count,
        most_used_lib: Some(most_lib_name),
        most_used_lib_count: most_lib_count,
    }
}
