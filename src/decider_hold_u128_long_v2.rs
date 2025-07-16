// TODO inner repeated cycle to move faster
// Sometimes the same rhythm repeats itself until the tape bit change
// Example BB5_MAX: Here A1-C1-E1 repeat as long there are 3 '1' on the tape and are replaced with 001:
// The simple speed-up would be to shift the tape by 3 each time. This is simpler regarding the update of the tape and long tape,
// but will be much less efficient.
// The idea is to count the ones, shift the tape in one big step and continue normally. This has massive implications on long_tape and
// loading of long tape. All cells of long tape would need to be updated.
// On very large jumps it might be interesting to not update long_tape at all, but maintain a range of long tape fields which repeat.
// In this case this would be a set of 3 u32-bit fields, as 3 bits are repeated, which will not make identical u32.
// Step  4128 B1 1RB: 11111111111111111111111111111111_111111111111111111111111_11111111*11111111_111111110000000000000000_00000000000000000000000000000000
// Step  4144 B1 1RB: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4145 B0 1RC: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4146 C0 1RD: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4147 D0 1LA: 01111111111111111111111111111111_111111111111111111111111_111111[11*1]1000000_000000000000000000000000_00000000000000000000000000000000
// Step  4148 A1 1LC: 00111111111111111111111111111111_111111111111111111111111_11111111*11100000_000000000000000000000000_00000000000000000000000000000000
// Step  4149 C1 0LE: 00011111111111111111111111111111_111111111111111111111111_11111111*10110000_000000000000000000000000_00000000000000000000000000000000
// Step  4150 E1 0LA: 00001111111111111111111111111111_111111111111111111111111_11111111*1[001]1000_000000000000000000000000_00000000000000000000000000000000
// Step  4151 A1 1LC: 00000111111111111111111111111111_111111111111111111111111_11111111*11001100_000000000000000000000000_00000000000000000000000000000000
// Step  4152 C1 0LE: 00000011111111111111111111111111_111111111111111111111111_11111111*10100110_000000000000000000000000_00000000000000000000000000000000
// Step  4153 E1 0LA: 00000001111111111111111111111111_111111111111111111111111_11111111*10010011_000000000000000000000000_00000000000000000000000000000000
// Step  4154 A1 1LC: 00000000111111111111111111111111_111111111111111111111111_11111111*11001001_100000000000000000000000_00000000000000000000000000000000
// Step  4155 C1 0LE: 00000000011111111111111111111111_111111111111111111111111_11111111*10100100_110000000000000000000000_00000000000000000000000000000000
// Step  4156 E1 0LA: 00000000001111111111111111111111_111111111111111111111111_11111111*10010010_011000000000000000000000_00000000000000000000000000000000
// Step  4157 A1 1LC: 00000000000111111111111111111111_111111111111111111111111_11111111*11001001_001100000000000000000000_00000000000000000000000000000000
// Step  4158 C1 0LE: 00000000000011111111111111111111_111111111111111111111111_11111111*10100100_100110000000000000000000_00000000000000000000000000000000
// Step  4159 E1 0LA: 00000000000001111111111111111111_111111111111111111111111_11111111*10010010_010011000000000000000000_00000000000000000000000000000000
// Step  4160 A1 1LC: 00000000000000111111111111111111_111111111111111111111111_11111111*11001001_001001100000000000000000_00000000000000000000000000000000
// Step  4161 C1 0LE: 00000000000000011111111111111111_111111111111111111111111_11111111*10100100_100100110000000000000000_00000000000000000000000000000000
// Step  4162 E1 0LA: 00000000000000001111111111111111_111111111111111111111111_11111111*10010010_010010011000000000000000_00000000000000000000000000000000
// ...
// Step  4261 E1 0LA: 00000000000000000000000000000000_000000000000000000000000_000000[11*1]0010010_010010010010010010010010_01001001001001001001001001001001
// Step  4262 A1 1LC: 00000000000000000000000000000000_000000000000000000000000_00000001*11001001_001001001001001001001001_00100100100100100100100100100100
// Step  4263 C1 0LE: 00000000000000000000000000000000_000000000000000000000000_00000000*10100100_100100100100100100100100_10010010010010010010010010010010
// Step  4264 E1 0LA: 00000000000000000000000000000000_000000000000000000000000_00000000*0[001]0010_010010010010010010010010_01001001001001001001001001001001
//   no more 111
// Step  4265 A0 1RB: 00000000000000000000000000000000_000000000000000000000000_00000001*00100100_100100100100100100100100_10010010010010010010010010010010
// Step  4266 B0 1RC: 00000000000000000000000000000000_000000000000000000000000_00000011*01001001_001001001001001001001001_00100100100100100100100100100100
// Step  4267 C0 1RD: 00000000000000000000000000000000_000000000000000000000000_00000111*10010010_010010010010010010010010_01001001001001001001001001001000
// Step  4271 D1 1LD: 00000000000000000000000000000000_000000000000000000000000_00000000*01111001_001001001001001001001001_00100100100100100100100100100100
// Step  4272 D0 1LA: 00000000000000000000000000000000_000000000000000000000000_00000000*01111100_100100100100100100100100_10010010010010010010010010010010

