use crate::model::*;
use std::collections::HashMap;

pub fn build_graph(db: &AnalysisDatabase, include_nids: bool) -> DependencyGraph {
    let mut game_nodes = Vec::new();
    let mut lib_nodes_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut nid_nodes_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut edges: Vec<GraphEdge> = Vec::new();

    for game in &db.games {
        game_nodes.push(game.name.clone());

        let mut lib_counts: HashMap<String, usize> = HashMap::new();
        for imp in &game.imports {
            *lib_counts.entry(imp.library_name.clone()).or_insert(0) += 1;
        }

        for (lib, count) in &lib_counts {
            lib_nodes_set.insert(lib.clone());
            edges.push(GraphEdge {
                from: game.name.clone(),
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
                    *nid_lib_counts.entry((imp.library_name.clone(), imp.resolved_name.clone())).or_insert(0) += 1;
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

    DependencyGraph { game_nodes, lib_nodes, edges }
}
