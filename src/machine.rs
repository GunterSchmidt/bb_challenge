//! A single Turing Machine.
use std::{fmt::Display, io};

use crate::{
    file::BBFileReader,
    machine_info::MachineInfo,
    transition_symbol2::{TransitionSymbol2, TransitionTableSymbol2},
};

// TODO move and update text
/// Creation of variants
///   
/// Write Symbol: 0, 1 (optional undefined for last step, but will result in same number of steps)  
/// Direction, L, R (undefined without effect)  
/// Next Status: A, B, C, D, E (BB5)  
///
/// For each transition, this results in 2 (symbols) * 2 (directions) * 5 (states) + 1 (undefined) = 21 possibilites.  
/// In the transition table there are 5 (current state) * 2 (current symbol) = 10 fields, so to the power of 10.  
/// General formula (4*s+1)^2*s (s = number of status)  
/// Results for  
/// BB=1: 25
/// BB=2: 6.561
/// BB=3: 4.826.809 (4.8 million)
/// BB=4: 6.975.757.441 (7 billion)
/// BB=5: 16.679.880.978.201 (16.7 trillion)
/// BB=6: 59.604.644.775.390.600 (59.6e15)
/// BB=7: 297.558.232.675.799.000.000 (257.6e18) Limit 64-Bit
/// BB=8: 1.977.985.201.462.560.000.000.000 (2e24)
/// BB=9: 16.890.053.810.563.300.000.000.000.000
/// BB=10: 180.167.782.956.421.000.000.000.000.000.000
///
/// However, a lot of variants are redundant, as they will produce the same result, e.g.
/// -- When the first step does not change the state, it will run indefinetely. Thus
/// the transitions of the other fields do not matter.
/// -- L and R can be switched for all variants resulting in same stap count
/// -- status labels can be switched (other than A), e.g. B and C with same result
/// -- all transitions go in the same direction
/// -- all transitions write 0 (test hyothesis: either holds in 2*s steps or runs indefinetly)
/// -- loop within?

/// Turing Machine (which is a single generated permutation)
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Machine {
    id: u64,
    /// Field 0 is used for more information to keep the size of this struct small.
    transition_table: TransitionTableSymbol2,
    // has_self_referencing_transition: bool,
}

impl Machine {
    // #[inline(always)]
    pub fn new(id: u64, transition_table: TransitionTableSymbol2) -> Self {
        // let mut has_self_referencing_transition = false;
        // for (id, t) in transition_table.transitions.iter().enumerate().skip(2) {
        //     if t.array_id() == id {
        //         // has_self_referencing_transition = true;
        //         transition_table.set_has_self_referencing_transition();
        //         break;
        //     }
        //     if t.is_unused() {
        //         break;
        //     }
        // }

        Self {
            id,
            transition_table,
            // has_self_referencing_transition,
        }
    }

    /// Creates the transition table from the Standard TM Text Format \
    /// https://www.sligocki.com/2022/10/09/standard-tm-format.html
    pub fn from_standard_tm_text_format(
        machine_id: u64,
        transitions_text: &str,
    ) -> Result<Self, &'static str> {
        let tg = crate::transition_generic::TransitionTableGeneric::from_standard_tm_text_format(
            transitions_text,
        )?;
        let t = TransitionTableSymbol2::try_from(tg)?;
        let m = Self::new(machine_id, t);