use std::fmt::Display;
#[cfg(feature = "bb_enable_html_reports")]
use std::{fs::File, io::Write, path::MAIN_SEPARATOR_STR};

#[cfg(feature = "bb_enable_html_reports")]
use crate::html;
#[cfg(all(debug_assertions, feature = "bb_debug"))]
use crate::tape_utils::{VecU32Ext, TAPE_DISPLAY_RANGE_128};
use crate::{
    config::{
        Config, IdBig, StepTypeBig, StepTypeSmall, MAX_TAPE_GROWTH_BLOCKS,
        TAPE_SIZE_INIT_CELL_BLOCKS,
    },
    decider::{self, Decider, DECIDER_HOLD_ID},
    decider_result::BatchData,
    machine::Machine,
    status::{MachineStatus, UndecidedReason},
    tape_utils::{
        CLEAR_HIGH95_64BITS_U128, CLEAR_LOW63_00BITS_U128, CLEAR_LOW63_32BITS_U128,
        HIGH32_SWITCH_U128, LOW32_SWITCH_U128, MIDDLE_BIT_U128, POS_HALF_U128,
        TAPE_SIZE_FOURTH_UPPER_128, TAPE_SIZE_HALF_128, TL_POS_START_128,
    },
    transition_symbol2::{TransitionSymbol2, TransitionTableSymbol2, TRANSITION_SYM2_START},
    ResultUnitEndReason,
};

/// This decider runs on a 128-Bit number and moves data out to a long tape (Vec). \
/// The tape is not limited in size other than Vec memory limitations.
/// Usage:
/// # Set step_limit and tape_size_limit in the machine to evaluate
/// # Create a new decider for a single machine
/// # Run run_check_hold to check if the machine holds
// TODO Longer jump if multiple u32 in tape_long are FFFF
// TODO Multiple repeating steps, e.g 3 on 001
// TODO version with output tape, visualize
// TODO performance html: keep 1000 lines in memory, then write
// TO DO many steps: stop after limit, but write last 1000 lines (this is difficult without creating the lines anyway)
// TO DO speedup u64 than handover? Probably only very small gain
// pub struct DeciderU128Long<'a> {
pub struct DeciderHoldU128Long {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    tape_shifted: u128,
    /// Indication where the original pos_middle has moved within tape_shifted. Used to load data from long_tape.
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
    /// High bound in tape_long, this is the rightmost value.
    tl_high_bound: usize,
    /// TODO low bound in bit, this is the rightmost doubleword (16-bit) in tape_shifted (bit 0), min value is 0, but will be negative when testing.
    /// Low bound in tape_long, this is the leftmost value.
    tl_low_bound: usize,
    num_steps: StepTypeBig,
    tr_field_id: usize,
    tr: TransitionSymbol2,
    // machine id, just for debugging
    machine_id: IdBig,
    transition_table: TransitionTableSymbol2,
    #[allow(dead_code)]
    status: MachineStatus,
    step_limit: StepTypeBig,
    #[cfg(feature = "bb_enable_html_reports")]
    write_html_line_limit: u32,
    #[cfg(feature = "bb_enable_html_reports")]
    path: String,
    #[cfg(feature = "bb_enable_html_reports")]
    file: Option<File>,
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
            tr_field_id: 2,
            // Initialize transition with A0 as start
            tr: TRANSITION_SYM2_START,
            // copy the transition table as this runs faster
            machine_id: 0,
            transition_table: TransitionTableSymbol2::default(),
            status: MachineStatus::NoDecision,
            step_limit: config.step_limit_hold(),
            #[cfg(feature = "bb_enable_html_reports")]
            write_html_line_limit: if config.write_html_file() {
                config.write_html_line_limit()
            } else {
                0
            },
            #[cfg(feature = "bb_enable_html_reports")]
            path: Self::get_html_path(config.write_html_file(), config.n_states()),
            #[cfg(feature = "bb_enable_html_reports")]
            file: None,
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
        self.tr_field_id = 2;
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

