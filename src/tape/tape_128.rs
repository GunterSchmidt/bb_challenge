//! Tape128 is a tape which only uses a 128-Bit tape_shifted and as such has a limited tape size. \
//! It has a small memory footprint and only uses stack memory, which makes it a bit faster.
// TODO self ref

use std::fmt::Display;

use crate::{
    config::{Config, StepTypeBig},
    tape::{
        tape_utils::{
            TapeLongPositions, U128Ext, FILTER_HIGH_BITS_INCLUDING_HEAD_U128, FILTER_LOW_BITS_U128,
            MIDDLE_BIT_U128, POS_HALF_U128, TAPE_SIZE_BIT_U128,
        },
        Tape, TapeSpeedUp,
    },
    transition_binary::TransitionBinary,
};

/// This tape was designed to be more performant than the tape_long on the first steps as tape_long does not need
/// to be maintained. It turns out it does not speed up anything, since tape_long is not used anyhow in these cases. \
/// There are a few edge cases but these are less than 1% (of the already found cases) and make it irrelevant. \
/// It is kept for comparison reasons.
#[derive(Debug)]
pub struct Tape128 {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT. \
    /// The tape is 128 bit wide and cannot extend. The used section will shrink to the outmost one to use the
    /// tape as far as possible. In turn the tape size is not exact. \
    tape_shifted: u128,
    /// Indication where the original pos_middle has moved within tape_shifted. Used to find boundaries.
    pos_middle: u32,
    /// High bound in u128-tape, this is the leftmost bit which is a 1.
    high_bound: u32,
    /// Low bound in u128-tape, this is the rightmost bit which is a 1.
    low_bound: u32,
}

impl Tape128 {
    /// Counts ones/zeros for self referencing speed-up
    #[inline(always)]
    pub fn count_left(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = (self.tape_shifted >> 64) as u64;
        if symbol == 1 {
            t.trailing_ones() + 1
        } else {
            t.trailing_zeros() + 1
        }
    }

    /// Counts ones/zeros for self referencing speed-up
    #[inline(always)]
    pub fn count_right(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = self.tape_shifted as u64;
        if symbol == 1 {
            t.leading_ones()
        } else {
            t.leading_zeros()
        }
    }
}

impl Tape for Tape128 {
    fn new(_config: &Config) -> Self {
        Self::default()
    }

    /// resets the decider for a different machine
    #[inline(always)]
    fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128;
        self.low_bound = MIDDLE_BIT_U128;
        self.high_bound = MIDDLE_BIT_U128;
    }

    /// Returns the ones which are set in the tape.
    fn count_ones(&self) -> u32 {
        self.tape_shifted.count_ones()
    }

    #[inline(always)]
    fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
    }

    fn is_left_empty(&self) -> bool {
        // let x = self.tape_shifted & crate::tape_utils::FILTER_HIGH_BITS_INCLUDING_HEAD_U128;
        // println!("{}", x.to_binary_split_string());
        self.tape_shifted & FILTER_HIGH_BITS_INCLUDING_HEAD_U128 == 0
    }

    fn is_right_empty(&self) -> bool {
        self.tape_shifted & FILTER_LOW_BITS_U128 == 0
    }

    fn left_64_bit(&self) -> u64 {
        (self.tape_shifted >> 64) as u64
    }

    fn right_64_bit(&self) -> u64 {
        self.tape_shifted as u64
    }

    #[cfg(feature = "enable_html_reports")]
    fn pos_middle_print(&self) -> i64 {
        self.pos_middle as i64
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn set_current_symbol(&mut self, transition: TransitionBinary) {
        if transition.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    // fn supports_speed_up(&self) -> bool {
    //     true
    // }

    fn tape_long_positions(&self) -> Option<TapeLongPositions> {
        None
    }

    #[cfg(feature = "enable_html_reports")]
    fn tape_shifted_clean(&self) -> u128 {
        self.tape_shifted
    }

    /// Returns the approximate tape size, which is actually not known exactly. \
    /// The high/low bound may indicate the actual used tape or may have shifted to the first 1 in that direction.
    #[inline(always)]
    fn tape_size_cells(&self) -> u32 {
        // if self.low_bound >= 0 {
        self.high_bound - self.low_bound + 1
        // } else {
        //     self.high_bound + 1
        // }
    }

    /// Sets the symbol of the transition and moves the tape according to direction of the transition.
    /// # Returns
    /// False if the tape bounds were reached.
    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionBinary) -> bool {
        self.set_current_symbol(transition);

        // shift tape
        self.tape_shifted = if transition.is_dir_right() {
            self.pos_middle += 1;
            if self.high_bound == TAPE_SIZE_BIT_U128 - 1 {
                // Use the tape fully, which means shifted 0 is not relevant, wait until first 1 moves over tape limit
                let zeros = self.tape_shifted.leading_zeros();
                if zeros >= 64 {
                    // dbg!(zeros, self.high_bound);
                    self.high_bound = MIDDLE_BIT_U128.max(self.pos_middle);
                } else if zeros > 0 {
                    self.high_bound = (self.high_bound - zeros)
                        .max(self.pos_middle)
                        .max(MIDDLE_BIT_U128);
                    #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                    {
                        println!("High bound extended by {zeros}.");
                        println!("{self}");
                    }
                } else {
                    #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                    {
                        println!("High bound reached: Too many steps to right.");
                        println!("{self}");
                    }
                    return false;
                }
            } else {
                self.high_bound += 1;
            }
            // TODO Does it really matter if low_bound passes middle bit?
            if self.low_bound < MIDDLE_BIT_U128 {
                self.low_bound += 1;
            }
            self.tape_shifted << 1
        } else {
            // transition goes left
            if self.pos_middle == 0 {
                return false;
            }
            self.pos_middle -= 1;
            if self.low_bound == 0 {
                #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                {
                    println!("Low bound reached: Too many steps to left.");
                    println!("{self}");
                }
                // Use the tape fully, which means shifted 0 is not relevant, wait until first 1 moves over tape limit
                let zeros = self.tape_shifted.trailing_zeros();
                if zeros >= 64 {
                    // dbg!(zeros, self.high_bound);
                    self.low_bound = MIDDLE_BIT_U128.min(self.pos_middle);
                } else if zeros > 0 {
                    self.low_bound = (self.low_bound + zeros)
                        .min(self.pos_middle)
                        .min(MIDDLE_BIT_U128);
                    #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                    {
                        println!("High bound extended by {zeros}.");
                        println!("{self}");
                    }
                } else {
                    #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                    {
                        println!("High bound reached: Too many steps to right.");
                        println!("{self}");
                    }
                    return false;
                }
            } else {
                self.low_bound -= 1;
            }
            // TODO Does it really matter if high_bound passes middle bit?
            if self.high_bound > MIDDLE_BIT_U128 {
                self.high_bound -= 1;
            }
            self.tape_shifted >> 1
        };

        true
    }

    fn update_tape_self_ref_speed_up_unused_or_used(
        &mut self,
        transition: TransitionBinary,
    ) -> bool {
        todo!()
    }
}

