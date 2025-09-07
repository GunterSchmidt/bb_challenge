use std::fmt::Display;

use crate::{
    config::{Config, StepBig},
    machine_binary::MachineBinary,
    status::{MachineStatus, UndecidedReason},
    tape::{tape_macro::TapeCompact, tape_utils::TAPE_SIZE_BIT_U128, Tape},
    transition_binary::{TransitionBinary, TRANSITION_BINARY_FIRST},
};

/// This contains the functionality for a hold decider and can be used to create more elaborate deciders. \
#[derive(Debug)]
pub struct DeciderDataMacro {
    // decider_id: &'static DeciderId,
    /// Number of steps or current step no, where first step is 1
    pub step_no: StepBig,
    /// Current transition
    pub tr: TransitionBinary,
    /// Field Id of the current transition. This is the table field, e.g. B1 converted to a 1D-map (A0=2, B1=5).
    pub tr_field: usize,

    /// The tape_long is a ```Vec<u64>``` which allows to copy half of u128 tape_shifted to
    /// be copied into the long tape when a bound is reached.
    pub tape: TapeCompact,

    // machine id, just for debugging
    // machine_id: IdBig,
    pub transition_table: MachineBinary,

    /// Maximum number of steps, after that Undecided will be returned.
    pub step_limit: StepBig,
    // /// Tape size limit in number of cells
    // tape_size_limit_u32_blocks: u32,
    /// Final status, only valid once machine has ended, but intended to be used internally.
    pub status: MachineStatus,
    /// HTML step limit limits output to file. Set to 0 if write_html_file is false.
    #[cfg(feature = "enable_html_reports")]
    pub html_writer: Option<crate::html::HtmlWriter>,
}

impl DeciderDataMacro {
    // Sets the defaults and start transition A0.
    pub fn new(config: &Config) -> Self {
        Self {
            // decider_id,
            tape: TapeCompact::new(config),

            step_no: 0,
            transition_table: MachineBinary::default(),
            // Initialize transition with A0 as start
            tr: TRANSITION_BINARY_FIRST,
            tr_field: 2,
            // copy the transition table as this runs faster
            // machine_id: 0,
            // transition_table: TransitionTableSymbol2::default(),
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
        // self.html_writer.reset_write_html_line_count();
        // keep step_limit and other config data
    }

    #[inline(always)]
    pub fn get_current_symbol(&self) -> usize {
        self.tape.get_current_symbol()
    }

    // Returns the next transition and updates the step counter, but does not update the tape yet
    #[inline(always)]
    pub fn next_transition(&mut self) -> bool {
        self.step_no += 1;
        self.tr_field = self.tr.state_x2() + self.tape.get_current_symbol();
        self.tr = self.transition_table.transition(self.tr_field);
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
            self.tape.write_last_symbol(self.tr);
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

    fn status_undecided_step_limit(&self) -> MachineStatus {
        MachineStatus::Undecided(
            UndecidedReason::StepLimit,
            self.step_no as StepBig,
            self.tape.tape_size_cells() as u32,
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
                self.tape.tape_size_cells() as u32,
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

    pub fn step_limit(&self) -> StepBig {
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

    // pub fn tape_long_positions(&self) -> TapeLongPositions {
    //     self.tape.tape_long_positions()
    // }

    // pub fn tape_shifted(&self) -> u128 {
    //     self.tape.tape_shifted()
    // }

    /// Updates tape_shifted and tape_long.
    /// Also prints and writes step to html if feature is set.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit). Then self.status is set to that error.
    #[must_use]
    #[inline(always)]
    pub fn update_tape_single_step(&mut self) -> bool {
        let shift_ok = self.tape.update_tape_single_step(self.tr);
        if !shift_ok {
            self.status = MachineStatus::Undecided(
                UndecidedReason::TapeSizeLimit,
                self.step_no,
                TAPE_SIZE_BIT_U128,
            );
        }
        // if self.step_no % 1_000_000 == 0 {
        //     println!(
        //         "Step: {}, Cells: {}",
        //         self.step_no,
        //         self.tape.to_full_string()
        //     );
        // }
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

    //     /// Updates tape_shifted and tape_long.
    //     /// # Returns
    //     /// False if the tape could not be expanded (tape_size_limit). Then self.status is set to that error.
    //     // TODO some of this logic should be moved to TapeLong
    //     #[must_use]
    //     #[inline(always)]
    //     pub fn update_tape_self_ref_speed_up(&mut self) -> bool {
    //         // if self.step_no > 1273 {
    //         //     println!();
    //         // }
    //         let jump = self
    //             .tape
    //             .update_tape_self_ref_speed_up(self.tr, self.tr_field);
    //         // return value
    //         if jump == 0 {
    //             self.status = MachineStatus::Undecided(
    //                 UndecidedReason::TapeSizeLimit,
    //                 self.step_no,
    //                 self.tape.tape_size_cells(),
    //             );
    //             false
    //         } else {
    //             self.step_no += jump - 1;
    //
    //             #[cfg(all(debug_assertions, feature = "bb_debug"))]
    //             {
    //                 if self.step_no % 100 == 0 {
    //                     println!();
    //                 }
    //                 println!("{}", self.step_to_string());
    //             }
    //             #[cfg(feature = "enable_html_reports")]
    //             self.write_step_html();
    //             true
    //         }
    //     }

    // Creates
    #[cfg(feature = "enable_html_reports")]
    pub fn write_html_file_start(
        &mut self,
        decider_id: &super::DeciderId,
        machine: &crate::machine_binary::MachineId,
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
            "Step {:3} {} {}: {}",
            self.step_no,
            MachineBinary::array_id_to_field_name(self.tr_field),
            self.tr,
            self.tape,
        )
    }
}

impl Display for DeciderDataMacro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO other fields
        write!(f, "{}", self.step_to_string(),)
    }
}

#[cfg(feature = "enable_html_reports")]
impl From<&DeciderDataMacro> for crate::html::StepHtml {
    fn from(data: &DeciderDataMacro) -> Self {
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
            tape_long_positions: None,
        }
    }
}