        Ok(m)
    }

    // new from transitions as String tuple
    pub fn from_string_tuple(machine_id: u64, transitions_as_str: &[(&str, &str)]) -> Self {
        let transition_table = TransitionTableSymbol2::from_string_tuple(transitions_as_str);
        Self::new(machine_id, transition_table)
    }

    pub fn from_bbchallenge_id(
        machine_id: u64,
        path_to_bbchallenge_db: &str,
    ) -> io::Result<Machine> {
        BBFileReader::read_machine_single(machine_id, path_to_bbchallenge_db)
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn transition(&self, array_id: usize) -> TransitionSymbol2 {
        self.transition_table.transition(array_id)
    }

    pub fn transition_table(&self) -> &TransitionTableSymbol2 {
        &self.transition_table
    }

    // pub fn transition_table_as_ref(&self) -> &TransitionTableSymbol2 {
    //     &self.transition_table
    // }

    /// This only returns the correct value if [set_eval_has_self_referencing_transition] was run.
    pub fn has_self_referencing_transition(&self) -> bool {
        self.transition_table.has_self_referencing_transition()
    }

    /// This needs to be run once to identify self referencing transitions. Somewhat time consuming.
    pub fn set_eval_has_self_referencing_transition(&mut self) -> bool {
        self.transition_table
            .eval_set_has_self_referencing_transition()
    }

    pub fn n_states(&self) -> usize {
        self.transition_table.n_states()
    }

    // pub fn decide_hold(&self) -> MachineStatus {
    //     let config = Config::new_default(self.n_states());
    //     let mut d: DeciderU128Long<SubDeciderDummy> = DeciderU128Long::new(&config);
    //     d.decide_machine(&self)
    // }

    pub fn to_standard_tm_text_format(&self) -> String {
        self.transition_table.to_standard_tm_text_format()
    }

    /// Some notable machines
    /// Builds certain default machines which may be usefull for testing.
    /// SA: https://www.scottaaronson.com/papers/bb.pdf
    pub fn build_machine(name: &str) -> Option<Self> {
        let mut id = 0;
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        // let mut tm = "";

        match name.to_uppercase().to_owned().as_str() {
            "BB3_MAX" => {
                id = 651320;
                transitions.push(("1LB", "---"));
                transitions.push(("1RB", "0LC"));
                transitions.push(("1RC", "1RA"));
            }
            "BB4_MAX" => {
                id = 322636617;
                transitions.push(("1RC", "1LC"));
                transitions.push(("---", "1LD"));
                transitions.push(("1LA", "0LB"));
                transitions.push(("1RD", "0RA"));
            }
            "BB5_MAX" => {
                transitions.push(("1RB", "1LC"));
                transitions.push(("1RC", "1RB"));
                transitions.push(("1RD", "0LE"));
                transitions.push(("1LA", "1LD"));
                transitions.push(("---", "0LA"));
                // transitions.push(("1RZ", "0LA"));
            }
            "SA_BB2_MAX" => {
                id = 0;
                transitions.push(("1RB", "1LB"));
                transitions.push(("1LA", "---"));
            }
            "SA_BB3_MAX" => {
                id = 651320;
                transitions.push(("1LB", "---"));
                transitions.push(("1RB", "0LC"));
                transitions.push(("1RC", "1RA"));
            }
            // other older ones, check if usefull
            "BB4_28051367" => {
                // endless, no hold
                id = 28051367;
                transitions.push(("1LB", "1RC"));
                transitions.push(("0LC", "0LD"));
                transitions.push(("0RD", "0LA"));
                transitions.push(("1RA", "0RA"));
            }
            "BB3_SINUS" => {
                // endless
                id = 84080;
                transitions.push(("1RC", "0LB"));
                transitions.push(("1LA", "---"));
                transitions.push(("0LA", "0RA"));
            }
            "BB3_TEST" => {
                // wrong hold count
                id = 1469538;
                transitions.push(("1RB", "0LB"));
                transitions.push(("1LC", "1RB"));
                transitions.push(("---", "1LA"));
            }
            "BB5_S105" => {
                // 105 steps
                // https://bbchallenge.org/story#bb5
                transitions.push(("1RB", "1LC"));
                transitions.push(("0LB", "1LA"));
                transitions.push(("1RD", "1LB"));
                transitions.push(("1RE", "0RD"));
                transitions.push(("0RA", "---"));
            }
            "ENDLESS" => {
                transitions.push(("0RA", "---"));
            }
            "TEST" => {
                transitions.push(("1RB", "---"));
                transitions.push(("1LB", "0RA"));
                transitions.push(("0RA", "0RA"));
                transitions.push(("0RA", "0RA"));
            }

            _ => return None,
        }
        let mut m = Self::from_string_tuple(id, &transitions);
        m.set_eval_has_self_referencing_transition();

        Some(m)
    }
}

impl From<MachineInfo> for Machine {
    fn from(mi: MachineInfo) -> Self {
        Self {
            id: mi.id(),
            transition_table: mi.transition_table(),
        }
    }
}

impl Display for Machine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID: {} {}", self.id, self.transition_table)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::transition_compact::TRANSITION_SYM2_UNUSED;
//
//     use super::*;
//
//     // #[test]
//     // fn test_permutation_default() {
//     //     let p_default_unused = Machine::new();
//     //     for &t in &p_default_unused.transition_table.transitions_all()[2..] {
//     //         assert_eq!(t, TRANSITION_SYM2_UNUSED);
//     //     }
//     // }
// }

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_1RB0LB_1LA0RA() {
        let tm_in = "1RB0LB_1LA0RA";
        let m = Machine::from_standard_tm_text_format(0, tm_in).unwrap();
        let tm_out = m.to_standard_tm_text_format();
        assert_eq!(tm_in, tm_out);
    }
}
