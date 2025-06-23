use std::fmt::Display;

#[cfg(all(debug_assertions, feature = "bb_debug"))]
use crate::tape_utils::{VecU32Ext, TAPE_DISPLAY_RANGE_128};
use crate::{
    config::Config,
    decider::Decider,
    machine::Machine,
    result::BatchResult,
    status::{MachineStatus, UndecidedReason},
    tape_utils::{
        CLEAR_HIGH95_64BITS_U128, CLEAR_LOW63_32BITS_U128, HIGH32_SWITCH_U128, LOW64_SWITCH_U128,
        MIDDLE_BIT_U128, POS_HALF_U128, TAPE_SIZE_FOURTH_UPPER_128, TAPE_SIZE_HALF_128,
        TL_POS_START_128,
    },
    transition_symbol2::{TransitionSymbol2, TransitionTableSymbol2, TRANSITION_SYM2_START},
    StepType, MAX_TAPE_GROWTH, TAPE_SIZE_INIT_CELL_BLOCKS,
};

/// This decider runs on a 128-Bit number and moves data out to a long tape (Vec). \
/// The tape is not limited in size other than Vec memory limitations.
/// Usage:
/// # Set step_limit and tape_size_limit in the machine to evaluate
/// # Create a new decider for a single machine
/// # Run run_check_hold to check if the machine holds
// TODO Longer jump if multiple u32 in tape_long are FFFF
// TODO Multiple repeating steps, e.g 3 on 001
// pub struct DeciderU128Long<'a> {
pub struct DeciderHoldU128Long {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    tape_shifted: u128,
    pos_middle: usize,
    // The tape_long is a Vec<u64> which allows to copy half of u128 tape_shifted to
    // be copied into the long tape when a bound is reached.
    // TODO The tape has an initial size of e.g. 128 u64 which is 1024 Byte or 8192 tape cells.
    // The size will double every time its limit is reached. E.g it doubles 1x times to get a size of 256 or 16284 cells,
    // which is the size for BB5 Max tape length.
    // Once 131072 u64 is reached (1 MB), it will grow by 1 MB each time.
    // Generally speaking here the head is moving within the tape; it does not shift at all.
    tape_long: Vec<u32>,
    /// tl_pos represents the start of the 128 tape in the long tape (covering 4 u32)
    tl_pos: usize,
    tl_high_bound: usize,
    /// TODO low bound in bit, this is the rightmost doubleword in tape_shifted (bit 0), min value is 0, but will be negative when testing.
    /// Low bound in tape_long, this is the leftmost value.
    tl_low_bound: usize,
    num_steps: StepType,
    tr: TransitionSymbol2,
    // machine id, just for debugging
    id: u64,
    transition_table: TransitionTableSymbol2,
    #[allow(dead_code)]
    status: MachineStatus,
    config: Config,
}

impl DeciderHoldU128Long {
    pub fn new(config: &Config) -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128,

            tape_long: vec![0; TAPE_SIZE_INIT_CELL_BLOCKS],
            tl_pos: TL_POS_START_128,
            tl_low_bound: TL_POS_START_128,
            tl_high_bound: TL_POS_START_128 + 3,

