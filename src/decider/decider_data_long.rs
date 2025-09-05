//! DeciderData carries all data which is not specific to the decider. \
//! It holds the tape and the tape movement, the current step_no, the status and
//! functionality to write a HTML file. \
//! This allows to switch the tapes easily in the decider and the HTML logic does not need to be repeated.

use std::fmt::Display;

use crate::{
    config::{Config, StepTypeBig},
    machine_binary::MachineBinary,
    status::{MachineStatus, UndecidedReason},
    tape::tape_long_shifted::TapeLongShifted,
    tape::tape_utils::{
        self, CLEAR_LOW63_00BITS_U128, HIGH32_SWITCH_U128, LOW32_SWITCH_U128, TAPE_SIZE_HALF_128,
    },
    tape::Tape,
    transition_binary::{TransitionBinary, TRANSITION_BINARY_FIRST},
};

/// This contains the functionality for a hold decider and can be used to create more elaborate deciders. \
#[derive(Debug)]
pub struct DeciderDataLong {
    // decider_id: &'static DeciderId,
    /// Number of steps or current step no, where first step is 1
    pub step_no: StepTypeBig,
    /// Current transition
    pub tr: TransitionBinary,
    /// Field Id of the current transition. This is the table field, e.g. B1 converted to a 1D-map (A0=2, B1=5).
    pub tr_field: usize,

    /// The tape_long is a ```Vec<u64>``` which allows to copy half of u128 tape_shifted to
    /// be copied into the long tape when a bound is reached.
    pub tape: TapeLongShifted,

    // machine id, just for debugging
    // machine_id: IdBig,
    pub transition_table: MachineBinary,

    /// Maximum number of steps, after that Undecided will be returned.
    pub step_limit: StepTypeBig,
    // /// Tape size limit in number of cells
    // tape_size_limit_u32_blocks: u32,
    /// Final status, only valid once machine has ended, but intended to be used internally.
    pub status: MachineStatus,
    /// HTML step limit limits output to file. Set to 0 if write_html_file is false.
    #[cfg(feature = "enable_html_reports")]
    pub html_writer: Option<crate::html::HtmlWriter>,
}

impl DeciderDataLong {
    // Sets the defaults and start transition A0.
    pub fn new(config: &Config) -> Self {
        Self {
            tape: TapeLongShifted::new(config),

            step_no: 0,
            transition_table: MachineBinary::default(),
            // Initialize transition with A0 as start
            tr: TRANSITION_BINARY_FIRST,
            tr_field: 2,
            status: MachineStatus::NoDecision,
            step_limit: config.step_limit_decider_halt(),

            #[cfg(feature = "enable_html_reports")]
            html_writer: if config.write_html_file() {
                Some(crate::html::HtmlWriter::new(config))
            } else {
                None
            },
        }
    }

    #[inline]
    // resets the decider for a different machine
    pub fn clear(&mut self) {
        self.tape.clear();

        self.step_no = 0;
        self.tr = TRANSITION_BINARY_FIRST;
        self.tr_field = 2;
        self.status = MachineStatus::NoDecision;
    }

    /// Reads the current symbol of the tape. Use with care, as this inspects data in the tape directly, which should generally be avoided.
    #[inline(always)]
    pub fn get_current_symbol(&self) -> usize {
        self.tape.get_current_symbol()
    }

    /// Sets the next transition and updates the step counter. It does not update the tape yet,
    /// but in the case the execution ended because of halt or limit.
    /// # Returns
    /// true if execution ended (is_done)
    #[must_use]
    #[inline(always)]
    pub fn next_transition(&mut self) -> bool {
        self.step_no += 1;
        self.tr_field = self.tr.state_x2() + self.tape.get_current_symbol();
        self.tr = self.transition_table.transition(self.tr_field);

        // print tape before change
        // #[cfg(all(debug_assertions, feature = "bb_debug"))]
        // println!("{}", self.step_to_string());
        self.is_done()
    }

