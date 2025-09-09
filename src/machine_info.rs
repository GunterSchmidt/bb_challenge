use std::{cmp::Ordering, fmt::Display};

use num_format::ToFormattedString;

use crate::{
    config::{user_locale, StepBig},
    machine_binary::{MachineBinary, MachineId},
    status::MachineStatus,
};

/// Machine with its status and an optional id for result and display.
/// This is designed to be immutable and only created from another machine.
#[derive(Debug, Clone, Copy)]
pub struct MachineInfo {
    /// Outside id, e.g. file machine id. Normalized Id is always calculated, since it is only used for display purposes.
    id: Option<u64>,
    machine: MachineBinary,
    status: MachineStatus,
}

impl MachineInfo {
    pub fn new(machine: MachineBinary, status: MachineStatus) -> MachineInfo {
        Self {
            id: None,
            machine,
            status,
        }
    }

    pub fn new_m_id(machine: MachineId, status: MachineStatus) -> MachineInfo {
        Self {
            id: machine.id_as_option(),
            machine: *machine.machine(),
            status,
        }
    }

    pub fn from_machine(machine: &MachineBinary, status: &MachineStatus) -> MachineInfo {
        Self {
            id: None,
            machine: *machine,
            status: *status,
        }
    }

    pub fn from_machine_id(machine: &MachineId, status: &MachineStatus) -> MachineInfo {
        Self {
            id: machine.id_as_option(),
            machine: *machine.machine(),
            status: *status,
        }
    }

    pub fn has_id(&self) -> bool {
        self.id.is_some()
    }

    /// Returns the given id or the normalized id.
    /// This is inefficient if called multiple times, use [id_calc] instead.
    pub fn id(&self) -> u64 {
        return match self.id {
            Some(id) => id,
            None => self.calc_normalized_id(),
        };
    }

    /// Returns the given id or the normalized id. This will update the id if it does not exists, so it is not calculated twice.
    pub fn id_or_calc_id(&mut self) -> u64 {
        return match self.id {
            Some(id) => id,
            None => {
                let id = self.calc_normalized_id();
                self.id = Some(id);
                id
            }
        };
    }

    /// Calculates the id for forward rotating or backward rotating transitions.
    pub fn calc_normalized_id(&self) -> u64 {
        self.machine.normalized_id_calc()
    }

    /// Returns true if at least one self-referencing transition exists (D1 1LD). \
    /// Slightly slower then [has_self_referencing_transition_store_result] if called repeatedly.
    pub fn has_self_referencing_transition(&self) -> bool {
        self.machine.has_self_referencing_transition()
    }

    /// Returns true if at least one self-referencing transition exists (D1 1LD). \
    /// Also sets an internal marker to avoid another complex identification.
    pub fn has_self_referencing_transition_store_result(&mut self) -> bool {
        self.machine.has_self_referencing_transition_store_result()
    }

    pub fn n_states(&self) -> usize {
        self.machine.n_states()
    }

    pub fn steps(&self) -> StepBig {
        match self.status {
            MachineStatus::DecidedHalt(steps) => steps,
            _ => 0,
        }
    }

    pub fn machine(&self) -> MachineBinary {
        self.machine
    }

    pub fn status(&self) -> MachineStatus {
        self.status
    }

    pub fn to_standard_tm_text_format(&self) -> String {
        self.machine.to_standard_tm_text_format()
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

impl From<&MachineBinary> for MachineInfo {
    fn from(machine: &MachineBinary) -> Self {
        Self {
            id: None,
            machine: *machine,
            status: MachineStatus::NoDecision,
        }
    }
}

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
        let locale = &user_locale();
        let s = match self.id {
            Some(id) => {
                format!(
                    "Machine {:>12}, {}: {}",
                    id.to_formatted_string(locale),
                    self.machine,
                    self.status
                )
            }
            None => {
                format!(
                    "Machine {:>12}, {}: {}",
                    self.calc_normalized_id().to_formatted_string(locale),
                    self.machine,
                    self.status
                )
            }
        };
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
