use std::collections::{BTreeSet, HashSet};

use petgraph::graph::NodeIndex;

fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an argument");
    let file = std::fs::File::open(&filename).expect("failed to open the file");
    println!("Reading file: {filename}");

    // Some sort of parseable file format was needed and the final selection ended up being the
    // json representation of petgraph::Graph type. petgraph package uses internally adjacency
    // list to store its Graph type.

    let graph: petgraph::Graph<String, (), petgraph::Directed> = serde_json::from_reader(file)
        .expect("could not read the file contents as a serialized petgraph crate graph with directed edges, string-type nodes and no edge weights");

    // Find all strongly connected components:
    let sccs = find_sccs(&graph);

    if sccs.len() > 1 {
        println!("Recording the state of all of the nodes in the graph will not be possible as the graph does not create a single strongly connected component.")
    } else {
        println!("Recording the state of all the nods within the graph is possible.")
    }

    // now if we are asking for a specific node:
    if let Some(check_node) = std::env::args().nth(2) {
        let mut found_the_arg = false;
        for scc in sccs {
            if let Some(n) = scc
                .iter()
                .map(|idx| {
                    graph
                        .node_weight(*idx)
                        .expect("nodes should not dissappear during program execution")
                })
                .find(|item| check_node == **item)
            {
                found_the_arg = true;

                let mapped_for_print: Vec<_> = scc
                    .into_iter()
                    .map(|idx| {
                        graph
                            .node_weight(idx)
                            .expect("nodes should not dissappear during program execution")
                    })
                    .collect();
                println!("Node {n} can record the state of nodes: {mapped_for_print:?}");
            }
        }
        if !found_the_arg {
            println!("Failed to find node \"{check_node}\" in the graph");
        }
    }
}

/// Implemented using Kosaraju's algorithm:
fn find_sccs<'a>(
    graph: &'a petgraph::Graph<String, (), petgraph::Directed>,
) -> Vec<Vec<NodeIndex>> {
    let mut stk = Vec::new();
    let mut visited = HashSet::<NodeIndex>::new();
    let mut sccs = Vec::new();

    for n in graph.node_indices() {
        if !visited.contains(&n) {
            dfs_stack_backtracking(n, &mut visited, graph, &mut stk)
        }
    }

    visited = HashSet::new();

    while let Some(n) = stk.pop() {
        if !visited.contains(&n) {
            let mut scc = Vec::new();
            dfs_util(
                n,
                &mut visited,
                graph,
                &mut scc,
                petgraph::Direction::Incoming,
            );
            sccs.push(scc);
        }
    }
    sccs
}

fn dfs_util(
    node_idx: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    graph: &petgraph::Graph<String, (), petgraph::Directed>,
    store: &mut Vec<NodeIndex>,
    dir: petgraph::Direction,
) {
    visited.insert(node_idx);
    store.push(node_idx);
    for n in graph.neighbors_directed(node_idx, dir) {
        if !visited.contains(&n) {
            dfs_util(n, visited, graph, store, dir);
        }
    }
}

fn dfs_stack_backtracking(
    node_idx: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    graph: &petgraph::Graph<String, (), petgraph::Directed>,
    stk: &mut Vec<NodeIndex>,
) {
    visited.insert(node_idx);
    for n in graph.neighbors_directed(node_idx, petgraph::Direction::Outgoing) {
        if !visited.contains(&n) {
            dfs_stack_backtracking(n, visited, graph, stk);
        }
        stk.push(node_idx);
    }
}
