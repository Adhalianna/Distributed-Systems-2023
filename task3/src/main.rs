use colored::*;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    mem::drop,
    rc::Rc,
};

#[derive(Debug)]
pub struct NodeInner<T> {
    parent: Option<Node<T>>,
    data: T,
}

#[derive(Debug)]
pub struct Node<T>(Rc<RefCell<NodeInner<T>>>);

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Node<T> {
    pub fn root(data: T) -> Self {
        Self(Rc::new(RefCell::new(NodeInner { parent: None, data })))
    }
    pub fn produce_child(&self, data: T) -> Self {
        Self(Rc::new(RefCell::new(NodeInner {
            parent: Some(self.clone()),
            data,
        })))
    }
    pub fn make_child_of(&mut self, other: &Self) {
        let mut self_node = self.0.borrow_mut();
        self_node.parent = Some(other.clone());
    }
    pub fn make_root(&mut self) {
        let mut self_node = self.0.borrow_mut();
        self_node.parent = None;
    }
    pub fn invert_relation_with(&mut self, other: &mut Self) {
        let mut self_node = self.0.borrow_mut();
        // hacky but works good as no one else has any other reason to modify the `parent`
        let mut other_node = unsafe { other.0.as_ptr().as_mut().unwrap() };

        match &self_node.parent {
            Some(_) => match other_node.parent {
                Some(_) => {
                    // we are not the root and the other node is not the root
                    self_node.parent = Some(other.clone());
                    other_node.parent = None;
                }
                None => {
                    // we are not the root but the other node is the root
                    self_node.parent = None;
                    other_node.parent = Some(self.clone());
                }
            },
            None => {
                // we are the root
                self_node.parent = Some(other.clone());
                other_node.parent = None;
            }
        };
    }
    pub fn data(&self) -> &T {
        //hacky but the normal borrow rules should protect us well enough in the single threaded context
        &unsafe { self.0.try_borrow_unguarded() }.unwrap().data
    }
    pub fn data_mut(&mut self) -> &mut T {
        //also hacky
        let ref_mut: &mut NodeInner<T> = unsafe { self.0.as_ptr().as_mut().unwrap() };
        &mut ref_mut.data
    }
    pub fn is_root(&self) -> bool {
        self.0.borrow().parent.is_none()
    }
}

#[derive(Debug)]
pub struct Token;

pub enum SystemMsg {
    Token(Token),
    Request { from: String, handle: SystemNode },
}

