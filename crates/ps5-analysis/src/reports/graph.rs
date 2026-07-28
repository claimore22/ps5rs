use crate::model::*;
use std::collections::HashMap;

pub fn build_graph(db: &AnalysisDatabase, include_nids: bool) -> DependencyGraph {
    let mut game_nodes = Vec::new();
    let mut lib_nodes_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut nid_nodes_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    for game in &db.games {
        let gname = game
            .display_name
            .as_deref()
            .unwrap_or(&game.name)
            .to_string();
        game_nodes.push(gname.clone());

        let mut lib_counts: HashMap<String, usize> = HashMap::new();
        for imp in &game.imports {
            *lib_counts.entry(imp.library_name.clone()).or_insert(0) += 1;
        }

        for (lib, count) in &lib_counts {
            lib_nodes_set.insert(lib.clone());
            edges.push(GraphEdge {
                from: gname.clone(),
                to: lib.clone(),
                weight: *count,
            });

            if include_nids {
                for imp in &game.imports {
                    if &imp.library_name == lib {
                        nid_nodes_set.insert(imp.resolved_name.clone());
                    }
                }
            }
        }
    }

    let mut lib_nodes: Vec<String> = lib_nodes_set.into_iter().collect();
    lib_nodes.sort();
    let mut nid_nodes: Vec<String> = nid_nodes_set.into_iter().collect();
    nid_nodes.sort();

    // If including NIDs, add lib->nid edges
    if include_nids {
        let mut nid_lib_counts: HashMap<(String, String), usize> = HashMap::new();
        for game in &db.games {
            for imp in &game.imports {
                if imp.resolved_name != "?" {
                    *nid_lib_counts
                        .entry((imp.library_name.clone(), imp.resolved_name.clone()))
                        .or_insert(0) += 1;
                }
            }
        }
        for ((lib, nid), count) in &nid_lib_counts {
            edges.push(GraphEdge {
                from: lib.clone(),
                to: nid.clone(),
                weight: *count,
            });
        }
    }

    DependencyGraph {
        game_nodes,
        lib_nodes,
        edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{make_db, make_game, make_import};

    #[test]
    fn graph_empty() {
        let db = make_db(vec![]);
        let g = build_graph(&db, false);
        assert!(g.game_nodes.is_empty());
        assert!(g.lib_nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn graph_single_game_single_lib() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![
                make_import("aaa", "funcA", 1, "libA"),
                make_import("bbb", "funcB", 1, "libA"),
            ],
        )]);
        let g = build_graph(&db, false);
        assert_eq!(g.game_nodes, vec!["GameA"]);
        assert_eq!(g.lib_nodes, vec!["libA"]);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].from, "GameA");
        assert_eq!(g.edges[0].to, "libA");
        assert_eq!(g.edges[0].weight, 2);
    }

    #[test]
    fn graph_multi_game_multi_lib() {
        let db = make_db(vec![
            make_game(
                "GameA",
                vec![
                    make_import("aaa", "funcA", 1, "libA"),
                    make_import("bbb", "funcB", 2, "libB"),
                ],
            ),
            make_game("GameB", vec![make_import("aaa", "funcA", 1, "libA")]),
        ]);
        let g = build_graph(&db, false);
        assert_eq!(g.game_nodes.len(), 2);
        assert_eq!(g.lib_nodes, vec!["libA", "libB"]);
        // GameA -> libA (1), GameA -> libB (1), GameB -> libA (1)
        let game_a_to_liba = g.edges.iter().find(|e| e.from == "GameA" && e.to == "libA");
        assert_eq!(game_a_to_liba.unwrap().weight, 1);
        let game_a_to_libb = g.edges.iter().find(|e| e.from == "GameA" && e.to == "libB");
        assert_eq!(game_a_to_libb.unwrap().weight, 1);
        let game_b_to_liba = g.edges.iter().find(|e| e.from == "GameB" && e.to == "libA");
        assert_eq!(game_b_to_liba.unwrap().weight, 1);
    }

    #[test]
    fn graph_lib_nodes_sorted() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![
                make_import("a", "f", 3, "libC"),
                make_import("b", "f", 1, "libA"),
                make_import("c", "f", 2, "libB"),
            ],
        )]);
        let g = build_graph(&db, false);
        assert_eq!(g.lib_nodes, vec!["libA", "libB", "libC"]);
    }

    #[test]
    fn graph_with_nids() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "funcA", 1, "libA")],
        )]);
        let g = build_graph(&db, true);
        assert_eq!(g.game_nodes, vec!["GameA"]);
        assert_eq!(g.lib_nodes, vec!["libA"]);
        // GameA -> libA edge
        let game_edge = g.edges.iter().find(|e| e.from == "GameA" && e.to == "libA");
        assert!(game_edge.is_some());
        // libA -> funcA edge (resolved != "?")
        let lib_edge = g.edges.iter().find(|e| e.from == "libA" && e.to == "funcA");
        assert!(lib_edge.is_some());
        assert_eq!(lib_edge.unwrap().weight, 1);
    }

    #[test]
    fn graph_without_nids_no_nid_edges() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "funcA", 1, "libA")],
        )]);
        let g = build_graph(&db, false);
        // Only GameA -> libA, no libA -> funcA
        let lib_edge = g.edges.iter().find(|e| e.from == "libA" && e.to == "funcA");
        assert!(lib_edge.is_none());
    }

    #[test]
    fn graph_unresolved_nid_not_in_nid_edges() {
        let db = make_db(vec![make_game(
            "GameA",
            vec![make_import("aaa", "?", 1, "libA")],
        )]);
        let g = build_graph(&db, true);
        // No libA -> ? edge
        let bad_edge = g.edges.iter().find(|e| e.to == "?");
        assert!(bad_edge.is_none());
    }
}
