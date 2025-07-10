use std::{cmp::Ordering, fmt::Display};

use crate::{
    config::StepTypeBig, machine::Machine, status::MachineStatus,
    transition_symbol2::TransitionTableSymbol2,
};

/// Essential info about machine, can be used to store machine with less data.
/// This is designed to be immutable and only created from another machine.
#[derive(Debug, Clone, Copy)]
pub struct MachineInfo {
    id: u64,
    transition_table: TransitionTableSymbol2,
    status: MachineStatus,
}

impl MachineInfo {
    pub fn new(
        id: u64,
        transition_table: TransitionTableSymbol2,
        status: MachineStatus,
    ) -> MachineInfo {
        Self {
            id,
            transition_table,
            status,
        }
    }

    pub fn from_machine(machine: &Machine, status: &MachineStatus) -> MachineInfo {
        Self {
            id: machine.id(),
            transition_table: *machine.transition_table(),
            status: *status,
        }
    }

    pub fn steps(&self) -> StepTypeBig {
        match self.status {
            MachineStatus::DecidedHolds(steps) => steps,
            _ => 0,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn transition_table(&self) -> TransitionTableSymbol2 {
        self.transition_table
    }

    pub fn status(&self) -> MachineStatus {
        self.status
    }

    pub fn to_standard_tm_text_format(&self) -> String {
        self.transition_table.to_standard_tm_text_format()
    }
}

// impl From<&MachineCompactDeprecated> for MachineInfo {
//     fn from(machine: &MachineCompactDeprecated) -> Self {
//         Self {
//             id: machine.id,
//             transition_table: TransitionTableSymbol2::new_with_n_states(
//                 machine.transitions,
//                 machine.n_states,
//             ),
//             status: machine.status,
//         }
//     }
// }

// impl<'a, T: SubDecider> From<&DeciderU128Long<'a, T>> for MachineInfo {
//     fn from(decider: &DeciderU128Long<'a, T>) -> Self {
//         Self {
//             id: decider.machine().id(),
//             transition_table: *decider.machine().transition_table(),
//             status: decider.status_full(),
//         }
//     }
// }

impl PartialEq for MachineInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for MachineInfo {}

impl PartialOrd for MachineInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MachineInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Display for MachineInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!(
            "Machine {:12}, {}: {}",
            self.id, self.transition_table, self.status
        );
        // match self.status {
        //     MachineStatus::Undecided(steps, tape_len) => {
        //         s.push_str(format!(
        //     "Safety stop reached, machine did not hold for {steps} steps or tape length limit {tape_len}").as_str());
        //     }
        //     MachineStatus::DecidedHolds(steps, num_ones) => {
        //         s.push_str(format!("Steps till hold: {}\n", steps).as_str());
        //         s.push_str(format!("Ones on tape: {}", num_ones).as_str());
        //     }
        //     _ => {
        //         s.push_str(format!("State not yet documented: {:?}", self.status).as_str());
        //     }
        // }
        write!(f, "{s}")
    }
}
