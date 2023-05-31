use petgraph::graph::UnGraph;

pub struct Network {
    pub nodes: UnGraph<NodeResources, ()>,
    pub controller: Controller,
}

pub struct Controller {
    pub resources: Vec<String>,
    pub processes: Vec<String>,
    pub status_table: Vec<Vec<u8>>,
}

pub struct NodeResources {
    pub resources: Vec<String>,
    pub processes: Vec<String>,
    pub status_table: Vec<Vec<u8>>,
}

fn main() {
    println!("Hello, world!");
}
