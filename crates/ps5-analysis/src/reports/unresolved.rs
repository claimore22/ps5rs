use crate::model::*;

pub fn find_unresolved(db: &AnalysisDatabase) -> Vec<UnresolvedEntry> {
    let mut entries = Vec::new();
    for game in &db.games {
        for imp in &game.imports {
            if imp.resolved_name == "?" {
                entries.push(UnresolvedEntry {
                    game: game.name.clone(),
                    library: imp.library_name.clone(),
                    nid_hash: imp.nid_hash.clone(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.game.cmp(&b.game).then(a.library.cmp(&b.library)));
    entries
}
