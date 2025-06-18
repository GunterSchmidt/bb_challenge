use std::fmt::Display;

use crate::{
    config::Config,
    machine::Machine,
    pre_deciders::run_pre_deciders,
    status::{MachineStatus, UndecidedReason},
    tape_utils::{U128Ext, MIDDLE_BIT_U128, POS_HALF_U128, TAPE_SIZE_BIT_U128},
    transition_symbol2::TransitionSymbol2,
    StepType,
};

pub struct DeciderU128<'a> {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    tape_shifted: u128,
    // using u32 as low & high_bound results in 10% performance drop
    low_bound: usize,
    high_bound: usize,
    num_steps: StepType,
    tr: TransitionSymbol2,
    machine: &'a Machine,
    status: MachineStatus,
    /// Runs pre-decider test by default, can be turned off.
    pub check_pre_deciders: bool,
    step_limit: StepType,
}

impl<'a> DeciderU128<'a> {
    pub fn new(machine: &'a Machine, config: &Config) -> Self {
        Self {
            tape_shifted: 0,
            low_bound: MIDDLE_BIT_U128,
            high_bound: MIDDLE_BIT_U128,
            num_steps: 0,
            // Initialize transition with A0 as start
            tr: crate::transition_symbol2::TRANSITION_SYM2_START,
            machine,
            status: MachineStatus::NoDecision,
            check_pre_deciders: true,
            step_limit: config.step_limit,
        }
    }

    //     pub fn new_handover_u64(decider_u64: &'a DeciderU64) -> Self {
    //         let mut d = Self {
    //             tape_shifted: (decider_u64.tape_shifted as u128) << 32,
    //             low_bound: decider_u64.low_bound + 32,
    //             high_bound: decider_u64.high_bound + 32,
    //             num_steps: decider_u64.num_steps,
    //             // Initialize transition with A0 as start
    //             tr: decider_u64.tr,
    //             machine: decider_u64.machine,
    //             status: MachineStatus::NoDecision,
    //             check_pre_deciders: true,
    //         };
    //         d.update_and_move_tape();
    //
    //         d
    //     }

    // pub fn run_pre_deciders(&mut self) -> MachineStatus {
    //     run_pre_deciders(self.machine.transition_table())
    // }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps.  
    pub fn run_check_hold(&mut self) -> MachineStatus {
        if self.check_pre_deciders {
            let result = run_pre_deciders(self.machine.transition_table());
            if result != MachineStatus::NoDecision {
                return result;
            }
        }

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
        let t = (self.tape_shifted >> 64) as u64;
        if symbol == 1 {
            t.trailing_ones() + 1
        } else {
            t.trailing_zeros() + 1
        }
    }

    fn count_right(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = self.tape_shifted as u64;
        if symbol == 1 {
            t.leading_ones()
        } else {
            t.leading_zeros()
        }
    }

    fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
    }

    #[allow(dead_code)]
    fn get_status_hold_details(&self) -> MachineStatus {
        MachineStatus::DecidedHoldsDetail(
            self.num_steps as StepType,
            self.get_tape_size(),
            self.tape_shifted.count_ones() as usize,
        )
    }

    fn get_tape_size(&self) -> usize {
        // if self.high_bound < self.low_bound as usize {
        //     print!("");
        // }
        self.high_bound - self.low_bound + 1
    }

    /// reads next step and updates transition
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
            self.status = MachineStatus::DecidedHolds(self.num_steps);
            return false;
        } else if self.num_steps > self.step_limit {
            self.status = self.undecided_step_limit();
            return false;
        }

        if self.tr.is_dir_right() {
            self.high_bound += 1;
            if self.high_bound == TAPE_SIZE_BIT_U128 {
                self.status = MachineStatus::UndecidedFastTapeBoundReached;
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
            if self.low_bound < MIDDLE_BIT_U128 {
                self.low_bound += 1;
            }
        } else {
            if self.low_bound == 0 {
                self.status = MachineStatus::UndecidedFastTapeBoundReached;
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
            if self.high_bound > MIDDLE_BIT_U128 {
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
            self.status = MachineStatus::DecidedHolds(self.num_steps);
            return false;
        } else if self.num_steps > self.step_limit {
            self.status = self.undecided_step_limit();
            return false;
        }

        self.update_tape_symbol();
        if self.tr.is_dir_right() {
            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == arr_id {
                let mut jump = self.count_right(curr_read_symbol) as usize;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump right {jump}");
                if self.high_bound + jump > TAPE_SIZE_BIT_U128 {
                    jump = TAPE_SIZE_BIT_U128 - self.high_bound;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump right adjusted {jump}");
                }
                self.high_bound += jump;

                // shift tape
                self.tape_shifted <<= jump;
                self.num_steps += jump as StepType - 1;

                self.low_bound = MIDDLE_BIT_U128.min(self.low_bound + jump);
            } else {
                self.high_bound += 1;

                // shift tape
                self.tape_shifted <<= 1;

                if self.low_bound < MIDDLE_BIT_U128 {
                    self.low_bound += 1;
                }
            }

            if self.high_bound == TAPE_SIZE_BIT_U128 {
                self.status = MachineStatus::UndecidedFastTapeBoundReached;
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
                self.status = MachineStatus::UndecidedFastTapeBoundReached;
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
                let mut jump = self.count_left(curr_read_symbol) as usize;
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
                self.num_steps += jump as StepType - 1;
                self.high_bound = MIDDLE_BIT_U128.max(self.high_bound - jump);
            } else {
                self.low_bound -= 1;

                // shift tape
                self.tape_shifted >>= 1;

                if self.high_bound > MIDDLE_BIT_U128 {
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
            self.num_steps as StepType,
            self.get_tape_size(),
        )
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn update_tape_symbol(&mut self) {
        if self.tr.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    /// Update tape: write symbol at head position into cell.
    /// This is only required for handover from u64.
    // #[inline(always)]
    //     fn update_and_move_tape(&mut self) -> bool {
    //         if self.tr.goes_right() {
    //             self.high_bound += 1;
    //             if self.high_bound == TAPE_SIZE_BIT_U128 {
    //                 self.status = MachineStatus::UndecidedFastTapeBoundReached;
    //                 #[cfg(all(debug_assertions, feature = "bb_debug"))]
    //                 {
    //                     println!("{}", self.step_to_string());
    //                     println!("State: Undecided: Right bound reached.");
    //                     // println!("{}", self.status);
    //                 }
    //                 return false;
    //             }
    //             self.update_tape_symbol();
    //             if self.low_bound < MIDDLE_BIT_U128 {
    //                 self.low_bound += 1;
    //             }
    //             self.tape_shifted <<= 1;
    //         } else {
    //             if self.low_bound == 0 {
    //                 self.status = MachineStatus::UndecidedFastTapeBoundReached;
    //                 #[cfg(all(debug_assertions, feature = "bb_debug"))]
    //                 {
    //                     println!("{}", self.step_to_string());
    //                     println!("State: Undecided: Left bound reached.");
    //                     // println!("{}", self.status);
    //                 }
    //                 return false;
    //             }
    //             self.update_tape_symbol();
    //             self.low_bound -= 1;
    //             if self.high_bound > MIDDLE_BIT_U128 {
    //                 self.high_bound -= 1;
    //             }
    //             self.tape_shifted >>= 1;
    //         };
    //         #[cfg(all(debug_assertions, feature = "bb_debug"))]
    //         if self.num_steps % 100 == 0 {
    //             println!();
    //         }
    //
    //         true
    //     }

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
            "Step {:3} {}: {} H{:2} L{:2} Next {}{}",
            self.num_steps,
            self.tr,
            self.tape_shifted.to_binary_split_string(),
            self.high_bound,
            self.low_bound,
            self.tr.state_to_char(),
            self.get_current_symbol(),
        )
    }

    /// Do not use, it is only for a benchmark.
    pub fn check_hold(machine: &Machine, config: &Config) -> MachineStatus {
        let mut decider = DeciderU128::new(machine, config);

        // loop over transitions to write tape
        loop {
            if !decider.next_step() {
                return decider.status;
            };
        }
    }
}

impl Display for DeciderU128<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.step_to_string(),)
    }
}

pub fn test_decider_hold_u128_applies_not_bb5_max() {
    // BB5 Max
    let machine_bb5_max = Machine::build_machine("BB5_MAX").unwrap();
    // machine_bb5_max.step_limit = 50_000_000;
    // let mut decider_u64 = crate::decider_u64::DeciderU64::new(&machine_bb5_max);
    // let mut check_result = decider_u64.run();
    // if check_result == MachineStatus::UndecidedFastTapeBoundReached {
    // let mut decider = DeciderU128::new_handover_u64(&decider_u64);
    // check_result = decider.run();
    // }
    // println!("{}", check_result);

    let config = Config::new_default(5);
    let mut decider = DeciderU128::new(&machine_bb5_max, &config);
    let check_result = decider.run_check_hold();

    assert_eq!(check_result, MachineStatus::UndecidedFastTapeBoundReached);
}

#[cfg(test)]
mod tests {

    use crate::config::Config;

    use super::*;

    #[test]
    fn test_decider_hold_u128_applies_bb4_max() {
        // BB4 Max
        let config = Config::new_default(4);
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let mut d = DeciderU128::new(&machine, &config);
        let check_result = d.run_check_hold();
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    fn test_decider_hold_u64_applies_not_bb5_max() {
        // BB5 Max
        let config = Config::new_default(5);
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let mut d = DeciderU128::new(&machine, &config);
        let check_result = d.run_check_hold();
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::UndecidedFastTapeBoundReached);
    }
}
