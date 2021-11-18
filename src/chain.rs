use eth2::types::{Checkpoint, FinalityCheckpointsData};
use eth2::types::{Hash256, Slot};
use serde::Serialize;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone, Debug, Serialize, Default)]
pub struct Coordinate {
    pub slot: Slot,
    pub root: Hash256,
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.slot, self.root)
    }
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct FinalityData {
    pub justified_checkpoint: Option<Checkpoint>,
    pub finalized_checkpoint: Option<Checkpoint>,
}

impl From<FinalityCheckpointsData> for FinalityData {
    fn from(checkpoints: FinalityCheckpointsData) -> Self {
        Self {
            justified_checkpoint: Some(checkpoints.current_justified),
            finalized_checkpoint: Some(checkpoints.finalized),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ChainInner {
    finality_data: Option<FinalityData>,
}

#[derive(Clone, Debug, Default)]
pub struct Chain(Arc<Mutex<ChainInner>>);

impl Chain {
    // pub fn get_status(&self) -> Option<FinalityData> {
    //     self.0
    //         .lock()
    //         .ok()
    //         .and_then(|guard| guard.finality_data.clone())
    // }

    // pub fn set_status(&self, data: FinalityData) {
    //     if let Ok(mut inner) = self.0.lock() {
    //         inner.finality_data = Some(data);
    //     }
    // }
}
