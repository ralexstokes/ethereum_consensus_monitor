use slot_clock::{Slot, SlotClock, SystemTimeSlotClock};
use tokio::time::{sleep, Duration};

pub struct Timer {
    inner: SystemTimeSlotClock,
    slots_per_epoch: u64,
}

impl Timer {
    pub fn new(genesis_time: u64, seconds_per_slot: u64, slots_per_epoch: u64) -> Self {
        let genesis = Duration::from_secs(genesis_time);
        let slot_duration = Duration::from_secs(seconds_per_slot);
        let genesis_slot = Slot::new(0);
        let inner = SystemTimeSlotClock::new(genesis_slot, genesis, slot_duration);
        Self {
            inner,
            slots_per_epoch,
        }
    }

    pub fn is_before_genesis(&self) -> bool {
        self.inner
            .is_prior_to_genesis()
            .expect("can read the system clock")
    }

    pub async fn tick_slot(&self) -> Slot {
        let next_slot_duration = self
            .inner
            .duration_to_next_slot()
            .expect("can read system clock");

        sleep(next_slot_duration).await;
        self.inner.now().expect("can read system clock")
    }
}
