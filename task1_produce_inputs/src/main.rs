#![allow(non_snake_case)]
use std::collections::HashMap;

fn main() {
    use std::io::Write;

    //---
    let mut graph1 = petgraph::Graph::<String, (), petgraph::Directed>::new();
    let Aidx = graph1.add_node("A".to_owned());
    let Bidx = graph1.add_node("B".to_owned());
    let Cidx = graph1.add_node("C".to_owned());
    let Didx = graph1.add_node("D".to_owned());
    let Eidx = graph1.add_node("E".to_owned());
    let Fidx = graph1.add_node("F".to_owned());
    let Gidx = graph1.add_node("G".to_owned());

    graph1.add_edge(Aidx, Cidx, ());
    graph1.add_edge(Aidx, Didx, ());
    graph1.add_edge(Aidx, Eidx, ());
    graph1.add_edge(Bidx, Didx, ());
    graph1.add_edge(Cidx, Bidx, ());
    graph1.add_edge(Cidx, Didx, ());
    graph1.add_edge(Didx, Bidx, ());
    graph1.add_edge(Eidx, Bidx, ());
    graph1.add_edge(Eidx, Aidx, ());
    graph1.add_edge(Fidx, Gidx, ());
    graph1.add_edge(Fidx, Aidx, ());

    // JSON
    let mut file_opt = std::fs::File::options();
    let file = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example1.json")
        .expect("failed to open or create file");

    serde_json::to_writer(file, &graph1).unwrap();

    // Graphviz for preview
    let mut file_opt = std::fs::File::options();
    let mut file = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example1.dot")
        .expect("failed to open or create file");

    file.write_fmt(format_args!(
        "{:?}",
        petgraph::dot::Dot::with_config(&graph1, &[petgraph::dot::Config::EdgeNoLabel])
    ))
    .unwrap();

    //---

    let mut graph2 = petgraph::Graph::<String, (), petgraph::Directed>::new();
    let Aidx = graph2.add_node("A".to_owned());
    let Bidx = graph2.add_node("B".to_owned());
    let Cidx = graph2.add_node("C".to_owned());
    let Didx = graph2.add_node("D".to_owned());
    let Eidx = graph2.add_node("E".to_owned());
    let Fidx = graph2.add_node("F".to_owned());

    graph2.add_edge(Aidx, Cidx, ());
    graph2.add_edge(Aidx, Bidx, ());
    graph2.add_edge(Bidx, Aidx, ());
    graph2.add_edge(Bidx, Didx, ());
    graph2.add_edge(Cidx, Bidx, ());
    graph2.add_edge(Cidx, Didx, ());
    graph2.add_edge(Didx, Bidx, ());
    graph2.add_edge(Didx, Eidx, ());
    graph2.add_edge(Eidx, Didx, ());
    graph2.add_edge(Fidx, Eidx, ());

    let mut file_opt = std::fs::File::options();
    let file2 = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example2.json")
        .expect("failed to open or create file");

    serde_json::to_writer(file2, &graph2).unwrap();

    // Graphviz for preview
    let mut file_opt = std::fs::File::options();
    let mut file = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example2.dot")
        .expect("failed to open or create file");

    file.write_fmt(format_args!(
        "{:?}",
        petgraph::dot::Dot::with_config(&graph2, &[petgraph::dot::Config::EdgeNoLabel])
    ))
    .unwrap();

    //---

    let mut graph3 = petgraph::Graph::<String, (), petgraph::Directed>::new();
    let Aidx = graph3.add_node("A".to_owned());
    let Bidx = graph3.add_node("B".to_owned());
    let Cidx = graph3.add_node("C".to_owned());
    let Didx = graph3.add_node("D".to_owned());
    let Fidx = graph3.add_node("F".to_owned());

    graph3.add_edge(Aidx, Bidx, ());
    graph3.add_edge(Bidx, Aidx, ());
    graph3.add_edge(Bidx, Didx, ());
    graph3.add_edge(Didx, Bidx, ());
    graph3.add_edge(Didx, Cidx, ());
    graph3.add_edge(Didx, Aidx, ());
    graph3.add_edge(Cidx, Fidx, ());
    graph3.add_edge(Fidx, Cidx, ());

    let mut file_opt = std::fs::File::options();
    let file3 = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example3.json")
        .expect("failed to open or create file");

    serde_json::to_writer(file3, &graph3).unwrap();

    // Graphviz for preview
    let mut file_opt = std::fs::File::options();
    let mut file = file_opt
        .write(true)
        .create(true)
        .open("inputs/task1/example3.dot")
        .expect("failed to open or create file");

    file.write_fmt(format_args!(
        "{:?}",
        petgraph::dot::Dot::with_config(&graph3, &[petgraph::dot::Config::EdgeNoLabel])
    ))
    .unwrap();
}
