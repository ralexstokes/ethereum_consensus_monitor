use crate::chain::Coordinate;
use eth2::types::{Epoch, Hash256, Slot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize)]
pub struct ProtoNode {
    slot: Slot,
    root: Hash256,
    parent: Option<usize>,
    weight: u64,
    best_descendant: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct ProtoArray {
    finalized_epoch: Epoch,
    nodes: Vec<ProtoNode>,
    indices: HashMap<Hash256, usize>,
}

#[derive(Clone, Default)]
pub struct ForkChoice {
    pub tree: Arc<Mutex<ForkChoiceNode>>,
    slots_per_epoch: u64,
}

impl ForkChoice {
    pub fn new(block: Coordinate, slots_per_epoch: u64) -> Self {
        Self {
            tree: Arc::new(Mutex::new(ForkChoiceNode {
                slot: block.slot,
                root: block.root,
                children: vec![],
                weight: 0,
                is_canonical: false,
            })),
            slots_per_epoch,
        }
    }

    pub fn update(&mut self, proto_array: ProtoArray) {
        let finalized_slot = proto_array.finalized_epoch.start_slot(self.slots_per_epoch);
        let node_count = proto_array.nodes.len();
        match ForkChoiceNode::try_from((proto_array, finalized_slot)) {
            Ok(..) => {
                log::trace!(
                    "updated proto array starting at {} with {} nodes",
                    finalized_slot,
                    node_count,
                );
                // let _ = self.tree.write().map(|mut inner| {
                //     *inner = fork_choice;
                //     Some(())
                // });
            }
            Err(err) => log::warn!("failed to update fork choice: {}", err),
        }
    }
}

#[derive(Serialize, Default, Debug)]
pub struct ForkChoiceNode {
    slot: Slot,
    root: Hash256,
    weight: u64,
    children: Vec<ForkChoiceNode>,
    is_canonical: bool,
}

#[derive(Debug)]
pub enum ForkChoiceError {
    MissingFinalizedNode,
}

impl std::fmt::Display for ForkChoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFinalizedNode => write!(
                f,
                "missing finalized node in provided data (check API response?)"
            ),
        }
    }
}

impl TryFrom<(ProtoArray, Slot)> for ForkChoiceNode {
    type Error = ForkChoiceError;

    fn try_from((proto_array, finalized_slot): (ProtoArray, Slot)) -> Result<Self, Self::Error> {
        let mut parent_index_to_children: HashMap<usize, Vec<Hash256>> = HashMap::new();

        let first_node = &proto_array.nodes[0];
        let best_descendant = first_node.best_descendant;

        let mut finalized_root = None;
        for node in proto_array.nodes.iter() {
            if node.slot < finalized_slot {
                continue;
            }
            if node.slot == finalized_slot {
                finalized_root = Some(node.root);
            }

            if let Some(parent_index) = node.parent {
                let children = parent_index_to_children.entry(parent_index).or_default();
                children.push(node.root);
            }
        }
        finalized_root
            .ok_or(ForkChoiceError::MissingFinalizedNode)
            .map(|root| {
                build_fork_choice_tree(
                    &root,
                    &parent_index_to_children,
                    &proto_array,
                    best_descendant,
                )
            })
    }
}

fn build_fork_choice_tree(
    root: &Hash256,
    parent_index_to_children: &HashMap<usize, Vec<Hash256>>,
    proto_array: &ProtoArray,
    best_descendant: Option<u64>,
) -> ForkChoiceNode {
    let index = proto_array.indices[root];
    let proto_node = &proto_array.nodes[index];
    let is_canonical = match (best_descendant, proto_node.best_descendant) {
        (Some(head), Some(current)) => head == current,
        _ => false,
    };
    let children = if let Some(children) = parent_index_to_children.get(&index) {
        children
            .iter()
            .map(|child| {
                build_fork_choice_tree(
                    child,
                    parent_index_to_children,
                    proto_array,
                    best_descendant,
                )
            })
            .collect()
    } else {
        vec![]
    };
    ForkChoiceNode {
        slot: proto_node.slot,
        root: proto_node.root,
        weight: proto_node.weight,
        children,
        is_canonical,
    }
}
