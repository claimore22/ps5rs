use crate::model::*;
use std::collections::HashMap;

pub fn build_heatmap(db: &AnalysisDatabase) -> LibraryHeatmap {
    let mut lib_game_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut all_games: Vec<String> = Vec::new();
    let mut seen_games: std::collections::HashSet<String> = std::collections::HashSet::new();

    for game in &db.games {
        if !seen_games.contains(&game.name) {
            all_games.push(game.name.clone());
            seen_games.insert(game.name.clone());
        }
        for imp in &game.imports {
            lib_game_counts
                .entry(imp.library_name.clone())
                .or_default()
                .entry(game.name.clone())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
    }

    let mut lib_names: Vec<String> = lib_game_counts.keys().cloned().collect();
    lib_names.sort();

    let mut matrix = Vec::with_capacity(lib_names.len());
    for lib in &lib_names {
        let row: Vec<usize> = all_games.iter()
            .map(|game| lib_game_counts[lib].get(game).copied().unwrap_or(0))
            .collect();
        matrix.push(row);
    }

    LibraryHeatmap {
        libraries: lib_names,
        games: all_games,
        matrix,
    }
}
