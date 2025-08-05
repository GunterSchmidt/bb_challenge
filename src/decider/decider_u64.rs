use std::fmt::Display;

use crate::{
    config::{Config, StepTypeBig, StepTypeSmall},
    machine::Machine,
    status::{MachineStatus, UndecidedReason},
    tape_utils::{U64Ext, MIDDLE_BIT_U64, POS_HALF_U64, TAPE_SIZE_BIT_U64},
    transition_symbol2::TransitionSymbol2,
};

/// This decider is the fastest as it runs on a 64-Bit number only. \
/// The tape is limited to these 64 bit, so it runs only a few steps, roughly between 200 and 300 hundred.
/// Performance tests have yielded that the 128-bit version is just as fast,
/// so this will not be further developed.
pub struct DeciderU64<'a> {
    // TODO make fields private and include all tape logic here
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    pub tape_shifted: u64,
    pub low_bound: StepTypeSmall,
    pub high_bound: StepTypeSmall,
    pub num_steps: StepTypeSmall,
    pub tr: TransitionSymbol2,
    pub machine: &'a Machine,
    pub status: MachineStatus,
    step_limit: StepTypeSmall,
}

impl<'a> DeciderU64<'a> {
    pub fn new(machine: &'a Machine, config: &Config) -> Self {
        Self {
            tape_shifted: 0,
            low_bound: MIDDLE_BIT_U64 as StepTypeSmall,
            high_bound: MIDDLE_BIT_U64 as StepTypeSmall,
            num_steps: 0,
            // Initialize transition with A0 as start
            tr: crate::transition_symbol2::TRANSITION_SYM2_START,
            machine,
            status: MachineStatus::NoDecision,
            step_limit: config
                .step_limit_hold()
                .min(StepTypeSmall::MAX as StepTypeBig) as StepTypeSmall,
        }
    }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps.  
    pub fn run_check_hold(&mut self) -> MachineStatus {
        if self.machine.has_self_referencing_transition() {
            return self.run_check_hold_self_ref();
        }

        // loop over transitions to write tape
        loop {
            if !self.next_step() {
                return self.status;
            };
        }
    }

