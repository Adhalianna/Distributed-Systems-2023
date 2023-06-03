use std::collections::HashMap;

#[derive(serde::Deserialize)]
pub struct NodeDescription {
    pub name: String,
    pub requests: Vec<RequestDescription>,
}

#[derive(serde::Deserialize)]
pub struct RequestDescription {
    pub req_from: String,
    /// Makes sure that requests are blocked in the expected order.
    pub global_order: usize,
}

pub struct Node {
    pub name: String,
    pub public_lbl: usize,
    private_lbl: usize,
    pub received_req_from: Vec<String>,
    resource_available: bool,
}

impl Node {
    pub fn new(name: String, lbl: usize) -> Self {
        println!(
            "Node {} starts with labels (pub: {}, priv {})",
            name, lbl, lbl
        );

        Self {
            name,
            public_lbl: lbl,
            private_lbl: lbl,
            received_req_from: Vec::new(),
            resource_available: true,
        }
    }
    pub fn process_msg(&mut self, from: String, msg: Msg) -> Option<(String, Msg)> {
        match msg {
            Msg::RequestResource => {
                self.received_req_from.push(from.clone());
                if !self.resource_available {
                    Some((
                        from,
                        Msg::Block {
                            public_lbl: self.public_lbl,
                        },
                    ))
                } else {
                    self.resource_available = false;
                    Some((from, Msg::GrantResource))
                }
            }
            Msg::Block { public_lbl } => {
                let new_lbl = public_lbl.max(self.public_lbl) + 1;

                self.public_lbl = new_lbl;
                self.private_lbl = new_lbl;

                println!(
                    "Node {} changed labels to (pub: {}, priv {})",
                    self.name, self.public_lbl, self.private_lbl
                );

                if let Some(blocked) = self.received_req_from.pop() {
                    Some((blocked, Msg::TransmitBlock { public_lbl }))
                } else {
                    None
                }
            }
            Msg::TransmitBlock { public_lbl } => {
                println!("Node {name} received the transit message", name = self.name);

                if self.public_lbl == self.private_lbl && public_lbl == self.public_lbl {
                    println!("Node {name} has detected a deadlock!", name = self.name);
                }
                if public_lbl > self.public_lbl {
                    self.public_lbl = public_lbl;

                    println!(
                        "Node {} changed labels to (pub: {}, priv {})",
                        self.name, self.public_lbl, self.private_lbl
                    );

                    if let Some(blocked) = self.received_req_from.pop() {
                        Some((blocked, Msg::TransmitBlock { public_lbl }))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Msg::GrantResource => {
                self.private_lbl += 1;
                None
            }
        }
    }
    pub fn compose_req(&mut self, to: &String) -> Msg {
        println!(
            "Node {name} sent request for resources to node {to}",
            name = self.name
        );
        Msg::RequestResource
    }
}

pub enum Msg {
    RequestResource,
    GrantResource,
    /// When a node issues a request that may not be granted 'block' it using
    /// the `Block` message containg the public label of the blocking process.
    Block {
        public_lbl: usize,
    },
    /// The `TransmitBlock` is propagated back in the wait-for-graph when a
    /// node which was previously asked for resources gets itself blocked by
    /// another node.
    TransmitBlock {
        public_lbl: usize,
    },
}

pub struct Network {
    pub nodes: HashMap<String, Node>,
    pub requests: Vec<(String, RequestDescription)>,
}

impl Network {
    /// Change descriptions into a structure that will be
    pub fn build(from: Vec<NodeDescription>) -> Self {
        let mut requests = Vec::with_capacity(from.len());
        let mut nodes = HashMap::with_capacity(from.len());
        let mut id = 1;

        for node in from {
            nodes.insert(node.name.clone(), Node::new(node.name.clone(), id));

            for req in node.requests {
                requests.push((node.name.clone(), req));
            }
            id += 1;
        }

        requests.sort_by(|r1, r2| r2.1.global_order.cmp(&r1.1.global_order));

        Self { nodes, requests }
    }
    pub fn run_algorithm(&mut self) {
        while let Some((n, req)) = self.requests.pop() {
            let new_resource_request = self.nodes.get_mut(&n).unwrap().compose_req(&req.req_from);
            let mut next_msg = self
                .nodes
                .get_mut(&req.req_from)
                .unwrap()
                .process_msg(n, new_resource_request);
            // propagate the _transit_ or _block_ messages back to the nodes
            // which requested access to the resource:
            while let Some((to, msg)) = next_msg {
                next_msg = self.nodes.get_mut(&to).unwrap().process_msg(to, msg);
            }
        }
        println!("Scenario finished running.")
    }
}

pub fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an input");
    println!("Reading from file {filename}");
    let file = std::fs::File::open(&filename).expect("failed to open the file");
    let nodes: Vec<NodeDescription> =
        serde_json::from_reader(file).expect("failed to deserialize into a vector of nodes");

    let mut net = Network::build(nodes);
    net.run_algorithm()
}
