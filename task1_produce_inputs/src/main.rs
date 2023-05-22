#![allow(non_snake_case)]
use std::collections::HashMap;

fn main() {
    use std::io::Write;

    // The example from assignment slides

    let mut graph1 = petgraph::Graph::<String, (), petgraph::Directed>::new();
    let Aidx = graph1.add_node("A".to_owned());
    let Bidx = graph1.add_node("B".to_owned());
    let Cidx = graph1.add_node("C".to_owned());
    let Didx = graph1.add_node("D".to_owned());
    let Eidx = graph1.add_node("E".to_owned());

    graph1.add_edge(Aidx, Cidx, ());
    graph1.add_edge(Aidx, Didx, ());
    graph1.add_edge(Aidx, Eidx, ());
    graph1.add_edge(Bidx, Didx, ());
    graph1.add_edge(Cidx, Bidx, ());
    graph1.add_edge(Cidx, Didx, ());
    graph1.add_edge(Didx, Bidx, ());
    graph1.add_edge(Eidx, Bidx, ());
    graph1.add_edge(Eidx, Aidx, ());

    let mut file_opt = std::fs::File::options();
    let file1 = file_opt
        .write(true)
        .open("inputs/task1/example1.json")
        .unwrap();

    serde_json::to_writer(file1, &graph1);

    // The example from the lecture slides made to be complete
    let mut graph2 = petgraph::Graph::<String, (), petgraph::Directed>::new();
    let Aidx = graph1.add_node("A".to_owned());
    let Bidx = graph1.add_node("B".to_owned());
    let Cidx = graph1.add_node("C".to_owned());
    let Didx = graph1.add_node("D".to_owned());

    graph1.add_edge(Aidx, Cidx, ());
    graph1.add_edge(Aidx, Bidx, ());
    graph1.add_edge(Bidx, Aidx, ());
    graph1.add_edge(Bidx, Didx, ());
    graph1.add_edge(Cidx, Bidx, ());
    graph1.add_edge(Cidx, Didx, ());
    graph1.add_edge(Didx, Bidx, ());

    let mut file_opt = std::fs::File::options();
    let file2 = file_opt
        .write(true)
        .open("inputs/task1/example2.json")
        .unwrap();

    serde_json::to_writer(file2, &graph2);
}
