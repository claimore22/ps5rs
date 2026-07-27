use crate::model::*;
use std::collections::HashMap;

pub fn build_heatmap(db: &AnalysisDatabase) -> LibraryHeatmap {
    let mut lib_game_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut all_games: Vec<String> = Vec::new();
    let mut seen_games: std::collections::HashSet<String> = std::collections::HashSet::new();

    for game in &db.games {
        let gname = game.display_name.as_deref().unwrap_or(&game.name).to_string();
        if !seen_games.contains(&gname) {
            all_games.push(gname.clone());
            seen_games.insert(gname.clone());
        }
        for imp in &game.imports {
            lib_game_counts
                .entry(imp.library_name.clone())
                .or_default()
                .entry(gname.clone())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_game, make_db, make_import};

    #[test]
    fn heatmap_empty() {
        let db = make_db(vec![]);
        let h = build_heatmap(&db);
        assert!(h.libraries.is_empty());
        assert!(h.games.is_empty());
        assert!(h.matrix.is_empty());
    }

    #[test]
    fn heatmap_single_game_single_lib() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libA"),
                make_import("bbb", "fB", 1, "libA"),
            ]),
        ]);
        let h = build_heatmap(&db);
        assert_eq!(h.games, vec!["GameA"]);
        assert_eq!(h.libraries, vec!["libA"]);
        assert_eq!(h.matrix, vec![vec![2]]);
    }

    #[test]
    fn heatmap_multi_game_multi_lib() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libA"),
                make_import("bbb", "fB", 2, "libB"),
            ]),
            make_game("GameB", vec![
                make_import("aaa", "fA", 1, "libA"),
                make_import("ccc", "fC", 1, "libA"),
            ]),
        ]);
        let h = build_heatmap(&db);
        assert_eq!(h.games, vec!["GameA", "GameB"]);
        assert_eq!(h.libraries, vec!["libA", "libB"]);
        // libA: GameA=1, GameB=2
        // libB: GameA=1, GameB=0
        assert_eq!(h.matrix, vec![vec![1, 2], vec![1, 0]]);
    }

    #[test]
    fn heatmap_game_not_in_lib() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libX"),
            ]),
            make_game("GameB", vec![
                make_import("bbb", "fB", 2, "libY"),
            ]),
        ]);
        let h = build_heatmap(&db);
        assert_eq!(h.games, vec!["GameA", "GameB"]);
        assert_eq!(h.libraries, vec!["libX", "libY"]);
        assert_eq!(h.matrix, vec![vec![1, 0], vec![0, 1]]);
    }

    #[test]
    fn heatmap_libraries_sorted() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("a", "f", 3, "libC"),
                make_import("b", "f", 1, "libA"),
                make_import("c", "f", 2, "libB"),
            ]),
        ]);
        let h = build_heatmap(&db);
        assert_eq!(h.libraries, vec!["libA", "libB", "libC"]);
    }
}
