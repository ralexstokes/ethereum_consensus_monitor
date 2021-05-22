use eth2::types::{Hash256, Slot};
use serde::Serialize;
use std::fmt;

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