    // /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps. \
    // /// This version has a long tape, so it is not restricted to the 128 bit range.
    // /// This will use self_referencing_transition speed-up if available.
    // fn decide_machine_hold(&mut self, machine: &Machine) -> MachineStatus {
    //     self.clear();
    //     self.id = machine.id();
    //     self.transition_table = *machine.transition_table();
    //     let result = if cfg!(feature = "bb_no_self_ref") {
    //         self.run_check_hold_without_self_referencing_transitions()
    //     } else if self
    //         .transition_table
    //         .eval_set_has_self_referencing_transition()
    //     {
    //         self.run_check_hold_with_self_referencing_transition()
    //     } else {
    //         self.run_check_hold_without_self_referencing_transitions()
    //     };
    //     result
    // }

    fn decide_machine_with_self_referencing_transition(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            self.num_steps += 1;
            self.tr_field_id = self.tr.state_x2() + self.current_symbol();
            self.tr = self.transition_table.transition(self.tr_field_id);
            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            println!("{}", self.step_to_string());

            // check if done
            if self.tr.is_hold() {
                // write last symbol
                if !self.tr.is_symbol_undefined() {
                    self.set_current_symbol();
                }
                self.status = MachineStatus::DecidedHolds(self.num_steps);
                // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html_line_limit > 0 {
                    self.write_step_html();
                    self.write_html_p(
                        format!("Decided: Holds after {} steps.", self.num_steps).as_str(),
                    );
                }
                return self.status;
            } else if self.num_steps > self.step_limit {
                self.status = self.undecided_step_limit();
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html_line_limit > 0 {
                    self.write_step_html();
                    self.write_html_p(
                        format!("Undecided: Limit of {} steps reached.", self.step_limit).as_str(),
                    );
                }
                return self.status;
            }

