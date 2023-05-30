use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use uuid::Uuid;

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
        let mut other_node = other.0.borrow_mut();

        match &self_node.parent {
            Some(_) => match other_node.parent {
                Some(_) => {
                    // we are not the root and the other node is not the root
                    self_node.parent = Some(other.clone());
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
        }
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
}

#[derive(Debug)]
pub struct Token;

pub enum SystemMsg {
    Token(Token),
    Request { from: String },
}

#[derive(Debug)]
pub struct SystemNodeData {
    // will be asserted by the (de)serialiazation format that this value is unique
    pub id: String,
    pub request_queue: VecDeque<String>,
    pub instructions: VecDeque<Instruction>,
    pub token: Option<Token>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Instruction {
    pub kind: TaskKind,
    /// How long will the task execute in the critical section
    pub duration: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum TaskKind {
    Idle,
    CriticalSection,
}

#[derive(Clone, Debug)]
pub struct SystemNode(pub Node<SystemNodeData>);

impl SystemNode {
    pub fn receive_msg(&mut self, msg: SystemMsg) {}
    pub fn execute_task(&mut self) -> Option<SystemMsg> {
        let node_data = self.0.data_mut();
        let next = node_data.instructions.iter_mut().nth(0);
        match next {
            Some(instr) => match instr.kind {
                TaskKind::Idle => {
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
                    }),
                },
            },
            None => None,
        }
    }
    pub fn add_request(&mut self, requesting: String) {
        let node_data = self.0.data_mut();
        node_data.request_queue.push_back(requesting);
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
    execution_duration: usize,
}

impl SystemDescription {
    /// Produces a vector of nodes ands take the execution duration out of the
    /// description.
    pub fn build_system(self) -> (Vec<SystemNode>, usize) {
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

        (final_nodes, self.execution_duration)
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

    let (mut nodes, loop_dur) = sys_description.build_system();

    for i in 0..loop_dur {
        for node in &mut nodes {}
    }
}
