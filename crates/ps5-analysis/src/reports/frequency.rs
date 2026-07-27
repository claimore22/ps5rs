use crate::model::*;
use std::collections::HashMap;

pub fn build_frequency(db: &AnalysisDatabase) -> NidFrequency {
    let mut nid_info: HashMap<String, (String, usize, Vec<String>)> = HashMap::new();

    for game in &db.games {
        let gname = game.display_name.as_deref().unwrap_or(&game.name).to_string();
        for imp in &game.imports {
            let entry = nid_info.entry(imp.nid_hash.clone())
                .or_insert_with(|| (imp.resolved_name.clone(), 0, Vec::new()));
            entry.1 += 1;
            if !entry.2.contains(&gname) {
                entry.2.push(gname.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_game, make_db, make_import};

    #[test]
    fn frequency_empty() {
        let db = make_db(vec![]);
        let f = build_frequency(&db);
        assert!(f.entries.is_empty());
        assert_eq!(f.total_imports, 0);
        assert_eq!(f.unique_nids, 0);
    }

    #[test]
    fn frequency_single_nid() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("aaa", "funcA", 1, "libA"),
                make_import("aaa", "funcA", 1, "libA"),
            ]),
        ]);
        let f = build_frequency(&db);
        assert_eq!(f.total_imports, 3);
        assert_eq!(f.unique_nids, 1);
        assert_eq!(f.entries.len(), 1);
        assert_eq!(f.entries[0].nid_hash, "aaa");
        assert_eq!(f.entries[0].name, "funcA");
        assert_eq!(f.entries[0].count, 3);
        assert_eq!(f.entries[0].games, vec!["GameA"]);
    }

    #[test]
    fn frequency_sorted_by_count_desc() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libA"),
                make_import("bbb", "fB", 1, "libA"),
                make_import("ccc", "fC", 1, "libA"),
            ]),
            make_game("GameB", vec![
                make_import("aaa", "fA", 1, "libA"),
            ]),
        ]);
        let f = build_frequency(&db);
        assert_eq!(f.entries[0].nid_hash, "aaa");
        assert_eq!(f.entries[0].count, 2);
        assert_eq!(f.entries[1].count, 1);
        assert_eq!(f.entries[2].count, 1);
    }

    #[test]
    fn frequency_games_listed() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libA"),
            ]),
            make_game("GameB", vec![
                make_import("aaa", "fA", 1, "libA"),
            ]),
        ]);
        let f = build_frequency(&db);
        let entry = f.entries.iter().find(|e| e.nid_hash == "aaa").unwrap();
        assert!(entry.games.contains(&"GameA".to_string()));
        assert!(entry.games.contains(&"GameB".to_string()));
        assert_eq!(entry.games.len(), 2);
    }

    #[test]
    fn frequency_no_duplicates_in_games_list() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "fA", 1, "libA"),
                make_import("aaa", "fA", 1, "libA"),
            ]),
        ]);
        let f = build_frequency(&db);
        let entry = f.entries.iter().find(|e| e.nid_hash == "aaa").unwrap();
        assert_eq!(entry.games, vec!["GameA"]);
        assert_eq!(entry.count, 2);
    }
}
