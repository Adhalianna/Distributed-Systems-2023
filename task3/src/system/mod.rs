use crate::tree::{NodeData, TreeNode};
use crossbeam_channel::Receiver;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    thread::JoinHandle,
};

pub mod node_ref;
use node_ref::{NodeId, SystemNodeRef};
pub mod runner;
pub use runner::SystemNodeRunner;

pub enum SystemMsg {
    TokenMsg,
    /// Whenever a node requests a resource it actually expects a response
    /// that initially goes against the direction of the connections in the
    /// tree so passing the sender (tx end of channel) is required.
    RequestMsg {
        requesting: SystemNodeRef,
        /// The handle is used to reverse the direction of the link between the
        /// requesting and receiving node once the request will be answered
        node_handle: TreeNode<SystemNodeLocalData, SystemNodeLinkData>,
    },
}

#[derive(Clone)]
pub struct SystemNodeLocalData {
    self_ref: SystemNodeRef,
    /// The rx part of the channel which can be accessed from the senders
    /// cloned with the `SystemNodeRef` present in the `self_ref`. Any other
    /// node that received this node's `SystemNodeRef` will be sending to this
    /// receiver.
    network_rx: Receiver<SystemMsg>,
    /// Queue of deffered requests for access to the critical section.
    requests_queue: VecDeque<(
        SystemNodeRef,
        TreeNode<SystemNodeLocalData, SystemNodeLinkData>,
    )>,
    req_to_parent_pending: bool,
}

#[derive(Debug, Clone)]
pub struct SystemNodeLinkData {
    parent_ref: SystemNodeRef,
}

#[derive(Clone)]
pub struct SystemNode(TreeNode<SystemNodeLocalData, SystemNodeLinkData>);

impl SystemNode {
    pub fn idle_task(&self) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        std::thread::sleep(std::time::Duration::from_millis(
            800 + rng.gen_range(0..5000),
        ));
    }
    pub fn critical_section_task(&self) {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        std::thread::sleep(std::time::Duration::from_millis(
            800 + rng.gen_range(0..5000),
        ));
    }

    pub fn execute(&mut self) {
        //extract and own the data that will make logs much more clear
        let display_name = self.0.inner().data().self_ref.to_string();
        //might as well keep the reference to self on thread's stack too
        let self_ref = self.0.inner().data().self_ref.clone();

        if self.0.is_root() {
            println!("Node {display_name} is the root");
        }
        println!("Node {display_name} is ready to work");

        loop {
            // First start with something that does not require critical section.
            // While the idle task is performed the node cannot pass on requests
            // for access to the CS but that is fine, it will respond to those
            // eventually. The requests are stored within the channels used by the
            // nodes to connect with each other.
            self.idle_task();
            println!("Node {display_name} finished its idle task");

            // now that we want access to the CS we need to issue a request
            self_ref.send(SystemMsg::RequestMsg {
                requesting: self_ref.clone(),
                node_handle: self.0.clone(),
            });
            println!("Node {display_name} sent a request for access to the critical section");

            // To be able to proceed with the request we need the token so
            // we start listening to the messages on the network. In case we
            // were already a root node and as such should already have a token it
            // will be resolved from the messages anyway as a request to self
            // will be sent over the network like any other request.
            'listen: loop {
                // This implementation attempts to do as much as possible on receiving
                // the `RequestMsg`. The reason is: as order in which messages arrive is
                // random it may happen that the receiving node should have been changed
                // into a root node before it proceeds to handle any other incoming request
                // as that changes the way the request is handled. This is why as soon as
                // the node realizes that it is a root it interpets a request to self as
                // if it also included a token message.

                // NOTE: Currently this deadlocks after the first of run nodes requesting for
                // access to the CS! Maybe the way the mutex over the node is handled is wrong.
                // At a first glance at least, it seems like the problem could reside in the way
                // this application emulates a network using threads.

                match self.recv() {
                    SystemMsg::RequestMsg {
                        requesting,
                        node_handle,
                    } => {
                        self.push_request(requesting.clone(), node_handle.clone());

                        let mut self_mx = self.0.inner();
                        match self_mx.deref_mut() {
                            crate::tree::Node::Child(c) => {
                                if !c.data.req_to_parent_pending {
                                    c.data.req_to_parent_pending = true;
                                    c.parent.data.parent_ref.send(SystemMsg::RequestMsg {
                                        requesting: self_ref.clone(),
                                        node_handle: self.0.clone(),
                                    });
                                }
                            }
                            crate::tree::Node::Root(_) => {
                                std::mem::drop(self_mx);
                                if self.handle_received_token(self_ref.clone()) {
                                    break 'listen;
                                }
                            }
                        }
                    }
                    SystemMsg::TokenMsg => {
                        if self.handle_received_token(self_ref.clone()) {
                            break 'listen;
                        }
                    }
                }
            }
        }
    }

    fn handle_received_token(&mut self, self_ref: SystemNodeRef) -> bool {
        self.0.inner().data_mut().req_to_parent_pending = false;
        if let Some(next) = self.0.inner().data_mut().requests_queue.pop_front() {
            if next.0 == self_ref {
                println!("Node {self_ref} starts its task within critical section");
                self.critical_section_task();
                println!("Node {self_ref} finishes its task within critical section");

                if let Some(next) = self.0.inner().data_mut().requests_queue.pop_front() {
                    self.0.inner().deref_mut().make_child_of(
                        &next.1,
                        SystemNodeLinkData {
                            parent_ref: next.0.clone(),
                        },
                    );
                    println!("Node {self_ref} became a child of {}", next.0);
                    next.0.send(SystemMsg::TokenMsg);
                    if !self.0.inner().data().requests_queue.is_empty() {
                        self.0.inner().data_mut().req_to_parent_pending = true;
                        next.0.send(SystemMsg::RequestMsg {
                            requesting: self_ref.clone(),
                            node_handle: self.0.clone(),
                        })
                    }
                }
                return true;
            } else {
                self.0.inner().deref_mut().make_child_of(
                    &next.1,
                    SystemNodeLinkData {
                        parent_ref: next.0.clone(),
                    },
                );
                println!("Node {self_ref} became a child of {}", next.0);
                next.0.send(SystemMsg::TokenMsg);
                if !self.0.inner().data().requests_queue.is_empty() {
                    self.0.inner().data_mut().req_to_parent_pending = true;
                    next.0.send(SystemMsg::RequestMsg {
                        requesting: self_ref.clone(),
                        node_handle: self.0.clone(),
                    })
                }
            }
            false
        } else {
            false
        }
    }

    fn push_request(
        &mut self,
        req_ref: SystemNodeRef,
        node_handle: TreeNode<SystemNodeLocalData, SystemNodeLinkData>,
    ) {
        let mut mx = self.0.inner();
        let queue = &mut mx.deref_mut().data_mut().requests_queue;
        queue.push_back((req_ref, node_handle));
    }
    fn recv(&self) -> SystemMsg {
        self.0.inner().data().network_rx.recv().unwrap()
    }
}