    /// Checks if the decider is done.
    /// # Returns
    /// True when the decider ended for hold or step limit breach. In this case also self.status is set.
    #[must_use]
    #[inline(always)]
    pub fn is_done(&mut self) -> bool {
        if self.tr.is_halt() {
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.tape.set_current_symbol(self.tr);
                // self.tape.update_tape_single_step(self.tr);
            }
            // println!("{}", self.tl.tape_shifted.to_binary_split_string());
            self.status = MachineStatus::DecidedHalts(self.step_no);
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            #[cfg(feature = "enable_html_reports")]
            self.write_step_html();

            return true;
        } else if self.step_no >= self.step_limit {
            self.status = self.status_undecided_step_limit();
            #[cfg(feature = "enable_html_reports")]
            self.write_step_html();

            return true;
        }
        false
    }

    /// Returns true if html is enabled and the step_no is < 1000 or > config.write_html_step_start .
    /// step_no must be smaller or equal \
    /// line count must be smaller, so one more can fit
    #[cfg(feature = "enable_html_reports")]
    pub fn is_write_html_in_limit(&self) -> bool {
        if let Some(html_writer) = &self.html_writer {
            html_writer.is_write_html_in_limit(self.step_no)
        } else {
            false
        }
    }

    // #[cfg(feature = "enable_html_reports")]
    // pub fn rename_html_file_to_status(&self) {
    //     if let Some(html_writer) = &self.html_writer {
    //         if let Some(file_name) = html_writer.file_name() {
    //             let path = self.html_writer.as_ref().unwrap().path().unwrap();
    //             crate::html::rename_file_to_status(path, file_name, &self.status);
    //         }
    //     }
    // }

    fn status_undecided_step_limit(&self) -> MachineStatus {
        MachineStatus::Undecided(
            UndecidedReason::StepLimit,
            self.step_no as StepTypeBig,
            self.tape.tape_size_cells(),
        )
    }

    /// Returns the status of the decider
    pub fn status(&self) -> MachineStatus {
        self.status
    }

    /// Returns the status of the decider and additionally written Ones on tape and Tape Size
    pub fn status_full(&self) -> MachineStatus {
        match self.status {
            MachineStatus::DecidedHalts(steps) => MachineStatus::DecidedHaltsDetail(
                steps,
                self.tape.tape_size_cells(),
                self.tape.count_ones(),
            ),
            _ => self.status,
        }
    }

    // TODO implement
    // pub fn status_hold_details(&self) -> MachineStatus {
    //     MachineStatus::DecidedHoldsDetail(
    //         self.num_steps as StepType,
    //         self.get_tape_size(),
    //         self.tape_shifted.count_ones() as usize,
    //     )
    // }

    pub fn step_limit(&self) -> StepTypeBig {
        self.step_limit
    }

    // /// Returns a copy of the tape, which can be time consuming
    // pub fn tape_long(&self) -> TapeLong {
    //     TapeLong {
    //         tape_long: self.tape_long.to_vec(),
    //         tl_pos: self.tl_pos,
    //         tl_high_bound: self.tl_high_bound,
    //         tl_low_bound: self.tl_low_bound,
    //     }
    // }

    pub fn tape_shifted(&self) -> u128 {
        self.tape.tape_shifted
    }
    /// Updates tape_shifted and tape_long.
    /// Also prints and writes step to html if feature is set.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit). Then self.status is set to that error.
    // TODO move into tape
    #[must_use]
    #[inline(always)]
    pub fn update_tape_single_step(&mut self) -> bool {
        let shift_ok = self.tape.update_tape_single_step(self.tr);
        if !shift_ok {
            self.status = MachineStatus::Undecided(
                UndecidedReason::TapeSizeLimit,
                self.step_no,
                self.tape.tape_size_cells(),
            );
        }
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            if self.step_no % 100 == 0 {
                println!();
            }
            println!("{}", self.step_to_string());
        }
        #[cfg(feature = "enable_html_reports")]
        self.write_step_html();

        shift_ok
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit). Then self.status is set to that error.
    // TODO some of this logic should be moved to TapeLong
    #[must_use]
    #[inline(always)]
    pub fn update_tape_self_ref_speed_up(&mut self) -> bool {
        let shift_ok = if self.tr.is_dir_right() {
            // normal shift RIGHT -> tape moves left

            // Check if self referencing, which speeds up the shift greatly.
            // Self referencing means also that the symbol does not change, ergo no need to update the fields
            if self.tr.self_ref_array_id() == self.tr_field {
                // get jump within tape_shifted, which is only the lower part and thus a maximum of 63 bits
                let mut jump = self.tape.count_right(self.tr_field & 1);
                // if self.num_steps > 50_000 {
                //     // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                //     println!("  jump R {jump}, {}", self.step_to_string());
                // }
                // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                // Interestingly, the version with the long_jump logic runs faster.
                let mut long_jump = false;
                if jump == 32 && self.tape.pos_middle + jump == HIGH32_SWITCH_U128 {
                    // check further for larger jump
                    // compare depending on symbol
                    let v32 = if self.tr_field & 1 == 0 { 0 } else { u32::MAX };
                    // head goes right, tape shifts left
                    // tl_pos + 2 is now a known required value v32, because that is what count_right just tested
                    let mut p = self.tape.tl_pos() + 3;
                    let mut j = 1;
                    while p < self.tape.tl_high_bound() && self.tape.tape_long[p] == v32 {
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
                        let tape_shifted_left_1 = (self.tape.tape_shifted >> 64) as u32;
                        let p_tmp = self.tape.tl_pos() + 1;
                        self.tape.tape_long[p_tmp] = tape_shifted_left_1;
                        self.tape.set_tl_pos(p - 3);
                        // println!("before {}", self.tape_shifted.to_binary_split_string());
                        self.tape.tape_shifted = if self.tr_field & 1 == 0 {
                            0
                        } else {
                            CLEAR_LOW63_00BITS_U128
                        };
                        // println!("filled {}", self.tape_shifted.to_binary_split_string());
                        self.tape.pos_middle = HIGH32_SWITCH_U128;
                        self.step_no += j * 32 - 1;
                        // shift in low bits (low part is already cleared)
                        self.tape.tape_shifted |=
                            (self.tape.tape_long[self.tape.tl_pos() + 3] as u128) << 32;
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
                    if self.tape.pos_middle + jump > HIGH32_SWITCH_U128 {
                        jump = HIGH32_SWITCH_U128 - self.tape.pos_middle;
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  jump right adjusted {jump}");
                    }
                    self.tape.pos_middle += jump;

                    // shift tape
                    // self.set_current_symbol();
                    self.tape.tape_shifted <<= jump;
                    self.step_no += jump as StepTypeBig - 1;
                }
                // #[cfg(feature = "enable_html_reports")]
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
                self.tape.pos_middle += 1;

                // shift tape
                self.tape.set_current_symbol(self.tr);
                self.tape.tape_shifted <<= 1;
            }

            self.tape.shift_tape_long_head_dir_right()
        } else {
            // normal shift LEFT -> tape moves right

            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.self_ref_array_id() == self.tr_field {
                let mut jump = self.tape.count_left(self.tr_field & 1);
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                // Interestingly, the version with the long_jump logic runs faster.
                let mut long_jump = false;
                if jump == 33 && LOW32_SWITCH_U128 - 1 + jump == self.tape.pos_middle {
                    // check further for larger jump
                    // compare depending on symbol
                    let v32 = if self.tr_field & 1 == 0 { 0 } else { u32::MAX };
                    // head goes left, tape shifts right
                    // tl_pos + 1 is known required value v32, because that is what count_left just tested
                    let mut p = self.tape.tl_pos();
                    let mut j = 1;
                    while p >= self.tape.tl_low_bound() && self.tape.tape_long[p] == v32 {
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
                        let tape_shifted_left_2 = (self.tape.tape_shifted >> 32) as u32;
                        let p_tmp = self.tape.tl_pos() + 2;
                        self.tape.tape_long[p_tmp] = tape_shifted_left_2;
                        self.tape.set_tl_pos(p);
                        // println!("before {}", self.tape_shifted.to_binary_split_string());
                        self.tape.tape_shifted = if self.tr_field & 1 == 0 {
                            0
                        } else {
                            u64::MAX as u128
                        };
                        // println!("filled {}", self.tape_shifted.to_binary_split_string());
                        self.tape.pos_middle = LOW32_SWITCH_U128;
                        self.step_no += j * 32 - 1;
                        // shift in high bits (high part is already cleared)
                        self.tape.tape_shifted |=
                            (self.tape.tape_long[self.tape.tl_pos()] as u128) << TAPE_SIZE_HALF_128;
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
                    if self.tape.pos_middle < LOW32_SWITCH_U128 + jump {
                        jump = self.tape.pos_middle - LOW32_SWITCH_U128;
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  jump left adjusted {jump}");
                    }
                    self.tape.pos_middle -= jump;

                    // self.set_current_symbol();
                    // shift tape
                    self.tape.tape_shifted >>= jump;
                    self.step_no += jump as StepTypeBig - 1;
                }
                // #[cfg(feature = "enable_html_reports")]
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
                self.tape.pos_middle -= 1;

                self.tape.set_current_symbol(self.tr);
                // shift tape
                self.tape.tape_shifted >>= 1;
            }
            self.tape.shift_tape_long_head_dir_left()
        };
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            if self.step_no % 100 == 0 {
                println!();
            }
            println!("{}", self.step_to_string());
        }
        #[cfg(feature = "enable_html_reports")]
        self.write_step_html();
        if !shift_ok {
            if self.tape.tl_pos() >= self.tape.tape_long.len() {
                println!(
                    "\n *** Error shift: TL len {}, tl_pos {}, tl_high_bound {}, machine {}",
                    self.tape.tape_long.len(),
                    self.tape.tl_pos(),
                    self.tape.tl_high_bound(),
                    self.transition_table
                );
                return false;
            }
            self.status = MachineStatus::Undecided(
                UndecidedReason::TapeSizeLimit,
                self.step_no,
                self.tape.tape_size_cells(),
            );
        }
        shift_ok
    }

    // Creates
    #[cfg(feature = "enable_html_reports")]
    pub fn write_html_file_start(
        &mut self,
        decider_id: &super::DeciderId,
        machine: &MachineBinary,
    ) {
        if let Some(html_writer) = &mut self.html_writer {
            html_writer
                .create_html_file_start(decider_id, machine)
                .expect("Html file could not be written");
            self.write_html_p(
                "Note: Here only the 128 Bit Tape is shown, the underlying long tape holds more data.",
            );
        }
    }

    #[cfg(feature = "enable_html_reports")]
    pub fn write_html_file_end(&mut self) {
        if let Some(html_writer) = &mut self.html_writer {
            html_writer.write_html_file_end(self.step_no, &self.status);
        }
    }

    #[cfg(feature = "enable_html_reports")]
    pub fn write_html_p(&mut self, text: &str) {
        if let Some(html_writer) = &mut self.html_writer {
            html_writer.write_html_p(text);
        }
    }

    #[cfg(feature = "enable_html_reports")]
    pub fn write_step_html(&mut self) {
        if let Some(html_writer) = &self.html_writer {
            if html_writer.is_write_html_in_limit(self.step_no) {
                let step_data = crate::html::StepHtml::from(&*self);
                self.html_writer
                    .as_mut()
                    .unwrap()
                    .write_step_html(&step_data);
            }
        }
    }

    /// Debug info on current step
    pub fn step_to_string(&self) -> String {
        format!(
            "Step {:3} {} {}: {} P{}-{} Next {}{}",
            self.step_no,
            MachineBinary::array_id_to_field_name(self.tr_field),
            self.tr,
            tape_utils::U128Ext::to_binary_split_string(&self.tape.tape_shifted),
            self.tape.pos_middle,
            self.tape.tl_pos(),
            // self.get_tape_size(),
            self.tr.state_to_char(),
            self.tape.get_current_symbol(),
        )
    }
}

impl Display for DeciderDataLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO other fields
        write!(f, "{}", self.step_to_string(),)
    }
}

#[cfg(feature = "enable_html_reports")]
impl From<&DeciderDataLong> for crate::html::StepHtml {
    fn from(data: &DeciderDataLong) -> Self {
        let is_u128_tape = if let Some(html_writer) = &data.html_writer {
            !html_writer.write_html_tape_shifted_64_bit()
        } else {
            true
        };
        let tape_shifted = if is_u128_tape {
            data.tape.tape_shifted_clean()
        } else {
            data.tape.tape_shifted_clean() >> 32
        };
        Self {
            step_no: data.step_no,
            tr_field_id: data.tr_field,
            transition: data.tr,
            tape_shifted,
            is_u128_tape,
            pos_middle: data.tape.pos_middle_print(),
            tape_long_positions: data.tape.tape_long_positions(),
        }
    }
}
