use std::{collections::HashMap, time::Duration};

use crossbeam_channel::{Receiver, Sender};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct NodeDescription {
    pub name: String,
    pub instructions: Vec<Instruction>,
    pub connected_to: Vec<String>,
}

#[derive(Debug)]
pub struct Network {
    pub nodes: HashMap<String, NodeRunner>,
}

impl Network {
    pub fn build(from: Vec<NodeDescription>) -> Self {
        let mut connections = HashMap::<String, Vec<String>>::with_capacity(from.len());
        let mut nodes = HashMap::with_capacity(from.len());
        let mut id = 1;
        for desc in from {
            let runner = NodeRunner::new(desc.name.clone(), desc.instructions, id);
            connections.insert(desc.name.clone(), desc.connected_to);
            nodes.insert(desc.name.clone(), runner);
            id += 1;
        }
        for (node, connected) in connections {
            let connected_nodes = connected
                .into_iter()
                .map(|n| (n.clone(), nodes.get(&n).unwrap().connection.0.clone()))
                .collect::<HashMap<_, _>>();
            nodes.get_mut(&node).unwrap().init(connected_nodes);
        }

        Self { nodes }
    }
}

#[derive(Debug)]
pub enum Msg {
    RequestResource {
        requesting: String,
        resource_from: String,
        respond_back: Sender<Msg>,
    },
    /// Sent in response to the `RequestResource` if a resource is available.
    ResourceFree { resource_from: String },
    WaitingFor {
        waiting_for: String,
        waiting_for_node_lbl: usize,
    },
    /// This message will be used to make a node realise that it is being
    /// blocked. Sent only as a response to the `RequestResource` message.
    Busy {
        busy_node: String,
        busy_node_lbl: usize,
        respond_back: Sender<Msg>,
    },
    /// Sent after a node realises that it is blocked.
    TryDetect { respond_back: Sender<Msg> },
    /// After receiving the `TryDetect` message a node inspects its own labels
    /// and the stored label of another blocking process (if any). If those
    /// indicate a deadlock the `Detected` message is sent in response.
    Detected,
    /// If we have no reason to respond with `Detected`, reply with
    /// `NotDetected` to hint the node that it should resend the request and
    /// see how it goes then. Not the most efficient solution but the task
    /// handling and resource management performed by a node are assumed not to
    /// be a part of the assigmnent.
    NotDetected,
}

