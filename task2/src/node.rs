use std::{
    sync::mpsc::{Receiver, Sender},
    thread::ThreadId,
};

/// The node as a unit executable on the [`NodeRunner`]. This struct stores
/// properties of a node which are decided before the simulation even starts.
#[derive(Default)]
pub struct Node {
    /// Optional, improves logs
    pub given_name: Option<String>,
    pub instructions: Option<Vec<crate::input::NodeTaskInstruction>>,
}

impl Node {
    pub fn new(given_name: String, instructions: Vec<crate::NodeTaskInstruction>) -> Self {
        Self {
            given_name: Some(given_name),
            instructions: Some(instructions),
        }
    }

    fn initialize(&self, current_node: &mut NodeLocalData) {
        loop {
            let msg = current_node.network_connection.recv().unwrap();
            match msg {
                SystemMsg::NewNodeInNetwork { node_id, node_tx } => {
                    current_node.connected_to.push(NodeInfo {
                        is_request_accepted: false,
                        node_id,
                        connection: node_tx,
                    });

                    println!(
                        "Node {:?}{}is now aware of node {:?} being in the network",
                        current_node.node_id,
                        if let Some(name) = &self.given_name {
                            format!(" (named: {name}) ")
                        } else {
                            String::new()
                        },
                        node_id
                    );
                }
                SystemMsg::Start => break, //starting
                _ => {
                    //do nothing untill the start
                }
            };
        }
        println!(
            "Node {:?}{} finished initialization",
            current_node.node_id,
            if let Some(name) = &self.given_name {
                format!(" (named: {name}) ")
            } else {
                String::new()
            }
        );
    }
    fn is_done(&self) -> bool {
        if let Some(list) = &self.instructions {
            list.is_empty()
        } else {
            false
        }
    }
    fn execute_idle_task(&mut self) {
        if let Some(list) = &mut self.instructions {
            if let Some((idx, duration)) = list
                .iter()
                .enumerate()
                .filter_map(|(idx, instruction)| match instruction {
                    crate::NodeTaskInstruction::Idle { duration } => Some((idx, duration)),
                    _ => None,
                })
                .nth(0)
            {
                std::thread::sleep(std::time::Duration::from_millis(*duration));
                list.remove(idx);
            }
        } else {
            use rand::Rng;

            let mut rng = rand::thread_rng();
            std::thread::sleep(std::time::Duration::from_millis(
                1000 + rng.gen_range(0..30000),
            ));
        }
    }
    fn execute_in_critical_section(&mut self) {
        if let Some(list) = &mut self.instructions {
            if let Some((idx, duration)) = list
                .iter()
                .enumerate()
                .filter_map(|(idx, instruction)| match instruction {
                    crate::NodeTaskInstruction::CriticalSection { duration } => {
                        Some((idx, duration))
                    }
                    _ => None,
                })
                .nth(0)
            {
                std::thread::sleep(std::time::Duration::from_millis(*duration));
                list.remove(idx);
            }
        } else {
            use rand::Rng;

            let mut rng = rand::thread_rng();
            std::thread::sleep(std::time::Duration::from_millis(
                1000 + rng.gen_range(0..30000),
            ));
        }
    }

    // ------ FOR THE ALGORITHM IMPLEMENTATION LOOK HERE BELOW -------

