use crate::model::*;

pub fn compute_stats(db: &AnalysisDatabase) -> AnalysisStats {
    let total_games = db.games.len();
    let total_imports: usize = db.games.iter().map(|g| g.imports.len()).sum();
    let unique_nids: std::collections::HashSet<&str> = db
        .games
        .iter()
        .flat_map(|g| g.imports.iter().map(|i| i.nid_hash.as_str()))
        .collect();
    let unique_libs: std::collections::HashSet<&str> = db
        .games
        .iter()
        .flat_map(|g| g.imports.iter().map(|i| i.library_name.as_str()))
        .collect();

    let resolved = db
        .games
        .iter()
        .flat_map(|g| g.imports.iter())
        .filter(|i| i.resolved_name != "?")
        .count();

    let resolution_rate = if total_imports > 0 {
        resolved as f64 / total_imports as f64 * 100.0
    } else {
        0.0
    };

    // Most common NID
    let mut nid_counts: std::collections::HashMap<&str, (usize, &str)> =
        std::collections::HashMap::new();
    for game in &db.games {
        for imp in &game.imports {
            let entry = nid_counts
                .entry(&imp.nid_hash)
                .or_insert((0, &imp.resolved_name));
            entry.0 += 1;
        }
    }
    let (most_nid_hash, most_nid_name, most_nid_count) = nid_counts
        .iter()
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
    let (most_lib_name, most_lib_count) = lib_counts
        .iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_db, make_game, make_import};

    #[test]
    fn stats_empty_database() {
        let db = make_db(vec![]);
        let s = compute_stats(&db);
        assert_eq!(s.total_games, 0);
        assert_eq!(s.total_imports, 0);
        assert_eq!(s.unique_nids, 0);
        assert_eq!(s.unique_libs, 0);
        assert_eq!(s.resolution_rate, 0.0);
    }

    #[test]
    fn stats_single_game() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "funcB", 2, "libB"),
                make_import("aaa", "funcA", 1, "libA"),
            ],
        )]);
        let s = compute_stats(&db);
        assert_eq!(s.total_games, 1);
        assert_eq!(s.total_imports, 3);
        assert_eq!(s.unique_nids, 2); // aaa, bbb
        assert_eq!(s.unique_libs, 2); // libA, libB
        assert_eq!(s.most_common_nid.as_deref(), Some("aaa"));
        assert_eq!(s.most_common_nid_count, 2);
        assert_eq!(s.most_used_lib.as_deref(), Some("libA"));
        assert_eq!(s.most_used_lib_count, 2);
    }

    #[test]
    fn stats_resolution_rate() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "?", 1, "libA"),
                make_import("ccc", "?", 1, "libA"),
            ],
        )]);
        let s = compute_stats(&db);
        // 1 resolved out of 3 = 33.33%
        assert!((s.resolution_rate - 33.33).abs() < 0.1);
    }

    #[test]
    fn stats_all_unresolved() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "?", 1, "libA")],
        )]);
        let s = compute_stats(&db);
        assert_eq!(s.resolution_rate, 0.0);
    }

    #[test]
    fn stats_multi_game_tiebreaker() {
        let db = make_db(vec![
            make_game("GameA", vec![make_import("aaa", "funcA", 1, "libA")]),
            make_game(
                "GameB",
                vec![
                    make_import("aaa", "funcA", 1, "libA"),
                    make_import("ccc", "funcC", 2, "libB"),
                ],
            ),
        ]);
        let s = compute_stats(&db);
        assert_eq!(s.total_imports, 3);
        assert_eq!(s.unique_nids, 2);
        // 'aaa' appears in both games = 2 occurrences
        assert_eq!(s.most_common_nid.as_deref(), Some("aaa"));
        assert_eq!(s.most_common_nid_count, 2);
        assert_eq!(s.most_used_lib.as_deref(), Some("libA"));
        assert_eq!(s.most_used_lib_count, 2);
    }

    #[test]
    fn stats_empty_database_fields_are_none() {
        let db = make_db(vec![]);
        let s = compute_stats(&db);
        assert!(s.most_common_nid.is_none() || s.most_common_nid.as_deref() == Some(""));
        assert!(s.most_used_lib.is_none() || s.most_used_lib.as_deref() == Some(""));
    }
}
