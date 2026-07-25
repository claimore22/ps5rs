use crate::model::*;
use std::collections::HashMap;

pub fn build_frequency(db: &AnalysisDatabase) -> NidFrequency {
    let mut nid_info: HashMap<String, (String, usize, Vec<String>)> = HashMap::new();

    for game in &db.games {
        for imp in &game.imports {
            let entry = nid_info.entry(imp.nid_hash.clone())
                .or_insert_with(|| (imp.resolved_name.clone(), 0, Vec::new()));
            entry.1 += 1;
            if !entry.2.contains(&game.name) {
                entry.2.push(game.name.clone());
            }
        }
    }

    let total_imports: usize = db.games.iter().map(|g| g.imports.len()).sum();
    let unique_nids = nid_info.len();

    let mut entries: Vec<NidFrequencyEntry> = nid_info.into_iter()
        .map(|(hash, (name, count, games))| NidFrequencyEntry {
            nid_hash: hash,
            name,
            count,
            games,
        })
        .collect();

    entries.sort_by_key(|b| std::cmp::Reverse(b.count));

    NidFrequency { entries, total_imports, unique_nids }
}