    pub fn run_check_hold_self_ref(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            if !self.next_step_self_ref() {
                return self.status;
            };
        }
    }

    fn count_left(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get upper part
        let t = (self.tape_shifted >> 32) as u32;
        if symbol == 1 {
            t.trailing_ones() + 1
        } else {
            t.trailing_zeros() + 1
        }
    }

    fn count_right(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = self.tape_shifted as u32;
        if symbol == 1 {
            t.leading_ones()
        } else {
            t.leading_zeros()
        }
    }

    fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U64) != 0) as usize
    }

    #[allow(dead_code)]
    fn get_status_hold_details(&self) -> MachineStatus {
        MachineStatus::DecidedHoldsDetail(
            self.num_steps as StepTypeBig,
            self.get_tape_size() as StepTypeSmall,
            self.tape_shifted.count_ones() as StepTypeSmall,
        )
    }

    fn get_tape_size(&self) -> StepTypeSmall {
        self.high_bound - self.low_bound + 1
    }

    // reads next step and updates transition
    #[inline(always)]
    fn next_step(&mut self) -> bool {
        self.num_steps += 1;
        let curr_read_symbol = self.get_current_symbol();
        let arr_id = self.tr.state_x2() + curr_read_symbol;
        self.tr = self.machine.transition(arr_id);
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        println!("{}", self.step_to_string());

        // check if done
        if self.tr.is_hold() {
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.update_tape_symbol();
            }
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            self.status = MachineStatus::DecidedHolds(self.num_steps as StepTypeBig);
            return false;
        } else if self.num_steps > self.step_limit {
            self.status = self.undecided_step_limit();
            return false;
        }

        if self.tr.is_dir_right() {
            self.high_bound += 1;
            if self.high_bound == TAPE_SIZE_BIT_U64 {
                self.status = MachineStatus::Undecided(
                    UndecidedReason::TapeLimitLeftBoundReached,
                    self.num_steps as StepTypeBig,
                    32,
                );
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!("{}", self.step_to_string());
                    println!("State: Undecided: Right bound reached.");
                    // println!("{}", self.status);
                }
                return false;
            }
            self.update_tape_symbol();
            self.tape_shifted <<= 1;
            if self.low_bound < MIDDLE_BIT_U64 {
                self.low_bound += 1;
            }
        } else {
            if self.low_bound == 0 {
                self.status = MachineStatus::Undecided(
                    UndecidedReason::TapeLimitRightBoundReached,
                    self.num_steps as StepTypeBig,
                    32,
                );
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!("{}", self.step_to_string());
                    println!("State: Undecided: Left bound reached.");
                    // println!("{}", self.status);
                }
                return false;
            }
            self.update_tape_symbol();
            self.tape_shifted >>= 1;
            self.low_bound -= 1;
            if self.high_bound > MIDDLE_BIT_U64 {
                self.high_bound -= 1;
            }
        };
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        if self.num_steps % 100 == 0 {
            println!();
        }

        true
    }

    /// reads next step and updates transition
    #[inline(always)]
    fn next_step_self_ref(&mut self) -> bool {
        self.num_steps += 1;
        let curr_read_symbol = self.get_current_symbol();
        let arr_id = self.tr.state_x2() + curr_read_symbol;
        self.tr = self.machine.transition(arr_id);
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        println!("{}", self.step_to_string());

        // check if done
        if self.tr.is_hold() {
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.update_tape_symbol();
            }
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            self.status = MachineStatus::DecidedHolds(self.num_steps as StepTypeBig);
            return false;
        } else if self.num_steps > self.step_limit {
            self.status = self.undecided_step_limit();
            return false;
        }

        self.update_tape_symbol();
        if self.tr.is_dir_right() {
            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == arr_id {
                let mut jump = self.count_right(curr_read_symbol) as StepTypeSmall;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump right {jump}");
                if self.high_bound + jump > TAPE_SIZE_BIT_U64 {
                    jump = TAPE_SIZE_BIT_U64 - self.high_bound;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump right adjusted {jump}");
                }
                self.high_bound += jump;

                // shift tape
                self.tape_shifted <<= jump;
                self.num_steps += jump - 1;

                self.low_bound = MIDDLE_BIT_U64.min(self.low_bound + jump);
            } else {
                self.high_bound += 1;

                // shift tape
                self.tape_shifted <<= 1;

                if self.low_bound < MIDDLE_BIT_U64 {
                    self.low_bound += 1;
                }
            }

            if self.high_bound == TAPE_SIZE_BIT_U64 {
                self.status = MachineStatus::Undecided(
                    UndecidedReason::TapeLimitLeftBoundReached,
                    self.num_steps as StepTypeBig,
                    32,
                );
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!("{}", self.step_to_string());
                    println!("State: Undecided: Right bound reached.");
                    // println!("{}", self.status);
                }
                return false;
            }
        } else {
            // goes left

            if self.low_bound == 0 {
                self.status = MachineStatus::Undecided(
                    UndecidedReason::TapeLimitRightBoundReached,
                    self.num_steps as StepTypeBig,
                    32,
                );
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!("{}", self.step_to_string());
                    println!("State: Undecided: Left bound reached.");
                    // println!("{}", self.status);
                }
                return false;
            }

            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == arr_id {
                let mut jump = self.count_left(curr_read_symbol) as StepTypeSmall;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                if self.low_bound < jump {
                    jump = self.low_bound;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump left adjusted {jump}");
                }
                self.low_bound -= jump;

                // shift tape
                self.tape_shifted >>= jump;
                self.num_steps += jump - 1;
                self.high_bound = MIDDLE_BIT_U64.max(self.high_bound - jump);
            } else {
                self.low_bound -= 1;

                // shift tape
                self.tape_shifted >>= 1;

                if self.high_bound > MIDDLE_BIT_U64 {
                    self.high_bound -= 1;
                }
            }
        };

        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        if self.num_steps % 100 == 0 {
            println!();
        }

        true
    }

    fn undecided_step_limit(&self) -> MachineStatus {
        MachineStatus::Undecided(
            UndecidedReason::StepLimit,
            self.num_steps as StepTypeBig,
            self.get_tape_size(),
        )
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn update_tape_symbol(&mut self) {
        if self.tr.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U64
        } else {
            self.tape_shifted &= !POS_HALF_U64
        };
    }

    // fn write_last_symbol(&mut self) -> MachineStatus {
    //     // write last symbol
    //     if !self.tr.is_symbol_undefined() {
    //         self.update_tape_symbol();
    //     }
    //     // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
    //     MachineStatus::DecidedHolds
    //     // MachineStatus::DecidedHoldsOld(num_steps as StepType, tape_shifted.count_ones() as usize)
    // }

    fn step_to_string(&self) -> String {
        format!(
            "Step {:3} {}: {} H{} L{} Size {}, Next {}{}",
            self.num_steps,
            self.tr,
            self.tape_shifted.to_binary_split_string(),
            self.high_bound,
            self.low_bound,
            self.get_tape_size(),
            self.tr.state_to_char(),
            self.get_current_symbol(),
        )
    }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps.
    pub fn check_hold(machine: &Machine, config: &Config) -> MachineStatus {
        let mut decider = DeciderU64::new(machine, config);

        // loop over transitions to write tape
        loop {
            if !decider.next_step() {
                return decider.status;
            };
        }
    }
}

impl Display for DeciderU64<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.step_to_string(),)
    }
}

#[cfg(test)]
mod tests {

    use crate::config::Config;

    use super::*;

    #[test]
    fn test_decider_hold_u64_applies_bb4_max() {
        let config = Config::new_default(4);
        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let mut d = DeciderU64::new(&machine, &config);
        let check_result = d.run_check_hold();
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    fn test_decider_hold_u64_applies_not_bb5_max() {
        let config = Config::new_default(5);
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let mut d = DeciderU64::new(&machine, &config);
        let check_result = d.run_check_hold();
        // println!("{}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::Undecided(UndecidedReason::TapeLimitRightBoundReached, 301, 32)
        );
    }
}
