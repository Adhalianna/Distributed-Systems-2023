use std::{
    collections::{HashMap, HashSet},
    iter::Inspect,
};

use petgraph::{
    graph::{self, NodeIndex},
    visit::IntoNodeIdentifiers,
    Directed,
};

fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an argument");
    let file = std::fs::File::open(&filename).expect("failed to open the file");
    println!("Reading file: {filename}");

    // Some sort of parseable file format was needed and the final selection ended up being the
    // json representation of petgraph::Graph type. petgraph package uses internally adjacency
    // list to store its Graph type.

    let graph: petgraph::Graph<String, (), Directed> = serde_json::from_reader(file)
        .expect("could not read the file contents as a serialized petgraph crate graph with directed edges, string-type nodes and no edge weights");

    if let Some(selected_node) = std::env::args().nth(2) {
        let select_idx = graph
            .node_indices()
            .find(|idx| graph.node_weight(*idx).unwrap() == &selected_node)
            .expect("failed to find the selected node in the provided graph");

        if bfs_check_initiator(select_idx, &graph) {
            println!("{selected_node} is a good candidate for an initiator!");
        } else {
            println!("{selected_node} is NOT a good candidate for an initiator");
        }
    } else {
        let good_candidates = bfs_check_all_initiators(&graph);
        if !good_candidates.is_empty() {
            println!(
                "nodes [ {} ] make good candidates for initiators",
                good_candidates
                    .iter()
                    .map(|idx| graph.node_weight(*idx).unwrap())
                    .map(|lbl| lbl.to_string())
                    .reduce(|acc, next| acc + ", " + &next)
                    .unwrap()
            )
        } else {
            println!("no good candidates found");
        }
    }
}

fn bfs_check_initiator(
    candidate_idx: NodeIndex,
    graph: &petgraph::Graph<String, (), Directed>,
) -> bool {
    let mut nodes_covered = HashSet::new();
    let mut to_visit = Vec::new();
    let nodes_total_num = graph.node_count();

    nodes_covered.insert(candidate_idx);
    to_visit.push(candidate_idx);

    while let Some(inspect) = to_visit.pop() {
        for neigh in graph.neighbors(inspect) {
            if nodes_covered.insert(neigh) {
                to_visit.push(neigh)
            }
        }
        if nodes_covered.len() == nodes_total_num {
            return true;
        }
    }

    !(nodes_covered.len() < nodes_total_num)
}

fn bfs_check_all_initiators(graph: &petgraph::Graph<String, (), Directed>) -> Vec<NodeIndex> {
    let mut initiators = Vec::new();
    for n in graph.node_indices() {
        if bfs_check_initiator(n, graph) {
            initiators.push(n);
        }
    }
    initiators
}
