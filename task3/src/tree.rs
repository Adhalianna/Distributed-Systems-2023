use std::{
    ops::DerefMut,
    sync::{Arc, Mutex, MutexGuard},
};

/// Common behaviour among child and root nodes - access to the associated data
pub trait NodeData<N> {
    /// The data is boxed so that it stays in a single place in memory as a
    /// heap allocation even after ownership of the TreeNode is altered.
    fn data(&self) -> &Box<N>;
    fn data_mut(&mut self) -> &mut Box<N>;
}

impl<N> NodeData<N> for RootNode<N> {
    fn data(&self) -> &Box<N> {
        &self.data
    }
    fn data_mut(&mut self) -> &mut Box<N> {
        &mut self.data
    }
}

impl<N, L> NodeData<N> for ChildNode<N, L> {
    fn data(&self) -> &Box<N> {
        &self.data
    }
    fn data_mut(&mut self) -> &mut Box<N> {
        &mut self.data
    }
}

impl<N, L> NodeData<N> for Node<N, L> {
    fn data(&self) -> &Box<N> {
        match self {
            Node::Child(n) => n.data(),
            Node::Root(n) => n.data(),
        }
    }
    fn data_mut(&mut self) -> &mut Box<N> {
        match self {
            Node::Child(n) => n.data_mut(),
            Node::Root(n) => n.data_mut(),
        }
    }
}

/// `N` is the data associated with the node, a weight or a label of the node.
/// `L` is the data type of information associated with the directed edge
/// pointing to the node's parent.
pub enum Node<N, L> {
    Child(ChildNode<N, L>),
    Root(RootNode<N>),
}

#[derive(Clone)]
pub struct RootNode<N> {
    pub data: Box<N>,
}

pub struct ChildNode<N, L> {
    pub parent: NodeLink<N, L>,
    pub data: Box<N>,
}

/// The direction of the link (`points_to` field) can be changed separately
/// form the data associated with it.
pub struct NodeLink<N, L> {
    points_to: TreeNode<N, L>,
    pub data: Box<L>,
}

impl<N, L> NodeLink<N, L> {
    pub fn directed_to(mut self, other: &TreeNode<N, L>) -> Self {
        self.points_to = other.clone();
        self
    }
}

/// A [`Node`] put behind an `Arc` to achieve shared ownership of the data.
pub struct TreeNode<N, L>(Arc<Mutex<Node<N, L>>>);

// derive does not work well for this particular case
impl<N, L> Clone for TreeNode<N, L> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<N, L> TreeNode<N, L> {
    pub fn new(data: N) -> Self {
        Self(Arc::new(Mutex::new(Node::Root(RootNode {
            data: Box::new(data),
        }))))
    }
    pub fn new_under(node: &TreeNode<N, L>, with_data: N, and_link_data: L) -> Self {
        let link = NodeLink {
            points_to: node.clone(),
            data: Box::new(and_link_data),
        };
        Self(Arc::new(Mutex::new(Node::Child(ChildNode {
            parent: link,
            data: Box::new(with_data),
        }))))
    }
    pub fn is_root(&self) -> bool {
        match *self.0.as_ref().lock().unwrap() {
            Node::Child(_) => false,
            Node::Root(_) => true,
        }
    }
    /// Get access to the inner enum type [`Node`]. Useful in match statements.
    pub fn inner(&self) -> MutexGuard<Node<N, L>> {
        self.0.as_ref().lock().unwrap()
    }
}

impl<N: Clone, L> TreeNode<N, L> {
    pub fn make_child_of(&mut self, other: &TreeNode<N, L>, with_link_data: L) {
        self.inner()
            .deref_mut()
            .make_child_of(other, with_link_data)
    }
}

impl<N: Clone> RootNode<N> {
    pub fn make_child_of<L>(&self, other: &TreeNode<N, L>, with_link_data: L) -> Node<N, L> {
        let mut other_mx = other.inner();
        match other_mx.deref_mut() {
            Node::Child(_) => {
                *other_mx = Node::Root(RootNode {
                    data: other_mx.data().clone(),
                });
            }
            Node::Root(_) => {
                return Node::Root(RootNode {
                    data: self.data.clone(),
                });
            }
        };

        let link = NodeLink {
            points_to: other.clone(),
            data: Box::new(with_link_data),
        };
        Node::Child(ChildNode {
            parent: link,
            data: self.data.clone(),
        })
    }
}

impl<N: Clone, L> ChildNode<N, L> {
    pub fn make_child_of(&self, other: &TreeNode<N, L>, with_link_data: L) -> ChildNode<N, L> {
        Self {
            parent: NodeLink {
                points_to: other.clone(),
                data: Box::new(with_link_data),
            },
            data: self.data.clone(),
        }
    }
}

impl<N: Clone, L> Node<N, L> {
    pub fn make_child_of(&mut self, other: &TreeNode<N, L>, with_link_data: L) {
        match self {
            Node::Child(c) => {
                let newer_child = c.make_child_of(other, with_link_data);
                *self = Self::Child(newer_child);
            }
            Node::Root(r) => {
                let new_child = r.make_child_of(other, with_link_data);
                *self = new_child;
            }
        }
    }
}
