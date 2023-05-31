use std::{
    collections::{
        hash_map::{Entry, Keys},
        HashMap, HashSet,
    },
    default,
};

#[derive(PartialEq, Eq, Debug, Default, Clone, Copy, Hash)]
pub enum ResourceState {
    Requested,
    #[default]
    Free,
    InUse,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default, Clone, Hash)]
pub struct ProcessLabel(pub String);
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Default, Clone, Hash)]
pub struct ResourceLabel(pub String);

pub struct Network {
    pub nodes: Vec<Node>,
    pub controller: Controller,
}

/// A site running multiple processes and owning multiple resources
pub struct Node {
    /// Just like dictionary of dictionaries in Python
    pub status_table: HashMap<ProcessLabel, HashMap<ResourceLabel, ResourceState>>,
}

impl Node {
    pub fn processes(&self) -> impl Iterator<Item = &ProcessLabel> {
        self.status_table.keys()
    }
    pub fn resources(&self) -> Option<impl Iterator<Item = &ResourceLabel>> {
        self.status_table
            .values()
            .nth(0)
            .and_then(|map| Some(map.keys()))
    }
}

pub struct Controller {
    status_table: HashMap<ProcessLabel, HashMap<ResourceLabel, ResourceState>>,
    wait_for: HashMap<ProcessLabel, ProcessLabel>,
}

impl Controller {
    pub fn processes(&self) -> impl Iterator<Item = &ProcessLabel> {
        self.status_table.keys()
    }
    pub fn resources(&self) -> Option<impl Iterator<Item = &ResourceLabel>> {
        self.status_table
            .values()
            .nth(0)
            .and_then(|map| Some(map.keys()))
    }
    /// Update `Controller`'s status table with contents of other table
    pub fn collect_table(
        &mut self,
        table: HashMap<ProcessLabel, HashMap<ResourceLabel, ResourceState>>,
    ) {
        for (k, v) in table {
            let existing_entry = self.status_table.get_mut(&k);
            match existing_entry {
                Some(entry) => {
                    entry.extend(v);
                }
                None => {
                    self.status_table.insert(k, v);
                }
            }
        }
    }
    pub fn update_wait_for_graph(&mut self) {
        let mut resource_deps =
            HashMap::<&ResourceLabel, (Option<&ProcessLabel>, Option<&ProcessLabel>)>::new();
        for (proc, res_table) in &mut self.status_table {
            for (res, state) in res_table.iter_mut() {
                if *state == ResourceState::Requested {
                    match resource_deps.entry(res) {
                        Entry::Occupied(mut entry) => {
                            let mapping = entry.get_mut();
                            mapping.0 = Some(proc);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert((Some(proc), None));
                        }
                    }
                }
                if *state == ResourceState::InUse {
                    match resource_deps.entry(res) {
                        Entry::Occupied(_) => todo!(),
                        Entry::Vacant(_) => todo!(),
                    }
                }
            }
        }
    }

    /// Returns a chain of elements making up a cycle
    pub fn detect_cycles(&self) -> HashSet<(&ProcessLabel, &ResourceLabel)> {
        let mut visited = HashSet::new();
        for (proc, res) in &self.status_table {
            // find a busy resource
            if let Some(busy_res) = res.iter().find_map(|(_, state)| {
                if *state == ResourceState::Requested || *state == ResourceState::InUse {
                    Some(state)
                } else {
                    None
                }
            }) {
                // start DFS - status table is an adjacency matrix
                if visited.insert(proc) {
                    // if there was any cycle involving this process it has already been detected
                    break;
                }
            }
        }
        todo!()
    }
}

fn main() {
    println!("Hello, world!");
}
