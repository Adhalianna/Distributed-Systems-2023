use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(transparent)]
pub struct Task2StudyCaseInstructions(pub HashMap<String, NodeInstructions>);

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(transparent)]
pub struct NodeInstructions(pub Vec<NodeTaskInstruction>);

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type")]
pub enum NodeTaskInstruction {
    #[serde(rename = "cs")]
    CriticalSection { duration: u64 },
    #[serde(rename = "idle")]
    Idle { duration: u64 },
}
