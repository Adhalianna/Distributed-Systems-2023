use std::{
    collections::VecDeque,
    thread::{JoinHandle, ThreadId},
};

use crate::tree::{TreeNode, WeakNodeRef};
use crossbeam_channel::{Receiver, Sender};

#[derive(Clone)]
pub struct NodeId(pub std::thread::ThreadId);

pub enum SystemMsg {
    TokenMsg,
    /// Required as an additional step of reversing the connection between nodes.
    /// A constraint added by the simulation environment.
    NewParentSender {
        parent: Sender<SystemMsg>,
    },
    /// Wait for Start message before running the simulation
    Start,
    RequestMsg {
        node_id: std::thread::ThreadId,
        requesting: Sender<SystemMsg>,
    },
}

pub struct SystemNode(TreeNode<SystemNodeData>);

#[derive(Clone, Debug)]
pub struct NodeLocalData {
    node_id: std::thread::ThreadId,
    parent_tx: Option<Sender<SystemMsg>>,
    // a sender given out by the node when a child needs new parent_tx
    // or when a new request is sent. The primary end point of the
    // children_rx.
    self_tx: Sender<SystemMsg>,
    children_rx: Receiver<SystemMsg>,
    requests_queue: VecDeque<(ThreadId, Sender<SystemMsg>)>,
}

impl SystemNode {
    pub fn execute(&mut self) {
        // extract some data as variables to function to avoid locking the lock
        let self_id = self.node_id();
        let display_name = self.display_name();

        // extra info to logs:
        println!("{display_name} finished initialization!");
        if self.is_root() {
            println!("{display_name} is the starting root!");
        }

        //start only on receiving the Start message
        self.wait_for_start();

        loop {
            self.execute_idle_task();

            {
                self.push_request(self_id, self.new_self_tx());

                if let Some(parent_tx) = self.parent_tx() {
                    parent_tx
                        .send(SystemMsg::RequestMsg {
                            node_id: self_id,
                            requesting: self.new_self_tx(),
                        })
                        .unwrap();

                    println!("{display_name} sent a request for CS access");
                } else {
                    self.start_with_token_to_self();
                }
            }
            loop {
                match self.recv() {
                    SystemMsg::TokenMsg => {
                        println!("{display_name} received a token");
                        if !self.is_root() {
                            // Now become a root
                            self.make_self_root();
                            println!("{display_name} is now a root!");

                            self.pop_do_own_and_pass(self_id);
                            break;
                        }
                    }
                    SystemMsg::NewParentSender { parent } => {
                        self.set_parent(parent);
                        println!("{display_name} switched its parent");
                    }
                    SystemMsg::RequestMsg {
                        node_id,
                        requesting,
                    } => {
                        println!("{display_name} received a request");
                        self.push_request(node_id, requesting);
                    }
                    SystemMsg::Start => {}
                }
            }
        }
    }
}

/// Some less interesting methods:
impl SystemNode {
    fn pop_do_own_and_pass(&mut self, self_id: ThreadId) {
        if let Some((id, tx)) = self.pop_request() {
            if id == self_id {
                // Execute CS
                self.execute_cs_task();
                println!("continue...");
                self.pop_do_own_and_pass(self_id);
            } else {
                println!("sending token...");
                tx.send(SystemMsg::TokenMsg).unwrap();
            }
        }
    }
    fn start_with_token_to_self(&self) {
        self.0
            .read()
            .data()
            .as_ref()
            .self_tx
            .send(SystemMsg::TokenMsg)
            .unwrap();
    }
    fn make_self_root(&mut self) {
        self.0.make_root();

        // Make sure that the new child can send to us
        {
            let mut self_wg = self.0.write();
            let data = self_wg.data_mut().as_mut();

            data.parent_tx
                .as_ref()
                .expect("a token must only be passed by a parent (to a node aware of the parent)")
                .send(SystemMsg::NewParentSender {
                    parent: data.self_tx.clone(),
                })
                .unwrap();
            data.parent_tx = None;
        }
    }
    fn display_name(&self) -> String {
        self.0.read().data().display_name()
    }
    fn parent_tx(&self) -> Option<Sender<SystemMsg>> {
        self.0.read().data().as_ref().parent_tx.clone()
    }
    fn set_parent(&mut self, parent_tx: Sender<SystemMsg>) {
        let mut wg = self.0.write();
        let data = wg.data_mut().as_mut();
        data.parent_tx = Some(parent_tx);
    }
    fn recv(&self) -> SystemMsg {
        let rg = self.0.read();
        let node_data = rg.data().as_ref();
        let msg = node_data.children_rx.recv().unwrap();
        std::mem::drop(rg);
        msg
    }
    fn is_root(&self) -> bool {
        self.0.is_root()
    }
    fn node_id(&self) -> ThreadId {
        self.0.read().data().as_ref().node_id
    }
    fn push_request(&mut self, node_id: ThreadId, tx: Sender<SystemMsg>) {
        self.0
            .write()
            .data_mut()
            .as_mut()
            .requests_queue
            .push_back((node_id, tx));
    }
    fn pop_request(&mut self) -> Option<(ThreadId, Sender<SystemMsg>)> {
        self.0
            .write()
            .data_mut()
            .as_mut()
            .requests_queue
            .pop_front()
    }
    fn new_self_tx(&self) -> Sender<SystemMsg> {
        self.0.read().data().as_ref().self_tx.clone()
    }
    fn wait_for_start(&self) {
        while match self.recv() {
            SystemMsg::Start => false,
            _ => true,
        } {}
    }