impl TapeSpeedUp for Tape128 {
    #[inline(always)]
    fn update_tape_self_ref_speed_up(
        &mut self,
        tr: TransitionBinary,
        tr_field: usize,
    ) -> StepTypeBig {
        let jump;
        // Check if self referencing, which speeds up the shift greatly.
        // Self referencing means also that the symbol does not change, ergo no need to update the fields
        if tr.self_ref_array_id() == tr_field {
            if tr.is_dir_right() {
                // normal shift RIGHT -> tape moves left

                // get jump within tape_shifted, which is only the lower part and thus a maximum of 63 bits
                jump = self.count_right(tr_field & 1);
                // The content is either always 0 or always 1
                // TODO How can this be possible?
                if self.high_bound + jump > TAPE_SIZE_BIT_U128 - 1 {
                    // The move would now repeated till end of tape, therefore the tape size limit has been reached.
                    // jump = TAPE_SIZE_BIT_U128 - 1 - self.high_bound;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump right {jump}");
                    return 0;
                }
                // shift tape
                // self.set_current_symbol(); not required
                self.tape_shifted <<= jump;
                self.pos_middle += jump;
                if self.tape_shifted == 0 {
                    self.high_bound = self.pos_middle.max(MIDDLE_BIT_U128);
                } else {
                    self.high_bound = (127 - self.tape_shifted.leading_zeros())
                        .max(self.pos_middle)
                        .max(MIDDLE_BIT_U128);
                }
                self.low_bound += jump.min(63);

                // self.step_no += jump as StepTypeBig - 1;
                // jump -= 1;
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
                // normal shift LEFT -> tape moves right

                // Check if self referencing, which speeds up the shift greatly.
                jump = self.count_left(tr_field & 1);
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                // The content is either always 0 or always 1
                if self.low_bound < jump {
                    // jump = self.low_bound as u32;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!("  jump left adjusted {jump}");
                    return 0;
                }
                self.tape_shifted >>= jump;
                self.pos_middle -= jump;
                if self.tape_shifted == 0 {
                    self.high_bound = self.pos_middle.max(MIDDLE_BIT_U128);
                } else {
                    self.high_bound = (127 - self.tape_shifted.leading_zeros())
                        .max(MIDDLE_BIT_U128)
                        .max(self.pos_middle);
                }
                self.low_bound -= jump;

                // self.set_current_symbol(); not required
                // shift tape
                // self.step_no += jump as StepTypeBig - 1;
                // jump -= 1;
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
            };
        } else {
            let r = self.update_tape_single_step(tr);
            jump = r as StepTypeBig;
        }
        jump
    }
}

impl Default for Tape128 {
    fn default() -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128,
            low_bound: MIDDLE_BIT_U128,
            high_bound: MIDDLE_BIT_U128,
        }
    }
}

impl Display for Tape128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} P{:3}, H {}, L {}",
            self.tape_shifted.to_binary_split_string(),
            self.pos_middle,
            self.high_bound,
            self.low_bound
        )
    }
}
