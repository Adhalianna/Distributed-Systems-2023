use super::*;
use crossbeam_channel::Sender;
use std::thread::ThreadId;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct NodeId(ThreadId);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // WARN: What happens here is rather quite unstable!
        let dbg_fmt = format!("{:?}", self.0);
        let stripped_dbg = dbg_fmt
            .strip_prefix("ThreadId(")
            .expect("expected the ThreadId type to be represented in the Debug format as \"ThreadId(n)\"")
            .strip_suffix(")")
            .expect("expected the ThreadId type to be represented in the Debug format as \"ThreadId(n)\"");
        f.write_str(stripped_dbg)
    }
}

impl From<ThreadId> for NodeId {
    fn from(value: ThreadId) -> Self {
        Self(value)
    }
}

impl NodeId {
    pub fn for_this_thread() -> Self {
        Self(std::thread::current().id())
    }
}

/// A 'reference' to a node in the system that contains information
/// required to communicate with the node and distinguish it.
#[derive(Debug, Clone)]
pub struct SystemNodeRef {
    /// Used to get good log messages
    pub node_id: NodeId,
    /// Allows sending messages between threads
    pub self_tx: Sender<SystemMsg>,
}

impl Eq for SystemNodeRef {}
impl PartialEq for SystemNodeRef {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}

impl SystemNodeRef {
    /// # Panics
    pub fn send(&self, msg: SystemMsg) {
        self.self_tx.send(msg).unwrap()
    }
}

impl std::fmt::Display for SystemNodeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.node_id)
    }
}
