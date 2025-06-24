#![allow(unused)]
use crate::{
    decider::Decider,
    decider_loop_v4::StepLoop,
    machine::Machine,
    status::MachineStatus,
    sub_decider::SubDecider,
    transition_symbol2::{TransitionSymbol2, TRANSITION_SYM2_START},
    MAX_STATES,
};

const STEP_LIMIT_DECIDER_LOOP: usize = 510;

#[derive(Debug)]
pub struct DeciderLoopV5 {
    steps: Vec<StepLoop>,
    /// stores the step ids for each State-Symbol combination (basically e.g. all from A0 steps)
    // TODO check if storage as u16 is faster
    maps_1d: [Vec<u16>; 2 * (MAX_STATES + 1)],
}

impl DeciderLoopV5 {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Decider for DeciderLoopV5 {
    fn new_decider(&self) -> Self {
        Self::default()
    }

    // TODO counter: longest loop
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        // initialize decider

        self.steps.clear();
        // map for each transition, which step went into it
        // TODO extra level for 0/1 at head position, maybe better calc single array
        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
        // Initialize transition with A0 as start
        let mut tr = TRANSITION_SYM2_START;

        MachineStatus::NoDecision
    }

    fn name(&self) -> String {
        "Decider Loop V5".to_string()
    }

    fn decider_run_batch(
        machines: &[Machine],
        run_predecider: bool,
        config: &crate::config::Config,
    ) -> Option<crate::result::ResultBatch> {
        todo!()
    }
}

impl Default for DeciderLoopV5 {
    fn default() -> Self {
        Self {
            steps: Vec::with_capacity(STEP_LIMIT_DECIDER_LOOP),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(STEP_LIMIT_DECIDER_LOOP / 4)),
        }
    }
}

impl SubDecider for DeciderLoopV5 {}
