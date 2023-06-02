use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct NodeName(pub String);
impl std::fmt::Display for NodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ResourceName(pub String);

#[derive(serde::Deserialize, Debug)]
pub struct NodeDescription {
    pub name: NodeName,
    pub resources: Vec<ResourceName>,
    pub instructions: Vec<Instruction>,
    pub connected_to: Vec<NodeName>,
}

#[derive(Debug)]
pub struct Network {
    pub nodes: HashMap<NodeName, NodeRunner>,
}

impl Network {
    pub fn build(from: Vec<NodeDescription>) -> Self {
        let mut connections = HashMap::<NodeName, Vec<NodeName>>::with_capacity(from.len());
        let mut nodes = HashMap::with_capacity(from.len());
        let mut id = 1;
        for desc in from {
            let runner = NodeRunner::new(desc.name.clone(), desc.instructions, desc.resources, id);
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
pub enum Msg {}

#[derive(Debug)]
pub struct Node {
    /// The public label as described in the Mitchell-Merrit algorithm.
    pub public_lbl: usize,
    /// The private label as described in the Mitchell-Merrit algorithm.    
    pub private_lbl: usize,
    //
    // --- implementation specific data: ---
    //
    /// Used to get pretty status updates.
    pub name: NodeName,
    /// Used only in the initialization process
    pub initialize_rx: Receiver<InitializeMsg>,
    /// Allows receiving messages from any node in the network.
    pub network_recv: Receiver<Msg>,
    /// Makes it possible to send to a specific neighbour.
    pub connected_nodes: HashMap<NodeName, Sender<Msg>>,
    /// Tasks for the node to execute while it is listening to the messages.
    pub instructions: Vec<Instruction>,
    /// The resources used currently by the node.
    pub collected_resources: Vec<ResourceName>,
    /// To whom which resource should be passed.
    pub waiting_for_resource: HashMap<NodeName, Vec<ResourceName>>,
    /// Allows inspecting the bacground task status.
    pub executed_task_handle: Option<Receiver<()>>,
    // -------------------------------------
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Instruction {
    Execute {
        /// How long (in seconds) the node should report that it is busy
        duration: u64,
        /// Issue a request to node specified by the name. The system will
        /// panic if the requested node cannot be reached from the instruction
        /// executing node.
        resources: Vec<ResourceName>,
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
    fn proceed_with_next_task(&mut self) -> Option<Vec<ResourceName>> {
        if let Some(next_task) = self.instructions.iter().nth(0) {
            let (tx, rx) = crossbeam_channel::bounded(1);
            self.executed_task_handle = Some(rx);

            match next_task {
                Instruction::Execute {
                    duration,
                    resources,
                } => {
                    if resources
                        .iter()
                        // slighlty inefficient but if we find ourselves needing to iterate over hundreds of resources
                        // then the system might have different characteristics than it was initially assumed and we might
                        // have bigger problems...
                        .all(|res| self.collected_resources.contains(res))
                    {
                        for requested in resources {
                            let index = self
                                .collected_resources
                                .iter()
                                .enumerate()
                                .find(|(_, req)| **req == *requested)
                                .map(|(idx, _)| idx)
                                .unwrap();
                            self.collected_resources.remove(index);
                        }

                        let dur = *duration;
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(dur));
                            tx.send(()).unwrap();
                        });
                    } else {
                        return Some(
                            resources
                                .iter()
                                .filter(|res| !self.collected_resources.contains(res))
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
                self.executed_task_handle = None;

                println!("Node {} finished task: {task:?}", self.name);

                // if let Instruction::Execute {
                //     duration: _,
                //     resources,
                // } = task
                // {
                //     for req in resources {
                //         if let Some(direct_req) = self.connected_nodes.get(&req) {
                //             direct_req
                //                 .send(Msg::ResourceFree { resource_from: req })
                //                 .unwrap();
                //         } else {
                //             for (to_pass_to, res_to_pass) in &self.requests_to_pass {
                //                 self.connected_nodes
                //                     .get(to_pass_to)
                //                     .unwrap()
                //                     .send(Msg::ResourceFree {
                //                         resource_from: res_to_pass.clone(),
                //                     })
                //                     .unwrap()
                //             }
                //         }
                //     }
                // }

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
        println!("Node {} initialized successfully!", self.name);
    }

    pub fn execute(&mut self) {
        self.initialize();
        'doing_tasks: loop {
            if self.instructions.is_empty() {
                break 'doing_tasks;
            }
            if let Some(resources_missing) = self.proceed_with_next_task() {
                for req in resources_missing {
                    self.public_lbl += 1;
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
    pub fn new(
        name: NodeName,
        instructions: Vec<Instruction>,
        resources: Vec<ResourceName>,
        id: usize,
    ) -> Self {
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
                instructions,
                executed_task_handle: None,
                initialize_rx: init_rx,
                collected_resources: resources,
                waiting_for_resource: HashMap::new(),
            };
            node.execute();
        });
        Self {
            thread_handle: thread,
            connection: (cloned_tx, init_tx),
        }
    }
    pub fn init(&mut self, connected_nodes: HashMap<NodeName, Sender<Msg>>) {
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
    pub connected_to: HashMap<NodeName, Sender<Msg>>,
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
