use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak},
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TreeNode<T>(pub Arc<RwLock<ChildOrRoot<T>>>);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum ChildOrRoot<T> {
    Node(ChildNode<T>),
    Root(Root<T>),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Root<T> {
    pub data: T,
}

/// A node within an inverted tree that has a parent. It shares the ownership
/// of the parent node with other nodes and only the parent can be accessed
/// from it. Using [`Arc`] and [`RwLock`] makes the structure thread-safe.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ChildNode<T> {
    pub parent: Arc<RwLock<ChildOrRoot<T>>>,
    pub data: T,
}

pub type WeakNodeRef<T> = Weak<RwLock<ChildOrRoot<T>>>;

impl<T> TreeNode<T> {
    pub fn new_tree(root_data: T) -> Self {
        Self(Arc::new(RwLock::new(ChildOrRoot::Root(Root {
            data: root_data,
        }))))
    }
    // The code probably could be organized in such way that leaf nodes are not
    // stored behind `Arc<RwLock<_>>` unneccessarily but it would explode the
    // code complexity.
    pub fn new_under(&self, data: T) -> Self {
        Self(Arc::new(RwLock::new(ChildOrRoot::Node(ChildNode {
            parent: self.0.clone(),
            data,
        }))))
    }
    /// # Panics
    pub fn new_under_weak(weak: WeakNodeRef<T>, data: T) -> Self {
        Self(Arc::new(RwLock::new(ChildOrRoot::Node(ChildNode {
            parent: weak.clone().upgrade().unwrap(),
            data,
        }))))
    }
    pub fn weak(&self) -> WeakNodeRef<T> {
        Arc::downgrade(&(self.0))
    }
    pub fn is_root(&self) -> bool {
        match self.0.as_ref().read().expect("RwLock poisoned").deref() {
            ChildOrRoot::Node(_) => false,
            ChildOrRoot::Root(_) => true,
        }
    }
    /// # Panics
    pub fn read(&self) -> RwLockReadGuard<ChildOrRoot<T>> {
        self.0.as_ref().read().expect("RwLock poisoned")
    }
    /// # Panics
    pub fn write(&self) -> RwLockWriteGuard<ChildOrRoot<T>> {
        self.0.as_ref().write().expect("RwLock poisoned")
    }
}

impl<T: Clone> TreeNode<T> {
    pub fn redirect_root_to(&self) {
        ChildOrRoot::redirect_root_to(&self.0)
    }
}

impl<T> ChildOrRoot<T> {
    pub fn data(&self) -> &T {
        match self {
            ChildOrRoot::Node(n) => &n.data,
            ChildOrRoot::Root(n) => &n.data,
        }
    }
    pub fn data_mut(&mut self) -> &mut T {
        match self {
            ChildOrRoot::Node(n) => &mut n.data,
            ChildOrRoot::Root(n) => &mut n.data,
        }
    }
}

impl<T: Clone> ChildOrRoot<T> {
    /// Change the relation between parent and child. The passed in node is
    /// expected to be a child in this relation. After transformation it will
    /// become the parent and the root. The operation can only happen if the
    /// parent was a root.
    ///
    /// # Panics
    /// It will panic in situations which would cause violation of the tree
    /// property

    // This will be the only operation needed for the algorithm and it will happen
    // only when a node receives a token.
    pub fn redirect_root_to(node: &Arc<RwLock<ChildOrRoot<T>>>) {
        let mut parent_guard = node.as_ref().write().expect("RwLock has been poisoned");
        let parent_inner = parent_guard.deref_mut();
        match parent_inner {
            ChildOrRoot::Node(_) => {
                // Parent had another parent
                panic!("cannot change the direction of connection between the two nodes as it will violate the tree property");
            }
            ChildOrRoot::Root(parent) => {
                // Parent was root and now the node will become a root
                let mut node_guard = node.as_ref().write().expect("RwLock has been poisoned");
                let node_inner = node_guard.deref_mut();
                let mut new_root = ChildOrRoot::Root(Root {
                    data: match node_inner {
                        ChildOrRoot::Node(inner) => inner.data.clone(),
                        ChildOrRoot::Root(_) => panic!("there were two roots in the tree!"),
                    },
                });
                std::mem::swap(node_inner, &mut new_root);
                std::mem::drop(node_inner);

                let mut new_child = ChildOrRoot::Node(ChildNode {
                    parent: node.clone(),
                    data: parent.data.clone(),
                });
                std::mem::swap(parent_inner, &mut new_child);
            }
        }
    }
}