            num_steps: 0,
            // Initialize transition with A0 as start
            tr: TRANSITION_SYM2_START,
            // copy the transition table as this runs faster
            id: 0,
            transition_table: TransitionTableSymbol2::default(),
            status: MachineStatus::NoDecision,
            config: config.clone(),
        }
    }

    // re-uses the decider like new
    pub fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128;

        self.tape_long.clear();
        self.tape_long.resize(TAPE_SIZE_INIT_CELL_BLOCKS, 0);
        self.tl_pos = TL_POS_START_128;
        self.tl_low_bound = TL_POS_START_128;
        self.tl_high_bound = TL_POS_START_128 + 3;

        self.num_steps = 0;
        self.tr = TRANSITION_SYM2_START;
        self.status = MachineStatus::NoDecision;
        // keep step_limit
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
    //         };
    //         d.update_and_move_tape();
    //
    //         d
    //     }

    // pub fn run_test<T: Decider>(&mut self, d: T) {
    //     let p = d.new_decider();
    // }

    // pub fn set_decider(&mut self, decider: T) {
    //     self.decider = Some(decider);
    // }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps. \
    /// This version has a long tape, so it is not restricted to the 128 bit range.
    /// This will use self_referencing_transition speed-up if available.
    fn run_check_hold(&mut self, machine: &Machine) -> MachineStatus {
        self.clear();
        self.id = machine.id();
        self.transition_table = *machine.transition_table();
        if self.transition_table.has_self_referencing_transition() {
            self.run_check_hold_with_self_referencing_transition()
        } else {
            self.run_check_hold_without_self_referencing_transitions()
        }
    }

    fn run_check_hold_with_self_referencing_transition(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            if !self.next_step_self_ref() {
                return self.status;
            };
        }
    }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps. \
    /// This version has a long tape, so it is not restricted to the 128 bit range.
    /// This is not using the self reference speed-up and should only be used if those would mess up the tests.
    fn run_check_hold_without_self_referencing_transitions(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            if !self.next_step() {
                return self.status;
            };
        }
    }

    /// Counts Ones for self referencing speed-up
    fn count_left(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = (self.tape_shifted >> 64) as u64;
        if symbol == 1 {
            t.trailing_ones() + 1
        } else {
            t.trailing_zeros() + 1
        }
    }

    /// Counts Ones for self referencing speed-up
    fn count_right(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = self.tape_shifted as u64;
        if symbol == 1 {
            t.leading_ones()
        } else {
            t.leading_zeros()
        }
    }

    // TODO correct tape
    pub fn count_ones(&self) -> u32 {
        let mut ones = self.tape_shifted.count_ones();
        if self.tl_high_bound - self.tl_low_bound > 3 {
            for n in self.tape_long[self.tl_low_bound..self.tl_pos].iter() {
                ones += n.count_ones();
            }
            for n in self.tape_long[self.tl_pos + 4..self.tl_high_bound + 1].iter() {
                ones += n.count_ones();
            }
        }
        ones
    }

    /// Shifts the pos in the long tape one to left and checks Vec dimensions
    #[inline(always)]
    fn shift_pos_to_left_checked(&mut self) {
        // check if tape is long enough
        if self.tl_pos < self.tl_low_bound + 1 {
            if self.tl_pos == 0 {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 0 -> pos 40
                let grow_by = MAX_TAPE_GROWTH.min(self.tape_long.len());
                let old_len = self.tape_long.len();
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  Tape Resize at start: {} -> {}",
                    self.tape_long.len(),
                    self.tape_long.len() + grow_by
                );
                // Make room in beginning. Grow vector first, then move elements.
                self.tape_long.resize(self.tape_long.len() + grow_by, 0);
                self.tape_long.copy_within(0..old_len, grow_by);
                self.tape_long[0..grow_by].fill(0);
                self.tl_pos += grow_by;
                self.tl_low_bound += grow_by;
                self.tl_high_bound += grow_by;
            }
            self.tl_low_bound -= 1;
        }
        self.tl_pos -= 1;
    }

    /// Shifts the pos in the long tape one to right and checks Vec dimensions
    #[inline(always)]
    pub fn shift_pos_to_right_checked(&mut self) {
        // check if tape is long enough
        if self.tl_pos + 4 > self.tl_high_bound {
            self.tl_high_bound += 1;
            if self.tl_high_bound == self.tape_long.len() {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 96 -> pos 96
                let grow_by = MAX_TAPE_GROWTH.min(self.tape_long.len());
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  Tape Resize at end: {} -> {}",
                    self.tape_long.len(),
                    self.tape_long.len() + grow_by
                );
                self.tape_long.resize(self.tape_long.len() + grow_by, 0);
            }
        }
        self.tl_pos += 1;
    }

    #[inline(always)]
    fn current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn set_current_symbol(&mut self) {
        if self.tr.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    // /// Returns the given machine reference
    // pub fn machine(&self) -> &'a Machine {
    //     self.machine
    // }

    /// Returns the status of the decider
    pub fn status(&self) -> MachineStatus {
        self.status
    }

    /// Returns the status of the decider and additionally written Ones on tape and Tape Size
    pub fn status_full(&self) -> MachineStatus {
        match self.status {
            MachineStatus::DecidedHolds(steps) => MachineStatus::DecidedHoldsDetail(
                steps,
                self.tape_size(),
                self.count_ones() as usize,
            ),
            _ => self.status,
        }
    }

    // fn get_status_hold_details(&self) -> MachineStatus {
    //     MachineStatus::DecidedHoldsDetail(
    //         self.num_steps as StepType,
    //         self.get_tape_size(),
    //         self.tape_shifted.count_ones() as usize,
    //     )
    // }

    // Returns the approximate tape size, which grows by 32 steps
    fn tape_size(&self) -> usize {
        (self.tl_high_bound - self.tl_low_bound + 1) * 32
    }

    /// reads next step and updates transition
    #[inline(always)]
    fn next_step(&mut self) -> bool {
        self.num_steps += 1;
        let curr_read_symbol = self.current_symbol();
        let arr_id = self.tr.state_x2() + curr_read_symbol;
        self.tr = self.transition_table.transition(arr_id);
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        println!("{}", self.step_to_string());

        // check if done
        if self.tr.is_hold() {
            // if self.id == 331136 {
            //     println!("{}", self.id);
            //     println!("{}", self.status);
            //     let machine = Machine::new(self.id, self.transition_table);
            //     let s =
            //         DeciderU128Long::<crate::sub_decider::SubDeciderDummy>::run_decider(&machine);
            //     let rf = machine.has_self_referencing_transition();
            //     println!("{s}");
            //     println!();
            // }
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.set_current_symbol();
            }
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            self.status = MachineStatus::DecidedHolds(self.num_steps);
            return false;
        } else if self.num_steps > self.config.step_limit() {
            self.status = self.undecided_step_limit();
            return false;
        }

        if self.tr.is_dir_right() {
            self.set_current_symbol();

            // normal shift RIGHT -> tape moves left
            self.tape_shifted <<= 1;

            self.pos_middle += 1;
            if self.pos_middle == HIGH32_SWITCH_U128 {
                // save high bytes
                self.shift_pos_to_right_checked();

                // The shift is right, so tape_shifted wanders left -> store high 32 bits.
                self.tape_long[self.tl_pos] =
                    (self.tape_shifted >> TAPE_SIZE_FOURTH_UPPER_128) as u32;

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  RIGHT SAVE HIGH P{}-{}: tape wanders left -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );

                self.pos_middle = MIDDLE_BIT_U128;

                // Load low 32 bit
                self.tape_shifted = (self.tape_shifted & CLEAR_LOW63_32BITS_U128)
                    | ((self.tape_long[self.tl_pos + 2] as u128) << 32);

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!(
                        "  ALoad {}",
                        crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                    );
                    println!(
                        "  RIGHT LOAD LOW  P{}-{}: tape wanders left -> {:?}",
                        self.pos_middle,
                        self.tl_pos,
                        self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                    );
                    print!("");
                }
            }
        } else {
            // normal shift LEFT -> tape moves left
            self.set_current_symbol();

            self.tape_shifted >>= 1;
            self.pos_middle -= 1;
            if self.pos_middle == LOW64_SWITCH_U128 {
                // save high bytes
                self.shift_pos_to_left_checked();

                // The shift is left, so tape_shifted wanders right -> store low 32 bits.
                self.tape_long[self.tl_pos + 3] = self.tape_shifted as u32;

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  LEFT  SAVE HIGH P{}-{}: tape wanders right -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );

                self.pos_middle = MIDDLE_BIT_U128;

                // load high bytes
                self.tape_shifted = (self.tape_shifted & CLEAR_HIGH95_64BITS_U128)
                    | ((self.tape_long[self.tl_pos + 1] as u128) << TAPE_SIZE_HALF_128);

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!(
                        "  ALoad {}",
                        crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                    );
                    println!(
                        "  LEFT  LOAD HIGH P{}-{}: tape wanders right -> {:?}",
                        self.pos_middle,
                        self.tl_pos,
                        self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                    );
                    print!("");
                }
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
        let curr_read_symbol = self.current_symbol();
        let arr_id = self.tr.state_x2() + curr_read_symbol;
        self.tr = self.transition_table.transition(arr_id);
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        println!("{}", self.step_to_string());

        // check if done
        if self.tr.is_hold() {
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.set_current_symbol();
            }
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            self.status = MachineStatus::DecidedHolds(self.num_steps);
            return false;
        } else if self.num_steps > self.config.step_limit() {
            self.status = self.undecided_step_limit();
            return false;
        }

        if self.tr.is_dir_right() {
            // normal shift RIGHT -> tape moves left

            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == arr_id {
                let mut jump = self.count_right(curr_read_symbol) as usize;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump right {jump}");
                if self.pos_middle + jump > HIGH32_SWITCH_U128 {
                    jump = HIGH32_SWITCH_U128 - self.pos_middle;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump right adjusted {jump}");
                }
                self.pos_middle += jump;

                // shift tape
                self.set_current_symbol();
                self.tape_shifted <<= jump;
                self.num_steps += jump as StepType - 1;
            } else {
                self.pos_middle += 1;

                // shift tape
                self.set_current_symbol();
                self.tape_shifted <<= 1;
            }

            if self.pos_middle == HIGH32_SWITCH_U128 {
                // save high bytes
                self.shift_pos_to_right_checked();

                // The shift is right, so tape_shifted wanders left -> store high 32 bits.
                self.tape_long[self.tl_pos] =
                    (self.tape_shifted >> TAPE_SIZE_FOURTH_UPPER_128) as u32;

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  RIGHT SAVE HIGH P{}-{}: tape wanders left -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );

                self.pos_middle = MIDDLE_BIT_U128;

                // Load low 32 bit
                self.tape_shifted = (self.tape_shifted & CLEAR_LOW63_32BITS_U128)
                    | ((self.tape_long[self.tl_pos + 2] as u128) << 32);

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!(
                        "  ALoad {}",
                        crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                    );
                    println!(
                        "  RIGHT LOAD LOW  P{}-{}: tape wanders left -> {:?}",
                        self.pos_middle,
                        self.tl_pos,
                        self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                    );
                    print!("");
                }
            }
        } else {
            // normal shift LEFT -> tape moves left

            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == arr_id {
                let mut jump = self.count_left(curr_read_symbol) as usize;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                if self.pos_middle < LOW64_SWITCH_U128 + jump {
                    jump = self.pos_middle - LOW64_SWITCH_U128;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump left adjusted {jump}");
                }
                self.pos_middle -= jump;

                self.set_current_symbol();
                // shift tape
                self.tape_shifted >>= jump;
                self.num_steps += jump as StepType - 1;
            } else {
                self.pos_middle -= 1;

                self.set_current_symbol();
                // shift tape
                self.tape_shifted >>= 1;
            }

            if self.pos_middle == LOW64_SWITCH_U128 {
                // save high bytes
                self.shift_pos_to_left_checked();

                // The shift is left, so tape_shifted wanders right -> store low 32 bits.
                self.tape_long[self.tl_pos + 3] = self.tape_shifted as u32;

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  LEFT  SAVE HIGH P{}-{}: tape wanders right -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128) // Self::tape_range_to_hex_string(&self.tape_long, TAPE_DISPLAY_RANGE)
                );

                self.pos_middle = MIDDLE_BIT_U128;

                // load high bytes
                self.tape_shifted = (self.tape_shifted & CLEAR_HIGH95_64BITS_U128)
                    | ((self.tape_long[self.tl_pos + 1] as u128) << TAPE_SIZE_HALF_128);

                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!(
                        "  ALoad {}",
                        crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                    );
                    println!(
                        "  LEFT  LOAD HIGH P{}-{}: tape wanders right -> {:?}",
                        self.pos_middle,
                        self.tl_pos,
                        self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                    );
                    print!("");
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
            self.tape_size(),
        )
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
    //             println!("");
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

    /// Debug info on current step
    fn step_to_string(&self) -> String {
        format!(
            "Step {:3} {}: P{}-{} {} Next {}{}",
            self.num_steps,
            self.tr,
            self.tl_pos,
            self.pos_middle,
            crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted),
            // self.get_tape_size(),
            self.tr.state_to_char(),
            self.current_symbol(),
        )
    }

    pub fn run_decider(machine: &Machine) -> MachineStatus {
        let config = Config::new_default(machine.n_states());
        let mut d = Self::new(&config);
        d.decide_machine(machine)
    }
}