    /// The "main" of every node where the implementation of the Ricart-Agrawal algorithm is implemented
    pub fn execute(mut self, mut current_node: NodeLocalData) -> () {
        // Initialize the node and wait for the main thread to signal the start
        self.initialize(&mut current_node);

        while !self.is_done() {
            // Start with whatever we have to do that we can do alone
            self.execute_idle_task();
            current_node.clock += 1;

            // Now we would like to enter the critical section and we will keep
            // checking if we can untill we get the access.

            let request_timestamp = current_node.clock;

            // Broadcast the request:
            current_node.connected_to.iter().for_each(|other| {
                other
                    .connection
                    .send(SystemMsg::CriticalSectionReq {
                        node_id: current_node.node_id,
                        timestamp: request_timestamp,
                    })
                    .unwrap();
            });
            current_node.clock += 1;

            // Wait for approvals:
            'cs: loop {
                let msg = current_node.network_connection.recv().unwrap();
                current_node.clock += 1;

                match msg {
                    SystemMsg::CriticalSectionReq { node_id, timestamp } => {
                        //log msg:
                        println!(
                            "Node {:?} asked the node {:?}{} for an access to the critical section",
                            node_id,
                            current_node.node_id,
                            if let Some(name) = &self.given_name {
                                format!(" (named: {name}) ")
                            } else {
                                String::new()
                            }
                        );

                        // NOTE:
                        // a very hacky way of checking if one id is somehow
                        // smaller than the other, a limitation introduced by the
                        // portability properties of the standard library
                        if (timestamp == request_timestamp
                            && format!("{:?}", node_id) < format!("{:?}", current_node.node_id))
                            || timestamp < request_timestamp
                        {
                            if timestamp == current_node.clock {
                                // sync the clock
                                current_node.clock = timestamp + 1;
                            }

                            //grant the access
                            let receiving_node = current_node
                                .connected_to
                                .iter()
                                .find(|other| other.node_id == node_id).expect("got a message from an unkown source, no channel to the source registered at the node");

                            receiving_node
                                .connection
                                .send(SystemMsg::AccessApproved {
                                    node_id: current_node.node_id,
                                    timestamp: current_node.clock,
                                })
                                .unwrap();

                            println!( //log msg
                                "Node {:?}{} granted to node {:?} the access to the critical section",
                                current_node.node_id, if let Some(name) = &self.given_name {
                                    format!(" (named: {name}) ")
                                } else {
                                    String::new()
                                }, node_id
                            );
                        } else {
                            current_node.deffered_reqs.push(node_id);
                        }
                    }
                    SystemMsg::AccessApproved { node_id, timestamp } => {
                        // let's synchronize clock first
                        if current_node.clock < timestamp {
                            current_node.clock = timestamp + 1;
                        }

                        // mark who has approved
                        let approving_node = current_node
                            .connected_to
                            .iter_mut()
                            .find(|other| other.node_id == node_id)
                            .expect("got an approval from a node unkown to the current node");

                        approving_node.is_request_accepted = true;
                    }
                    _ => {
                        // ignore other types of the messages after the initialization
                    }
                }

                // Check if we are finally allowed to proceed into the critical section.
                if current_node
                    .connected_to
                    .iter()
                    .all(|other| other.is_request_accepted)
                {
                    println!(
                        "\x1b[93mNode {:?}{} proceeds into the critical section\x1b[0m",
                        current_node.node_id,
                        if let Some(name) = &self.given_name {
                            format!(" (named: {name}) ")
                        } else {
                            String::new()
                        }
                    );
                    self.execute_in_critical_section();
                    current_node.clock += 1;

                    println!(
                        //log msg
                        "\x1b[93mNode {:?}{} exits the critical section\x1b[0m",
                        current_node.node_id,
                        if let Some(name) = &self.given_name {
                            format!(" (named: {name}) ")
                        } else {
                            String::new()
                        }
                    );

                    // Clean-up the flags in info on connected nodes
                    current_node
                        .connected_to
                        .iter_mut()
                        .for_each(|other| other.is_request_accepted = false);

                    // Let others know that the CS is now free
                    current_node
                        .connected_to
                        .iter()
                        .filter(|other| current_node.deffered_reqs.contains(&other.node_id))
                        .for_each(|other| {
                            other
                                .connection
                                .send(SystemMsg::AccessApproved {
                                    node_id: current_node.node_id,
                                    timestamp: current_node.clock,
                                })
                                .unwrap();
                        });
                    current_node.deffered_reqs.clear();

                    break 'cs; // Exit the inner loop and return to the idle task
                }
            }
        }

