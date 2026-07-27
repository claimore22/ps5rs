use crate::model::*;

pub fn find_unresolved(db: &AnalysisDatabase) -> Vec<UnresolvedEntry> {
    let mut entries = Vec::new();
    for game in &db.games {
        let gname = game.display_name.as_deref().unwrap_or(&game.name).to_string();
        for imp in &game.imports {
            if imp.resolved_name == "?" {
                entries.push(UnresolvedEntry {
                    game: gname.clone(),
                    library: imp.library_name.clone(),
                    nid_hash: imp.nid_hash.clone(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.game.cmp(&b.game).then(a.library.cmp(&b.library)));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_game, make_db, make_import};

    #[test]
    fn unresolved_empty() {
        let db = make_db(vec![]);
        let u = find_unresolved(&db);
        assert!(u.is_empty());
    }

    #[test]
    fn unresolved_all_resolved() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "funcB", 2, "libB"),
            ]),
        ]);
        let u = find_unresolved(&db);
        assert!(u.is_empty());
    }

    #[test]
    fn unresolved_some() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "?", 1, "libA"),
            ]),
        ]);
        let u = find_unresolved(&db);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].game, "GameA");
        assert_eq!(u[0].library, "libA");
        assert_eq!(u[0].nid_hash, "bbb");
    }

    #[test]
    fn unresolved_sorted_by_game_then_library() {
        let db = make_db(vec![
            make_game("Zebra", vec![
                make_import("x", "?", 1, "libB"),
                make_import("y", "?", 2, "libA"),
            ]),
            make_game("Alpha", vec![
                make_import("z", "?", 1, "libC"),
            ]),
        ]);
        let u = find_unresolved(&db);
        assert_eq!(u.len(), 3);
        assert_eq!(u[0].game, "Alpha");
        assert_eq!(u[1].game, "Zebra");
        assert_eq!(u[1].library, "libA");
        assert_eq!(u[2].game, "Zebra");
        assert_eq!(u[2].library, "libB");
    }

    #[test]
    fn unresolved_cross_game() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("aaa", "?", 1, "libX"),
            ]),
            make_game("GameB", vec![
                make_import("aaa", "?", 1, "libX"),
                make_import("bbb", "funcB", 2, "libY"),
            ]),
        ]);
        let u = find_unresolved(&db);
        assert_eq!(u.len(), 2);
        assert!(u.iter().all(|e| e.nid_hash == "aaa"));
    }
}