impl Decider for DeciderHoldU128Long {
    fn new_decider(&self) -> Self {
        Self::new(&self.config)
    }

    // tape_long_bits in machine?
    // TODO counter: longest loop
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        self.run_check_hold(machine)
    }

    fn decider_run_batch(
        machines: &[Machine],
        run_predecider: bool,
        config: &Config,
    ) -> Option<BatchResult> {
        let decider = Self::new(config);
        crate::decider::decider_generic_run_batch(decider, machines, run_predecider, config)
    }

    fn name(&self) -> String {
        "Decider U128 Long".to_string()
    }
}

impl Display for DeciderHoldU128Long {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let s = String::new();
        // println!("State: Undecided: Too many steps to left.");

        write!(f, "{}", self.step_to_string(),)
    }
}

// pub fn test_decider_hold_u128_applies_bb4_max() {
//     let config = Config::new_default(4);
//     // BB4 Max
//     let machine_bb4_max = Machine::build_machine("BB4_MAX").unwrap();
//
//     let mut decider: DeciderU128Long<SubDeciderDummy> = DeciderU128Long::new(config);
//     let check_result = decider.decide_machine(&machine_bb4_max);
//     // println!("{}", check_result);
//
//     assert_eq!(check_result, MachineStatus::DecidedHolds(107));
// }
//
// pub fn test_decider_hold_u128_applies_bb5_max() {
//     let config = Config::new_default(5);
//     // BB5 Max
//     let machine = Machine::build_machine("BB5_MAX").unwrap();
//     let mut d: DeciderU128Long<SubDeciderDummy> = DeciderU128Long::new(config);
//     let check_result = d.decide_machine(&machine);
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
// }

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_decider_hold_u128_applies_bb4_max() {
        let config = Config::new_default(4);
        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let mut d = DeciderHoldU128Long::new(&config);
        let check_result = d.decide_machine(&machine);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    /// This test runs 50 mio steps, so turn off default = ["bb_debug"].
    fn test_decider_hold_u128_applies_bb5_max() {
        let config = Config::new_default(5);
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let mut d = DeciderHoldU128Long::new(&config);
        let check_result = d.decide_machine(&machine);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(47_176_870));
    }
}