        println!(
            //log msg
            "\x1b[93mNode {:?}{} finished all its tasks\x1b[0m",
            current_node.node_id,
            if let Some(name) = &self.given_name {
                format!(" (named: {name}) ")
            } else {
                String::new()
            }
        );

        // All the tasks are finished but other nodes may be still holding the channel senders to this node.
        // If the receiver is dropped with the node the application will crash as its error handling is simplified.
        // A very hacky way to solve that is to let the thread keep running until we kill the whole application.
        loop {}
    }

    // ------ FOR THE ALGORITHM IMPLEMENTATION LOOK HERE ABOVE -------
}

/// Data stored and used by the [`Node`]. They do not define the role or tasks
/// of the `Node` but they are required at the run time of the `Node`.
pub struct NodeLocalData {
    pub node_id: ThreadId,
    // Could use a HashMap instead but the assignment instructions used the term "list" and Vec is closest to that.
    pub connected_to: Vec<NodeInfo>,
    pub network_connection: Receiver<SystemMsg>,
    pub deffered_reqs: Vec<ThreadId>,
    pub clock: u128,
}

impl NodeLocalData {
    pub fn new(thread_id: ThreadId, network_connection: Receiver<SystemMsg>) -> Self {
        Self {
            node_id: thread_id,
            connected_to: Vec::new(),
            network_connection,
            deffered_reqs: Vec::new(),
            clock: 0,
        }
    }
}

/// Information about other nodes as it is tracked locally by a node.
pub struct NodeInfo {
    pub is_request_accepted: bool,
    pub node_id: ThreadId,
    /// Allows reaching the node of which information we store directly with no
    /// need to broadcast or pass the message further.
    pub connection: Sender<SystemMsg>,
}

/// The messages sent by the nodes and also the messages which are specific to the simulation
/// and involvement of the main thread used to set-up the simulation environment.
pub enum SystemMsg {
    /// Message sent by the main thread only, used to connect nodes with each
    /// other and set up the simulation.
    NewNodeInNetwork {
        node_id: ThreadId,
        node_tx: Sender<SystemMsg>,
    },
    /// Sent by the main thread to start the system after registering the nodes
    /// in the network.
    Start,
    /// Request for an access to the critical section
    CriticalSectionReq { node_id: ThreadId, timestamp: u128 },
    /// Access allowed by the node
    AccessApproved { node_id: ThreadId, timestamp: u128 },
}

/// Owns a thread and uses the thread to run the [`Node`] on it. Basically
/// a handle that can be used by the main thread to connect the nodes with each other.
pub struct NodeRunner {
    thread_handle: std::thread::JoinHandle<()>,
    pub connection: Sender<SystemMsg>,
}

impl NodeRunner {
    pub fn new(node_task: Node) -> Self {
        // send from runner to node
        let (tx, rx) = std::sync::mpsc::channel();

        let thread = std::thread::spawn(move || {
            let thr_id = std::thread::current().id();
            let local_data = NodeLocalData::new(thr_id, rx);
            node_task.execute(local_data);
        });
        Self {
            thread_handle: thread,
            connection: tx,
        }
    }
    pub fn register_new_connection(
        &self,
        new_node_id: ThreadId,
        new_node_sender: Sender<SystemMsg>,
    ) {
        self.connection
            .send(SystemMsg::NewNodeInNetwork {
                node_id: new_node_id,
                node_tx: new_node_sender,
            })
            .expect("sending a request to register a new node in network failed");
    }
    pub fn give_registration_data(&self) -> (ThreadId, Sender<SystemMsg>) {
        (self.thread_handle.thread().id(), self.connection.clone())
    }
    pub fn start(&self) {
        self.connection.send(SystemMsg::Start).unwrap();
    }
}