#[derive(Debug)]
pub struct Node {
    /// Used to get pretty status updates.
    pub name: String,
    /// The public label as described in the Mitchell-Merrit algorithm.
    pub public_lbl: usize,
    /// The private label as described in the Mitchell-Merrit algorithm.    
    pub private_lbl: usize,
    //
    // --- implementation specific data: ---
    //
    pub initialize_rx: Receiver<InitializeMsg>,
    /// Allows receiving messages from any node in the network.
    pub network_recv: Receiver<Msg>,
    pub connected_nodes: HashMap<String, Sender<Msg>>,
    /// Stores the public label of the blocking process.
    pub is_blocked_by: Option<usize>,
    pub instructions: Vec<Instruction>,
    /// Instead of using separate identifiers it is assumed that each node
    /// stores at most a single resource and as such the resources are
    /// identified by the names of the nodes owning them -- is one way to
    /// interpret this. More precisely, the program does not care about the
    /// resources per se, it only tracks who is asking who.
    pub collected_resources_from: Vec<String>,
    /// To whom which resource should be passed.
    pub requests_to_pass: Vec<(String, String)>,
    pub executed_task_handle: Option<Receiver<()>>,
    pub self_tx: Sender<Msg>,
    // -------------------------------------
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Instruction {
    Execute {
        /// How long (in seconds) the node should report that it is busy
        duration: u64,
        /// Issue a request to node specified by the name. The system will
        /// panic if the requested node cannot be reached from the instruction
        /// executing node.
        req_from: Vec<String>,
    },
    /// How long (in seconds) the node will grant access to the resources it
    /// owns.
    Idle(u64),
    /// Block the node indefintively. It will forever report that it is busy
    Block,
}

impl Node {
    /// Returns a list of nodes which need to be requested for resources in
    /// order to proceed with the task.
    fn proceed_with_next_task(&mut self) -> Option<Vec<String>> {
        if let Some(next_task) = self.instructions.iter().nth(0) {
            let (tx, rx) = crossbeam_channel::bounded(1);
            self.executed_task_handle = Some(rx);

            match next_task {
                Instruction::Execute { duration, req_from } => {
                    if req_from
                        .iter()
                        // slighlty inefficient but if we find ourselves needing to iterate over hundreds of resources
                        // then the system might have different characteristics than it was initially assumed and we might
                        // have bigger problems...
                        .all(|res| self.collected_resources_from.contains(res))
                    {
                        for requested in req_from {
                            let index = self
                                .collected_resources_from
                                .iter()
                                .enumerate()
                                .find(|(_, req)| **req == *requested)
                                .map(|(idx, _)| idx)
                                .unwrap();
                            self.collected_resources_from.remove(index);
                        }

                        let dur = *duration;
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(dur));
                            tx.send(()).unwrap();
                        });
                    } else {
                        return Some(
                            req_from
                                .iter()
                                .filter(|res| !self.collected_resources_from.contains(res))
                                .map(|x| x.clone())
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                Instruction::Idle(duration) => {
                    let dur = *duration;
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(dur));
                        tx.send(()).unwrap();
                    });
                }
                Instruction::Block => {
                    // prevent dropping the `Sender` to avoid crashing the `Receiver`
                    Box::leak(Box::new(tx));
                }
            };
        } else {
            self.executed_task_handle = None;
        }
        None
    }
    /// Returns true if a task was completed
    fn is_task_done(&mut self) -> bool {
        if let Some(rx) = &self.executed_task_handle {
            if rx.try_recv().is_ok() {
                let task = self.instructions.pop().unwrap();
                self.is_blocked_by = None;
                self.executed_task_handle = None;

                println!("Node {} finished task: {task:?}", self.name);

                if let Instruction::Execute {
                    duration: _,
                    req_from,
                } = task
                {
                    for req in req_from {
                        if let Some(direct_req) = self.connected_nodes.get(&req) {
                            direct_req
                                .send(Msg::ResourceFree { resource_from: req })
                                .unwrap();
                        } else {
                            for (to_pass_to, res_to_pass) in &self.requests_to_pass {
                                self.connected_nodes
                                    .get(to_pass_to)
                                    .unwrap()
                                    .send(Msg::ResourceFree {
                                        resource_from: res_to_pass.clone(),
                                    })
                                    .unwrap()
                            }
                        }
                    }
                }

                self.collected_resources_from = vec![self.name.clone()];

                true
            } else {
                false
            }
        } else {
            true
        }
    }
    fn initialize(&mut self) {
        self.connected_nodes = self.initialize_rx.recv().unwrap().connected_to;
        self.collected_resources_from.push(self.name.clone());
        println!("Node {} initialized successfully!", self.name);
    }

    pub fn execute(&mut self) {
        self.initialize();
        'doing_tasks: loop {
            if self.instructions.is_empty() {
                break 'doing_tasks;
            }
            if let Some(requests_pending) = self.proceed_with_next_task() {
                for req in requests_pending {
                    if req != self.name {
                        for (_, neighbour) in &self.connected_nodes {
                            neighbour
                                .send(Msg::RequestResource {
                                    requesting: self.name.clone(),
                                    resource_from: req.clone(),
                                    respond_back: self.self_tx.clone(),
                                })
                                .unwrap();
                        }
                    }
                }
            }
            'reading_msg: while !self.is_task_done() {
                if let Ok(msg) = self.network_recv.try_recv() {
                    match msg {
                        Msg::RequestResource {
                            requesting,
                            resource_from,
                            respond_back,
                        } => {
                            if let Some((index, _)) = self
                                .collected_resources_from
                                .iter()
                                .enumerate()
                                .find(|(_, res)| **res == resource_from)
                            {
                                self.collected_resources_from.remove(index);
                                respond_back
                                    .send(Msg::ResourceFree { resource_from })
                                    .unwrap();
                            } else if self.name == resource_from {
                                self.requests_to_pass
                                    .push((requesting.clone(), resource_from.clone()));

                                for (_, n) in &self.connected_nodes {
                                    n.send(Msg::RequestResource {
                                        requesting: requesting.clone(),
                                        resource_from: resource_from.clone(),
                                        respond_back: self.self_tx.clone(),
                                    })
                                    .unwrap();
                                }
                                respond_back
                                    .send(Msg::Busy {
                                        busy_node_lbl: self.public_lbl,
                                        respond_back: self.self_tx.clone(),
                                        busy_node: self.name.clone(),
                                    })
                                    .unwrap();
                            } else {
                                respond_back
                                    .send(Msg::Busy {
                                        busy_node_lbl: self.public_lbl,
                                        respond_back: self.self_tx.clone(),
                                        busy_node: self.name.clone(),
                                    })
                                    .unwrap();
                            }
                        }
                        Msg::ResourceFree { resource_from } => {
                            println!(
                                "Node {} received access to resources owned by {resource_from}",
                                self.name
                            );
                            self.collected_resources_from.push(resource_from);
                        }
                        Msg::Busy {
                            busy_node_lbl,
                            respond_back,
                            busy_node,
                        } => {
                            let new_lbl = busy_node_lbl.max(self.public_lbl) + 1;
                            self.public_lbl = new_lbl;
                            self.private_lbl = new_lbl;
                            self.is_blocked_by = Some(busy_node_lbl);

                            println!(
                                "Node {} is being blocked by a node {busy_node} with public label {busy_node_lbl}",
                                self.name
                            );

                            respond_back
                                .send(Msg::TryDetect {
                                    respond_back: self.self_tx.clone(),
                                })
                                .unwrap();
                        }
                        Msg::TryDetect { respond_back } => {
                            if self.public_lbl == self.private_lbl
                                && self.is_blocked_by == Some(self.public_lbl)
                            {
                                respond_back.send(Msg::Detected).unwrap();
                            } else {
                                respond_back.send(Msg::NotDetected).unwrap();
                            }
                        }
                        Msg::Detected => println!("Node {} detected a deadlock!", self.name),
                        Msg::NotDetected => {
                            println!(
                                "Node {} checked for deadlock but did not detect any",
                                self.name
                            );
                            break 'reading_msg;
                        }
                    }
                }
            }
        }
        println!("Node {} finished all its tasks", self.name);
        loop {
            match self.network_recv.recv().unwrap() {
                Msg::RequestResource {
                    requesting: _,
                    resource_from,
                    respond_back,
                } => {
                    if let Some((index, _)) = self
                        .collected_resources_from
                        .iter()
                        .enumerate()
                        .find(|(_, res)| **res == resource_from)
                    {
                        self.collected_resources_from.remove(index);
                        respond_back
                            .send(Msg::ResourceFree { resource_from })
                            .unwrap();
                    }
                }
                Msg::ResourceFree { resource_from } => {
                    self.collected_resources_from.push(resource_from);
                }
                Msg::Detected => {
                    println!("Node {} detected a deadlock!", self.name)
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug)]
pub struct NodeRunner {
    thread_handle: std::thread::JoinHandle<()>,
    pub connection: (Sender<Msg>, Sender<InitializeMsg>),
}

impl NodeRunner {
    pub fn new(name: String, instructions: Vec<Instruction>, id: usize) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (init_tx, init_rx) = crossbeam_channel::bounded(1);
        let cloned_tx = tx.clone();
        let thread = std::thread::spawn(move || {
            let mut node = Node {
                name,
                public_lbl: id,
                private_lbl: id,
                network_recv: rx,
                connected_nodes: HashMap::new(),
                is_blocked_by: None,
                instructions,
                is_busy: false,
                collected_resources_from: Vec::new(),
                executed_task_handle: None,
                self_tx: tx,
                initialize_rx: init_rx,
                requests_to_pass: Vec::new(),
            };
            node.execute();
        });
        Self {
            thread_handle: thread,
            connection: (cloned_tx, init_tx),
        }
    }
    pub fn init(&mut self, connected_nodes: HashMap<String, Sender<Msg>>) {
        self.connection
            .1
            .send(InitializeMsg {
                connected_to: connected_nodes,
            })
            .unwrap();
    }
}

#[derive(Debug)]
pub struct InitializeMsg {
    pub connected_to: HashMap<String, Sender<Msg>>,
}

fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an input");
    println!("Reading from file {filename}");

    let file = std::fs::File::open(&filename).expect("failed to open the file");
    let nodes: Vec<NodeDescription> =
        serde_json::from_reader(file).expect("failed to deserialize into a vector of nodes");

    let net = Network::build(nodes);

    loop {}
}