            if self.tr.is_dir_right() {
                // normal shift RIGHT -> tape moves left

                // Check if self referencing, which speeds up the shift greatly.
                // Self referencing means also that the symbol does not change, ergo no need to update the fields
                if self.tr.array_id() == self.tr_field_id {
                    // get jump within tape_shifted, which is only the lower part and thus a maximum of 63 bits
                    let mut jump = self.count_right(self.tr_field_id & 1) as usize;
                    // if self.num_steps > 50_000 {
                    //     // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    //     println!("  jump R {jump}, {}", self.step_to_string());
                    // }
                    // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                    // Interestingly, the version with the long_jump logic runs faster.
                    let mut long_jump = false;
                    if jump == 32 && self.pos_middle + jump == HIGH32_SWITCH_U128 {
                        // check further for larger jump
                        // compare depending on symbol
                        let v32 = if self.tr_field_id & 1 == 0 {
                            0
                        } else {
                            u32::MAX
                        };
                        // head goes right, tape shifts left
                        // tl_pos + 2 is now a known required value v32, because that is what count_right just tested
                        let mut p = self.tl_pos + 3;
                        let mut j = 1;
                        while p <= self.tl_high_bound && self.tape_long[p] == v32 {
                            p += 1;
                            j += 1;
                        }
                        // j is one more as the first one is already checked with count_right
                        if j >= 2 {
                            // if tape_shifted_left_0 != v32 {
                            //     println!("Not v32 {v32} but {tape_shifted_left_0}");
                            // }
                            // println!(
                            //     "Step {}: Long jump = {j} u32 = {} bits",
                            //     self.num_steps,
                            //     j * 32
                            // );
                            // shift out high bit after moving 32 bit
                            let tape_shifted_left_1 = (self.tape_shifted >> 64) as u32;
                            self.tape_long[self.tl_pos + 1] = tape_shifted_left_1;
                            self.tl_pos = p - 3;
                            // println!("before {}", self.tape_shifted.to_binary_split_string());
                            self.tape_shifted = if self.tr_field_id & 1 == 0 {
                                0
                            } else {
                                CLEAR_LOW63_00BITS_U128
                            };
                            // println!("filled {}", self.tape_shifted.to_binary_split_string());
                            self.pos_middle = HIGH32_SWITCH_U128;
                            self.num_steps += j * 32 - 1;
                            // shift in low bits (low part is already cleared)
                            self.tape_shifted |= (self.tape_long[self.tl_pos + 3] as u128) << 32;
                            // println!("fill 2 {}", self.tape_shifted.to_binary_split_string());
                            long_jump = true;
                        }
                        //                         else {
                        //                             self.pos_middle += jump;
                        //
                        //                             // shift tape
                        //                             // self.set_current_symbol();
                        //                             self.tape_shifted <<= jump;
                        //                             self.num_steps += jump as StepTypeBig - 1;
                        //                         }
                    }
                    if !long_jump {
                        if self.pos_middle + jump > HIGH32_SWITCH_U128 {
                            jump = HIGH32_SWITCH_U128 - self.pos_middle;
                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            println!("  jump right adjusted {jump}");
                        }
                        self.pos_middle += jump;

                        // shift tape
                        // self.set_current_symbol();
                        self.tape_shifted <<= jump;
                        self.num_steps += jump as StepTypeBig - 1;
                    }
                    // #[cfg(feature = "bb_enable_html_reports")]
                    // if self.write_html_step_limit > 0 {
                    //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                    //     let s = format!(
                    //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                    //         self.num_steps,
                    //         self.tl_pos,
                    //         self.tape_long[tl_pos_min_1],
                    //         self.tape_long[self.tl_pos],
                    //         self.tape_long[self.tl_pos + 1],
                    //         self.tape_long[self.tl_pos + 2],
                    //         self.tape_long[self.tl_pos + 3],
                    //         self.tape_long[self.tl_pos + 4],
                    //     );
                    //     self.write_html_p(&s);
                    // }
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
                if self.tr.array_id() == self.tr_field_id {
                    let mut jump = self.count_left(self.tr_field_id & 1) as usize;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump left {jump}");
                    // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                    // Interestingly, the version with the long_jump logic runs faster.
                    let mut long_jump = false;
                    if jump == 33 && LOW32_SWITCH_U128 - 1 + jump == self.pos_middle {
                        // check further for larger jump
                        // compare depending on symbol
                        let v32 = if self.tr_field_id & 1 == 0 {
                            0
                        } else {
                            u32::MAX
                        };
                        // head goes left, tape shifts right
                        // tl_pos + 1 is known required value v32, because that is what count_left just tested
                        let mut p = self.tl_pos;
                        let mut j = 1;
                        while p >= self.tl_low_bound && self.tape_long[p] == v32 {
                            p -= 1;
                            j += 1;
                        }
                        // j is one more as the first one is already checked with count_right
                        if j >= 2 {
                            // if tape_shifted_left_0 != v32 {
                            //     println!("Not v32 {v32} but {tape_shifted_left_0}");
                            // }
                            // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            // println!(
                            //     "Step {}: Long jump = {j} u32 = {} bits",
                            //     self.num_steps,
                            //     j * 32
                            // );
                            // shift out low bit after moving 32 bit
                            let tape_shifted_left_2 = (self.tape_shifted >> 32) as u32;
                            self.tape_long[self.tl_pos + 2] = tape_shifted_left_2;
                            self.tl_pos = p;
                            // println!("before {}", self.tape_shifted.to_binary_split_string());
                            self.tape_shifted = if self.tr_field_id & 1 == 0 {
                                0
                            } else {
                                u64::MAX as u128
                            };
                            // println!("filled {}", self.tape_shifted.to_binary_split_string());
                            self.pos_middle = LOW32_SWITCH_U128;
                            self.num_steps += j * 32 - 1;
                            // shift in high bits (high part is already cleared)
                            self.tape_shifted |=
                                (self.tape_long[self.tl_pos] as u128) << TAPE_SIZE_HALF_128;
                            // println!("fill 2 {}", self.tape_shifted.to_binary_split_string());
                            long_jump = true;
                        }
                        //                         else {
                        //                             self.pos_middle += jump;
                        //
                        //                             // shift tape
                        //                             // self.set_current_symbol();
                        //                             self.tape_shifted <<= jump;
                        //                             self.num_steps += jump as StepTypeBig - 1;
                        //                         }
                    }
                    if !long_jump {
                        if self.pos_middle < LOW32_SWITCH_U128 + jump {
                            jump = self.pos_middle - LOW32_SWITCH_U128;
                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            println!("  jump left adjusted {jump}");
                        }
                        self.pos_middle -= jump;

                        // self.set_current_symbol();
                        // shift tape
                        self.tape_shifted >>= jump;
                        self.num_steps += jump as StepTypeBig - 1;
                    }
                    // #[cfg(feature = "bb_enable_html_reports")]
                    // if self.write_html_step_limit > 0 && self.num_steps < self.write_html_step_limit
                    // {
                    //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                    //     let s = format!(
                    //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                    //         self.num_steps,
                    //         self.tl_pos,
                    //         self.tape_long[tl_pos_min_1],
                    //         self.tape_long[self.tl_pos],
                    //         self.tape_long[self.tl_pos + 1],
                    //         self.tape_long[self.tl_pos + 2],
                    //         self.tape_long[self.tl_pos + 3],
                    //         self.tape_long[self.tl_pos + 4],
                    //     );
                    //     self.write_html_p(&s);
                    // }
                } else {
                    self.pos_middle -= 1;

                    self.set_current_symbol();
                    // shift tape
                    self.tape_shifted >>= 1;
                }

                if self.pos_middle == LOW32_SWITCH_U128 {
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
            {
                if self.num_steps % 100 == 0 {
                    println!();
                }
                println!("{}", self.step_to_string());
            }
            #[cfg(feature = "bb_enable_html_reports")]
            // TODO this is actually incorrect, but this decider is deprecated
            if self.write_html_line_limit > 0 && self.num_steps <= self.write_html_line_limit {
                self.write_step_html();
            }
        }
    }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps. \
    /// This version has a long tape, so it is not restricted to the 128 bit range.
    /// This is not using the self reference speed-up and should only be used if those would mess up the tests.
    fn decide_machine_without_self_referencing_transitions(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            self.num_steps += 1;
            self.tr_field_id = self.tr.state_x2() + self.current_symbol();
            self.tr = self.transition_table.transition(self.tr_field_id);
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
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html_line_limit > 0 {
                    self.write_step_html();
                    self.write_html_p(
                        format!("Decided: Holds after {} steps.", self.num_steps).as_str(),
                    );
                }
                return self.status;
            } else if self.num_steps > self.step_limit {
                self.status = self.undecided_step_limit();
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html_line_limit > 0 {
                    self.write_step_html();
                    self.write_html_p(
                        format!("Undecided: Limit of {} steps reached.", self.step_limit).as_str(),
                    );
                }
                return self.status;
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
                if self.pos_middle == LOW32_SWITCH_U128 {
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
            {
                if self.num_steps % 100 == 0 {
                    println!();
                }
                println!("{}", self.step_to_string());
            }
            #[cfg(feature = "bb_enable_html_reports")]
            if self.write_html_line_limit > 0 && self.num_steps <= self.write_html_line_limit {
                self.write_step_html();
            }
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
    pub fn count_ones(&self) -> StepTypeSmall {
        let mut ones = self.tape_shifted.count_ones();
        if self.tl_high_bound - self.tl_low_bound > 3 {
            for n in self.tape_long[self.tl_low_bound..self.tl_pos].iter() {
                ones += n.count_ones();
            }
            for n in self.tape_long[self.tl_pos + 4..self.tl_high_bound + 1].iter() {
                ones += n.count_ones();
            }
        }
        ones as StepTypeSmall
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

    #[cfg(feature = "bb_enable_html_reports")]
    fn get_html_path(write_html: bool, n_states: usize) -> String {
        if write_html {
            let p = format!(
                "{}{}{}{n_states}",
                Config::get_result_path(),
                MAIN_SEPARATOR_STR,
                "hold_bb",
            );
            html::create_css(&p).expect("CSS files could not be created.");
            p
        } else {
            String::new()
        }
    }

    /// Shifts the pos in the long tape one to left and checks Vec dimensions
    #[inline(always)]
    fn shift_pos_to_left_checked(&mut self) {
        // check if tape is long enough
        if self.tl_pos < self.tl_low_bound + 1 {
            if self.tl_pos == 0 {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 0 -> pos 40
                let grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len());
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
                let grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len());
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
            MachineStatus::DecidedHolds(steps) => {
                MachineStatus::DecidedHoldsDetail(steps, self.tape_size(), self.count_ones())
            }
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
    fn tape_size(&self) -> StepTypeSmall {
        ((self.tl_high_bound - self.tl_low_bound + 1) * 32) as StepTypeSmall
    }

    fn undecided_step_limit(&self) -> MachineStatus {
        MachineStatus::Undecided(
            UndecidedReason::StepLimit,
            self.num_steps as StepTypeBig,
            self.tape_size(),
        )
    }

    // / Update tape: write symbol at head position into cell.
    // / This is only required for handover from u64.
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

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_html_p(&mut self, text: &str) {
        if let Some(file) = self.file.as_mut() {
            writeln!(file, "<p>{text}</p>",).expect("Html file could not be written");
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_file_end(&mut self) {
        if let Some(file) = self.file.as_mut() {
            html::write_file_end(file).expect("Html file could not be written")
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_step_html(&mut self) {
        html::write_step_html_128(
            self.file.as_mut().unwrap(),
            self.num_steps,
            self.tr_field_id,
            self.tr,
            self.tape_shifted,
            self.pos_middle,
        );
    }

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
}

impl Decider for DeciderHoldU128Long {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_HOLD_ID
    }

    // tape_long_bits in machine?
    // TODO counter: longest loop
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        self.clear();
        self.machine_id = machine.id();
        self.transition_table = *machine.transition_table();

        #[cfg(feature = "bb_enable_html_reports")]
        if self.write_html_line_limit > 0 {
            let (file, _f_name) =
                html::create_html_file_start(&self.path, Self::decider_id().name, machine)
                    .expect("Html file could not be written");
            self.file = Some(file);
            // file_name = f_name;
            self.write_html_p(
                "Note: Here only the 128 Bit Tape is shown. Whenever the tape 'jumps' a few bytes \
                    the working area needed to be shifted or previously shifted out data is reloaded.<br> \
                    'tape_long' stores the remaining tape.",
            );
            if self
                .transition_table
                .eval_set_has_self_referencing_transition()
            {
                self.write_html_p("Note: This machine has self-referencing transitions (e.g. Field A1: 1RA) \
                which leads to repeatedly calling itself in case of tape head reads 1. This is used to speed up the \
                decider by jumping over these repeated steps. Max jump is currently 32 steps.");
            }
        }

        let result = if cfg!(feature = "bb_no_self_ref") {
            self.decide_machine_without_self_referencing_transitions()
        } else if self
            .transition_table
            .eval_set_has_self_referencing_transition()
        {
            self.decide_machine_with_self_referencing_transition()
        } else {
            self.decide_machine_without_self_referencing_transitions()
        };

        #[cfg(feature = "bb_enable_html_reports")]
        if self.write_html_line_limit > 0 {
            if self.num_steps >= self.write_html_line_limit {
                self.write_html_p(
                    format!(
                        "HTML Step Limit ({}) reached, total steps: {}.",
                        self.write_html_line_limit, self.num_steps
                    )
                    .as_str(),
                );
            }
            self.write_file_end();
        }
        result
    }

    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }

    // fn new_from_self(&self) -> Self {
    //     todo!()
    // }
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

pub fn test_decider_hold_u128_applies_bb5_max() {
    let config = Config::new_default(5);
    // let config = Config::builder(5)
    //     .write_html_file(true)
    //     .write_html_step_limit(50_000_000)
    //     // .step_limit_hold(5_000_000)
    //     .build();
    // BB5 Max
    let machine = Machine::build_machine("BB5_MAX").unwrap();
    let check_result = DeciderHoldU128Long::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_hold_u128_applies_bb4_max() {
        let config = Config::new_default(4);
        // let config = Config::builder(4).write_html_file(true).build();

        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let mut d = DeciderHoldU128Long::new(&config);
        let check_result = d.decide_machine(&machine);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    /// This test runs 50 mio steps, so turn off default = ["bb_debug"].
    fn decider_hold_u128_applies_bb5_max() {
        let config = Config::new_default(5);
        // let config = Config::builder(5)
        //     .write_html_file(false)
        //     .write_html_step_limit(1_000_000)
        //     // .step_limit_hold(1_000_000)
        //     .build();
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let check_result = DeciderHoldU128Long::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(47_176_870));
    }
}