#[derive(Debug)]
pub struct SystemNodeData {
    /// will be asserted by the (de)serialiazation format that this value is unique
    pub id: String,
    pub request_queue: VecDeque<(String, SystemNode)>,
    pub instructions: VecDeque<Instruction>,
    pub token: Option<Token>,
    pub is_req_sent: bool,
    /// additional variable introduced to keep the state consistent between iterations
    pub self_req_issued: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Instruction {
    pub kind: TaskKind,
    /// How long will the task execute in the critical section
    pub duration: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Idle,
    CriticalSection,
}

#[derive(Clone, Debug)]
pub struct SystemNode(pub Node<SystemNodeData>);

impl SystemNode {
    pub fn is_done(&self) -> bool {
        let node_data = self.0.data();
        node_data.request_queue.is_empty() && node_data.instructions.is_empty()
    }
    pub fn is_root(&self) -> bool {
        self.0.is_root()
    }
    pub fn id(&self) -> String {
        self.0.data().id.clone()
    }
    pub fn queue_status(&mut self) -> &[(String, SystemNode)] {
        let node_data = self.0.data_mut();
        node_data.request_queue.make_contiguous()
    }
    pub fn iterate(&mut self) {
        let maybe_msg = self.execute_task();
        let self_id = self.id();

        match maybe_msg {
            Some(msg) => {
                let node = self.0 .0.borrow();
                let parent = node.parent.clone();
                match parent {
                    Some(parent) => match msg {
                        SystemMsg::Token(_) => {
                            panic!("a non-root node should not be producing a token");
                        }
                        SystemMsg::Request { from, handle } => {
                            drop(node);

                            let node_data = self.0.data_mut();

                            if from == self_id && !node_data.self_req_issued {
                                println!(
                                    "{}",
                                    format!("--- Node {self_id} produced a request").bright_blue()
                                );

                                node_data
                                    .request_queue
                                    .push_back((from.clone(), handle.clone()));
                                node_data.self_req_issued = true;
                            }
                            if !node_data.is_req_sent {
                                node_data.is_req_sent = true;
                                drop(node_data);

                                SystemNode(parent).recv_msg(SystemMsg::Request { from, handle })
                            }
                        }
                    },
                    None => {
                        match msg {
                            SystemMsg::Token(t) => {
                                // drop what we do not need and reborrow
                                drop(node);

                                println!(
                                    "{}",
                                    format!("--- Node {self_id} relieves a token").bright_blue()
                                );

                                let node_data = self.0.data_mut();
                                let next_req = node_data.request_queue.pop_front();
                                match next_req {
                                    // pass on the token and stop being a root
                                    Some((name, mut handle)) => {
                                        handle.recv_msg(SystemMsg::Token(t));

                                        drop(node_data);
                                        self.0.invert_relation_with(&mut handle.0);
                                        let node_data = self.0.data_mut();

                                        if let Some(_) = node_data.request_queue.iter().nth(0) {
                                            handle.recv_msg(SystemMsg::Request {
                                                from: node_data.id.clone(),
                                                handle: self.clone(),
                                            })
                                        }
                                    }
                                    None => {
                                        // nothing to do
                                    }
                                }
                            }
                            SystemMsg::Request { from: _, handle: _ } => { /* do nothing, resolve in the next iteration */
                            }
                        }
                    }
                }
            }
            None => {
                // nothing to do
            }
        }
    }
    pub fn recv_msg(&mut self, msg: SystemMsg) {
        match msg {
            SystemMsg::Token(t) => {
                let node_data = self.0.data_mut();
                node_data.token = Some(t);
                node_data.is_req_sent = false;

                if let Some(next_req) = node_data.request_queue.pop_front() {
                    let (name, mut handle) = next_req;

                    if name == node_data.id {
                        /* do nothing, keep the token to execute the task in the next iter*/
                    } else {
                        println!(
                            "{}",
                            format!(
                                "--- Node {} received a token and passed it further to {name}",
                                node_data.id
                            )
                            .bright_blue()
                        );

                        drop(node_data); //drop
                        self.0.invert_relation_with(&mut handle.0);
                        let node_data = self.0.data_mut(); //reborrow

                        handle.recv_msg(SystemMsg::Token(node_data.token.take().unwrap()));
                        println!(
                            "{}",
                            format!("--- Node {name} received a token and became a root")
                                .bright_blue()
                        );

                        if let Some(_) = node_data.request_queue.iter().nth(0) {
                            handle.recv_msg(SystemMsg::Request {
                                from: node_data.id.clone(),
                                handle: self.clone(),
                            })
                        }
                    }
                }
            }
            SystemMsg::Request { from, handle } => {
                let node_data = self.0.data_mut();
                node_data.request_queue.push_back((from, handle));
            }
        }
    }
    pub fn execute_task(&mut self) -> Option<SystemMsg> {
        let self_id = self.id();
        let node_data = self.0.data_mut();
        let next = node_data.instructions.iter_mut().nth(0);
        match next {
            Some(instr) => match instr.kind {
                TaskKind::Idle => {
                    println!(
                        "{}",
                        format!("--- Node {self_id} executes the idle task").bright_blue()
                    );
                    if instr.duration > 1 {
                        instr.duration -= 1;
                        None
                    } else {
                        node_data.instructions.pop_front();
                        None
                    }
                }
                TaskKind::CriticalSection => match node_data.token {
                    Some(_) => {
                        println!(
                            "{}",
                            format!("--- Node {self_id} executes in the critical section")
                                .bright_blue()
                        );
                        if instr.duration > 1 {
                            instr.duration -= 1;
                            None
                        } else {
                            node_data.instructions.pop_front();
                            Some(SystemMsg::Token(Token))
                        }
                    }
                    None => Some(SystemMsg::Request {
                        from: node_data.id.clone(),
                        handle: self.clone(),
                    }),
                },
            },
            None => None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SystemNodeDescription {
    pub instructions: VecDeque<Instruction>,
    pub parent: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SystemDescription {
    nodes: HashMap<String, SystemNodeDescription>,
}

impl SystemDescription {
    /// Produces a vector of nodes and takes the execution duration out of the
    /// description.
    pub fn build_system(self) -> Vec<SystemNode> {
        let relations: HashMap<String, Option<String>> = self
            .nodes
            .iter()
            .map(|(name, data)| (name.to_owned(), data.parent.to_owned()))
            .collect();

        let nodes: HashMap<String, SystemNode> = self
            .nodes
            .into_iter()
            .map(|(name, data)| {
                (
                    name.clone(),
                    SystemNode(Node::root(SystemNodeData {
                        id: name,
                        request_queue: VecDeque::new(),
                        instructions: data.instructions,
                        token: None,
                        is_req_sent: false,
                        self_req_issued: false,
                    })),
                )
            })
            .collect();

        let mut final_nodes = Vec::new();

        for (rel_target, maybe_parent) in relations {
            let target_node = nodes.get(&rel_target).unwrap();
            match maybe_parent {
                Some(parent) => {
                    let target_parent = nodes.get(&parent).unwrap();
                    let mut target_clone = target_node.0.clone();
                    target_clone.make_child_of(&target_parent.0);

                    final_nodes.push(SystemNode(target_clone));
                }
                None => {
                    let mut target_clone = target_node.0.clone();
                    let data = target_clone.data_mut();
                    data.token = Some(Token);

                    final_nodes.push(SystemNode(target_clone))
                }
            }
        }

        final_nodes
    }
}

fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an input");
    println!("Reading from file {filename}");
    let file = std::fs::File::open(&filename).expect("failed to open the file");
    let sys_description: SystemDescription =
        serde_json::from_reader(file).expect("failed to deserialize");

    let mut nodes = sys_description.build_system();

    let mut i: usize = 1;
    loop {
        i += 1;
        println!("{}", format!("ITERATION {}", i).yellow());
        for node in &mut nodes {
            node.iterate();
            println!(
                "{}",
                format!(
                    "--- Node \"{}\" queue: {:?}",
                    node.id(),
                    node.queue_status()
                        .iter()
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>()
                )
                .bright_yellow()
            );
            drop(node);
        }
        if nodes.iter().all(|n| n.is_done()) {
            println!("{}", format!("FINISHED").yellow());

            break;
        }
    }
}
