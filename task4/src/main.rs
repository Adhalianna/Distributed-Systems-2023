use std::collections::{hash_map::Entry, HashMap};

#[derive(PartialEq, Eq, Debug, Default, Clone, Copy, Hash)]
pub enum ResourceState {
    Requested,
    #[default]
    Free,
    InUse,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Default, Clone, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ProcessLabel(pub String);

impl std::fmt::Debug for ProcessLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Default, Clone, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ResourceLabel(pub String);

impl std::fmt::Debug for ResourceLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Network {
    pub nodes: Vec<Node>,
    /// Assumed that it has a direct access to all other nodes
    pub controller: Controller,
}

/// A site running multiple processes and owning multiple resources
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Node {
    pub resources: HashMap<ResourceLabel, ProcessLabel>,
    pub processes: HashMap<ProcessLabel, Vec<ResourceLabel>>,
}

#[derive(Default)]
pub struct Controller {
    status_table: HashMap<ProcessLabel, HashMap<ResourceLabel, ResourceState>>,
    /// A wait-for-graph represented as an adjacency list
    waits_for: HashMap<ProcessLabel, Vec<ProcessLabel>>,
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
    pub fn collect_tables(
        &mut self,
        resources: &HashMap<ResourceLabel, ProcessLabel>,
        processes: &HashMap<ProcessLabel, Vec<ResourceLabel>>,
    ) {
        for (proc, res) in processes {
            match self.status_table.entry(proc.clone()) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();
                    entry.extend(
                        res.into_iter()
                            .map(|res| (res.clone(), ResourceState::Requested)),
                    );
                }
                Entry::Vacant(entry) => {
                    entry.insert(
                        res.into_iter()
                            .map(|res| (res.clone(), ResourceState::Requested))
                            .collect::<HashMap<ResourceLabel, ResourceState>>(),
                    );
                }
            }
        }
        for (res, proc) in resources {
            match self.status_table.entry(proc.clone()) {
                Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();
                    match entry.get_mut(res) {
                        Some(res_state) => {
                            *res_state = ResourceState::InUse;
                        }
                        None => {
                            entry.insert(res.clone(), ResourceState::InUse);
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let mut map = HashMap::with_capacity(1);
                    map.insert(res.clone(), ResourceState::InUse);
                    entry.insert(map);
                }
            }
        }
    }
    pub fn update_wait_for_graph(&mut self) -> Result<(), String> {
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
                        Entry::Occupied(mut entry) => {
                            let mapping = entry.get_mut();
                            mapping.1 = Some(proc);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert((None, Some(proc)));
                        }
                    }
                }
            }
        }
        let mut waits_for = HashMap::<ProcessLabel, Vec<ProcessLabel>>::new();
        for (_, (p1, p2)) in resource_deps {
            if let (Some(p1), Some(p2)) = (p1, p2) {
                match waits_for.entry(p1.clone()) {
                    Entry::Occupied(mut entry) => {
                        let waits_for = entry.get_mut();
                        waits_for.push(p2.clone())
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(vec![p2.clone()]);
                    }
                }
            }
        }

        self.waits_for = waits_for;

        Ok(())
    }

    /// Collects all cycles found in the graph
    pub fn detect_all_cycles<'a>(&'a self) -> Vec<Vec<&'a ProcessLabel>> {
        let mut reslt = Vec::new();

        let mut visited = HashMap::<&ProcessLabel, DfsSearchStatus>::new();
        visited.extend(
            self.waits_for
                .keys()
                .map(|proc| (proc, DfsSearchStatus::NotVisited)),
        );

        for p in self.processes() {
            if visited.get(p).copied() == Some(DfsSearchStatus::NotVisited) {
                let mut tracked = Vec::new();
                tracked.push(p);
                visited.insert(p, DfsSearchStatus::Tracked);
                self.detect_cycle(&mut tracked, &mut visited, &mut reslt);
            }
        }
        reslt
    }

    /// Returns optionally a chain of elements making up a cycle
    fn detect_cycle<'a>(
        &'a self,
        tracked: &mut Vec<&'a ProcessLabel>,
        visited: &mut HashMap<&'a ProcessLabel, DfsSearchStatus>,
        result: &mut Vec<Vec<&'a ProcessLabel>>,
    ) {
        let processes = self.waits_for.get(tracked.iter().last().unwrap());
        if let Some(processes) = processes {
            for p in processes {
                let process_track_status = visited.get(p).copied();
                if process_track_status == Some(DfsSearchStatus::Tracked) {
                    result.push(tracked.clone());
                    tracked.clear();
                    tracked.push(p);
                } else if process_track_status == Some(DfsSearchStatus::NotVisited) {
                    tracked.push(p);
                    visited.insert(p, DfsSearchStatus::Tracked);
                    return self.detect_cycle(tracked, visited, result);
                }
            }
        }
        if let Some(top) = tracked.iter().last() {
            visited.insert(*top, DfsSearchStatus::Done);
            tracked.pop();
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
enum DfsSearchStatus {
    // coloring
    NotVisited,
    Tracked,
    Done,
}

fn main() {
    let filename = std::env::args()
        .nth(1)
        .expect("expected a filename as an input");
    println!("Reading from file {filename}");
    let file = std::fs::File::open(&filename).expect("failed to open the file");
    let nodes =
        serde_json::from_reader(file).expect("failed to deserialize into a vector of nodes");

    let mut net = Network {
        nodes,
        controller: Controller::default(),
    };

    for node in net.nodes {
        net.controller
            .collect_tables(&node.resources, &node.processes);
    }

    net.controller
        .update_wait_for_graph()
        .expect("provided data was insufficient to build a complete wait-for-graph");

    println!("Recorded wait-for-graph:");
    println!("{:?}", &net.controller.waits_for);

    println!("Recorded status table:");
    println!("{:?}", &net.controller.status_table);

    let cycles = net.controller.detect_all_cycles();
    if cycles.is_empty() {
        println!("No cycles found");
    } else {
        println!("Found cycles causing deadlocks.");
        for c in &cycles {
            println!("Found deadlock invloving nodes: {c:?}");
        }
    }
}
