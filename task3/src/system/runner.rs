use super::*;

/// A proxy that executes the `SystemNode` on its own thread and keeps a
/// a reference to it that can be used to connect other nodes to the tree
/// from the thread that created the runner.
pub struct SystemNodeRunner {
    node_handle: SystemNode,
    /// The thread join handle can be used to detect if the node running thread
    /// panicked.
    pub join_handle: JoinHandle<()>,
}

impl SystemNodeRunner {
    pub fn new() -> Self {
        let (node_self_tx, network_rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(0);

        let thread_handle = std::thread::spawn(move || {
            let mut node = SystemNode(TreeNode::<SystemNodeLocalData, SystemNodeLinkData>::new(
                SystemNodeLocalData {
                    self_ref: SystemNodeRef {
                        node_id: NodeId::for_this_thread(),
                        self_tx: node_self_tx,
                    },
                    network_rx,
                    requests_queue: VecDeque::new(),
                    req_to_parent_pending: false,
                },
            ));

            oneshot_tx
                .send(node.clone())
                .expect("failed to send back to the runner a handle to the system node");

            node.execute();
        });

        Self {
            node_handle: oneshot_rx
                .recv()
                .expect("failed to receive a handle from the node that was run on a new thread"),
            join_handle: thread_handle,
        }
    }

    pub fn new_under(other_runner: &SystemNodeRunner) -> Self {
        let (node_self_tx, network_rx) = crossbeam_channel::unbounded();
        let (oneshot_tx, oneshot_rx) = crossbeam_channel::bounded(0);

        let other_node = other_runner.node_handle.0.clone();
        let parent_ref = other_runner.node_handle.0.inner().data().self_ref.clone();

        let thread_handle = std::thread::spawn(move || {
            let mut node = SystemNode(
                TreeNode::<SystemNodeLocalData, SystemNodeLinkData>::new_under(
                    &other_node,
                    SystemNodeLocalData {
                        self_ref: SystemNodeRef {
                            node_id: NodeId::for_this_thread(),
                            self_tx: node_self_tx,
                        },
                        network_rx,
                        requests_queue: VecDeque::new(),
                        req_to_parent_pending: false,
                    },
                    SystemNodeLinkData { parent_ref },
                ),
            );

            oneshot_tx
                .send(node.clone())
                .expect("failed to send back to the runner a handle to the system node");

            node.execute();
        });

        Self {
            node_handle: oneshot_rx
                .recv()
                .expect("failed to receive a handle from the node that was run on a new thread"),
            join_handle: thread_handle,
        }
    }
}
