use std::{
    sync::mpsc::{Receiver, Sender},
    thread::ThreadId,
};

/// The node as a unit executable on the [`NodeRunner`]
pub struct Node<TASK: Fn() -> () + Send + 'static, IDLE: Fn() -> () + Send + 'static> {
    /// The task executed within the critical section
    pub cs_task: TASK,
    /// The task executed when outside of critical section, meant primarily to
    /// reduce or randomize the frequency of requests for access to the critical
    /// section.
    pub idle_task: IDLE,
}

impl<T: Fn() -> () + Send + 'static, I: Fn() -> () + Send + 'static> Node<T, I> {
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
                        "Node {:?} is now aware of node {:?} being in the network",
                        current_node.node_id, node_id
                    );
                }
                SystemMsg::Start => break, //starting
                _ => {
                    //do nothing untill the start
                }
            };
        }
        println!("Node {:?} finished initialization", current_node.node_id);
    }

    pub fn execute(self, mut current_node: NodeLocalData) -> () {
        // Initialize the node and wait for the main thread to signal the start
        self.initialize(&mut current_node);

        loop {
            // Start with whatever we have to do that we can do alone
            (self.idle_task)();
            current_node.clock += 1;

            // Now we would like to enter the critical section and we will keep
            // checking if we can untill we get the access.

            let request_timestamp = current_node.clock;
            let mut deferred_requests_from = Vec::<ThreadId>::new();

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
                        println!(
                            "Node {:?} asked the node {:?} for an access to the critical section",
                            node_id, current_node.node_id
                        );

                        if (timestamp == request_timestamp
                            && format!("{:?}", node_id) < format!("{:?}", current_node.node_id))
                            || timestamp < request_timestamp
                        // (a very hacky way of checking if one id is somehow smaller than the other...)
                        {
                            if timestamp == current_node.clock {
                                // fix the clock
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

                            println!(
                                "Node {:?} granted to node {:?} the access to the critical section",
                                current_node.node_id, node_id
                            );
                        } else {
                            deferred_requests_from.push(node_id);
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
                        "\x1b[93mNode {:?} proceeds into the critical section\x1b[0m",
                        current_node.node_id
                    );
                    (self.cs_task)();
                    current_node.clock += 1;
                    println!(
                        "\x1b[93mNode {:?} exits the critical section\x1b[0m",
                        current_node.node_id
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
                        .filter(|other| deferred_requests_from.contains(&other.node_id))
                        .for_each(|other| {
                            other
                                .connection
                                .send(SystemMsg::AccessApproved {
                                    node_id: current_node.node_id,
                                    timestamp: current_node.clock,
                                })
                                .unwrap();
                        });

                    break 'cs; // Exit the inner loop and return to the idle task
                }
            }
        }
    }
}

/// Data stored and used by the [`Node`]
pub struct NodeLocalData {
    pub node_id: ThreadId,
    pub connected_to: Vec<NodeInfo>, //could use a HashMap instead but the assignment instructions used the term "list"
    pub network_connection: Receiver<SystemMsg>,
    pub clock: u128,
}

impl NodeLocalData {
    pub fn new(thread_id: ThreadId, network_connection: Receiver<SystemMsg>) -> Self {
        Self {
            node_id: thread_id,
            connected_to: Vec::new(),
            network_connection,
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

/// Owns a thread and uses the thread to run the [`Node`] on it. Basically
/// a handle that can be used by the main thread to connect the nodes with each other.
pub struct NodeRunner {
    thread_handle: std::thread::JoinHandle<()>,
    pub connection: Sender<SystemMsg>,
}

pub enum SystemMsg {
    /// Message orchestrated by the main loop, used to connect nodes with each
    /// other.
    NewNodeInNetwork {
        node_id: ThreadId,
        node_tx: Sender<SystemMsg>,
    },
    /// Sent by the main thread to start the system after registering the nodes
    /// in the network. This solution is meant to reduce complexity of the code
    /// and avoid obfuscation of the general idea behind the algorithm.
    Start,
    /// Request for an access to the critical section
    CriticalSectionReq { node_id: ThreadId, timestamp: u128 },
    /// Access allowed by the node
    AccessApproved { node_id: ThreadId, timestamp: u128 },
}

impl NodeRunner {
    pub fn new<T: Fn() -> () + Send + 'static, I: Fn() -> () + Send + 'static>(
        node_task: Node<T, I>,
    ) -> Self {
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

pub fn critically_critical_task() {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    std::thread::sleep(std::time::Duration::from_millis(
        1000 * 3 + rng.gen_range(0..10000),
    ));
}

pub fn idle_task() {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    std::thread::sleep(std::time::Duration::from_millis(
        1000 + rng.gen_range(0..30000),
    ));
}

impl Default for Node<fn() -> (), fn() -> ()> {
    fn default() -> Self {
        Self {
            cs_task: critically_critical_task,
            idle_task,
        }
    }
}

/// A very simple node which uses statically defined functions as tasks
type StaticNode = Node<fn() -> (), fn() -> ()>;

fn main() {
    println!("Starting system simulation...");

    let mut runners = Vec::<NodeRunner>::new();

    for _ in 0..9 {
        let nr = NodeRunner::new(StaticNode::default());
        for other in &runners {
            let own = nr.give_registration_data();
            other.register_new_connection(own.0, own.1);
            let other = other.give_registration_data();
            nr.register_new_connection(other.0, other.1);
        }
        runners.push(nr);
    }
    for nr in runners {
        nr.start()
    }

    // make sure we do not exit too early
    loop {}
}