    fn execute_idle_task(&mut self) {
        println!(
            "{} is executing the idle task",
            self.0.read().data().display_name()
        );

        match self.0.write().data_mut() {
            SystemNodeData::Scenario {
                given_name: _,
                tasks,
                core: _,
            } => {
                if let Some((idx, duration)) = tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, instruction)| match instruction {
                        NodeTaskInstruction::Idle { duration } => Some((idx, duration)),
                        _ => None,
                    })
                    .nth(0)
                {
                    std::thread::sleep(std::time::Duration::from_millis(*duration));
                    tasks.remove(idx);
                }
            }
            SystemNodeData::Simulated(_) => {
                use rand::Rng;

                let mut rng = rand::thread_rng();
                std::thread::sleep(std::time::Duration::from_millis(
                    500 + rng.gen_range(0..1000),
                ));
            }
        }
        println!(
            "{} finished the idle task",
            self.0.read().data().display_name()
        );
    }

    fn execute_cs_task(&mut self) {
        println!(
            "{} is executing the CS task",
            self.0.read().data().display_name()
        );

        match self.0.write().data_mut() {
            SystemNodeData::Scenario {
                given_name: _,
                tasks,
                core: _,
            } => {
                if let Some((idx, duration)) = tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, instruction)| match instruction {
                        NodeTaskInstruction::CriticalSection { duration } => Some((idx, duration)),
                        _ => None,
                    })
                    .nth(0)
                {
                    std::thread::sleep(std::time::Duration::from_millis(*duration));
                    tasks.remove(idx);
                }
            }
            SystemNodeData::Simulated(_) => {
                use rand::Rng;

                let mut rng = rand::thread_rng();
                std::thread::sleep(std::time::Duration::from_millis(
                    500 + rng.gen_range(0..1000),
                ));
            }
        }

        println!("{} finished the CS task", self.display_name());
    }
}

#[derive(Debug)]
pub struct SystemNodeRunner {
    node_handle: WeakNodeRef<SystemNodeData>,
}

impl SystemNodeRunner {
    pub fn start(&self) {
        self.node_handle
            .upgrade()
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .data()
            .as_ref()
            .self_tx
            .send(SystemMsg::Start)
            .unwrap();
    }
    pub fn new() -> (Self, JoinHandle<()>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(1);
        let node_self_tx = tx.clone();
        let thr = std::thread::spawn(move || {
            let mut node = SystemNode(TreeNode::new_tree(SystemNodeData::Simulated(
                NodeLocalData {
                    parent_tx: None,
                    children_rx: rx,
                    self_tx: node_self_tx,
                    requests_queue: VecDeque::new(),
                    node_id: std::thread::current().id(),
                },
            )));
            oneshot_tx.send(node.0.weak()).unwrap();

            node.execute();
        });

        (
            SystemNodeRunner {
                node_handle: oneshot_rx.recv().unwrap(),
            },
            thr,
        )
    }
    pub fn new_scenario_root(
        given_name: String,
        tasks: Vec<NodeTaskInstruction>,
    ) -> (Self, JoinHandle<()>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(1);
        let node_self_tx = tx.clone();
        let thr = std::thread::spawn(move || {
            let mut node = SystemNode(TreeNode::new_tree(SystemNodeData::Scenario {
                core: NodeLocalData {
                    parent_tx: None,
                    children_rx: rx,
                    self_tx: node_self_tx,
                    requests_queue: VecDeque::new(),
                    node_id: std::thread::current().id(),
                },
                given_name,
                tasks,
            }));
            oneshot_tx.send(node.0.weak()).unwrap();

            node.execute();
        });

        (
            SystemNodeRunner {
                node_handle: oneshot_rx.recv().unwrap(),
            },
            thr,
        )
    }
    pub fn new_under(other: &SystemNodeRunner) -> (Self, JoinHandle<()>) {
        let parent_tx = other
            .node_handle
            .upgrade()
            .unwrap()
            .read()
            .unwrap()
            .data()
            .as_ref()
            .self_tx
            .clone();
        let weak_ref = other.node_handle.clone();

        let (tx, rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(1);
        let node_self_tx = tx.clone();

        let thr = std::thread::spawn(move || {
            let mut node = SystemNode(TreeNode::new_under_weak(
                weak_ref,
                SystemNodeData::Simulated(NodeLocalData {
                    parent_tx: Some(parent_tx),
                    children_rx: rx,
                    self_tx: node_self_tx,
                    requests_queue: VecDeque::new(),
                    node_id: std::thread::current().id(),
                }),
            ));
            oneshot_tx.send(node.0.weak()).unwrap();

            node.execute();
        });

        (
            SystemNodeRunner {
                node_handle: oneshot_rx.recv().unwrap(),
            },
            thr,
        )
    }
    pub fn new_scenario_child_of(
        other: &SystemNodeRunner,
        given_name: String,
        tasks: Vec<NodeTaskInstruction>,
    ) -> (Self, JoinHandle<()>) {
        let parent_tx = other
            .node_handle
            .upgrade()
            .unwrap()
            .read()
            .unwrap()
            .data()
            .as_ref()
            .self_tx
            .clone();
        let weak_ref = other.node_handle.clone();

        let (tx, rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(1);
        let node_self_tx = tx.clone();

        let thr = std::thread::spawn(move || {
            let mut node = SystemNode(TreeNode::new_under_weak(
                weak_ref,
                SystemNodeData::Scenario {
                    core: NodeLocalData {
                        parent_tx: Some(parent_tx),
                        children_rx: rx,
                        self_tx: node_self_tx,
                        requests_queue: VecDeque::new(),
                        node_id: std::thread::current().id(),
                    },
                    given_name,
                    tasks,
                },
            ));
            oneshot_tx.send(node.0.weak()).unwrap();

            node.execute();
        });

        (
            SystemNodeRunner {
                node_handle: oneshot_rx.recv().unwrap(),
            },
            thr,
        )
    }
}

#[derive(Clone, Debug)]
pub enum SystemNodeData {
    Scenario {
        given_name: String,
        tasks: Vec<NodeTaskInstruction>,
        core: NodeLocalData,
    },
    Simulated(NodeLocalData),
}

impl SystemNodeData {
    pub fn display_name(&self) -> String {
        match self {
            SystemNodeData::Scenario {
                given_name,
                tasks: _,
                core: _,
            } => given_name.clone(),
            SystemNodeData::Simulated(_) => std::thread::current()
                .name()
                .unwrap_or(&format!("{:?}", std::thread::current().id()))
                .to_owned(),
        }
    }
}

impl AsRef<NodeLocalData> for SystemNodeData {
    fn as_ref(&self) -> &NodeLocalData {
        match self {
            SystemNodeData::Scenario {
                given_name: _,
                tasks: _,
                core,
            } => core,
            SystemNodeData::Simulated(core) => core,
        }
    }
}

impl AsMut<NodeLocalData> for SystemNodeData {
    fn as_mut(&mut self) -> &mut NodeLocalData {
        match self {
            SystemNodeData::Scenario {
                given_name: _,
                tasks: _,
                core,
            } => core,
            SystemNodeData::Simulated(core) => core,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum NodeTaskInstruction {
    #[serde(rename = "cs")]
    CriticalSection { duration: u64 },
    #[serde(rename = "idle")]
    Idle { duration: u64 },
}
