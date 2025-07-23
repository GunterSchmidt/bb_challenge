//! This crate hold a tape struct for 128-Bit tape_shifted with maximum usage. \

use std::fmt::Display;

use crate::{
    config::Config,
    tape::Tape,
    tape_utils::{U128Ext, MIDDLE_BIT_U128, POS_HALF_U128, TAPE_SIZE_BIT_U128},
    transition_symbol2::TransitionSymbol2,
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
    low_bound: i32,
}

impl Tape for Tape128 {
    fn new(_config: &Config) -> Self {
        Self::default()
    }

    /// resets the decider for a different machine
    #[inline(always)]
    fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128 as u32;
        self.low_bound = MIDDLE_BIT_U128 as i32;
        self.high_bound = MIDDLE_BIT_U128 as u32;
    }

    // Returns the ones which are set in the tape.
    fn count_ones(&self) -> u32 {
        self.tape_shifted.count_ones()
    }

    #[inline(always)]
    fn get_current_symbol(&self) -> u32 {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as u32
    }

    fn is_left_empty(&self) -> bool {
        // let x = self.tape_shifted & crate::tape_utils::FILTER_HIGH_BITS_INCLUDING_HEAD_U128;
        // println!("{}", x.to_binary_split_string());
        self.tape_shifted & crate::tape_utils::FILTER_HIGH_BITS_INCLUDING_HEAD_U128 == 0
    }

    fn is_right_empty(&self) -> bool {
        self.tape_shifted & crate::tape_utils::FILTER_LOW_BITS_U128 == 0
    }

    fn left_64_bit(&self) -> u64 {
        (self.tape_shifted >> 64) as u64
    }

    fn right_64_bit(&self) -> u64 {
        self.tape_shifted as u64
    }

    fn pos_middle(&self) -> u32 {
        self.pos_middle
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn set_current_symbol(&mut self, transition: TransitionSymbol2) {
        if transition.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    fn tape_long_positions(&self) -> Option<crate::tape_utils::TapeLongPositions> {
        None
    }

    fn tape_shifted_clean(&self) -> u128 {
        self.tape_shifted
    }

    /// Returns the approximate tape size, which is actually not known exactly. \
    /// The high/low bound may indicate the actual used tape or may have shifted to the first 1 in that direction.
    #[inline(always)]
    fn tape_size_cells(&self) -> u32 {
        self.high_bound - self.low_bound as u32 + 1
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape bounds were reached.
    #[must_use]
    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionSymbol2) -> bool {
        self.set_current_symbol(transition);

        // shift tape
        self.tape_shifted = if transition.is_dir_right() {
            self.pos_middle += 1;
            if self.high_bound == (TAPE_SIZE_BIT_U128 - 1) as u32 {
                // Use the tape fully, which means shifted 0 is not relevant, wait until first 1 moves over tape limit
                let zeros = self.tape_shifted.leading_zeros();
                if zeros >= 64 {
                    // dbg!(zeros, self.high_bound);
                    self.high_bound = MIDDLE_BIT_U128;
                } else if zeros > 0 {
                    self.high_bound -= zeros;
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
            }
            // adding high bound here, so i8 will not overflow
            self.high_bound += 1;
            // TODO Does it really matter if low_bound passes middle bit?
            if self.low_bound < MIDDLE_BIT_U128 as i32 {
                self.low_bound += 1;
            }
            self.tape_shifted << 1
        } else {
            self.pos_middle -= 1;
            self.low_bound -= 1;
            if self.low_bound == -1 {
                #[cfg(all(debug_assertions, feature = "bb_debug_tape"))]
                {
                    println!("Low bound reached: Too many steps to left.");
                    println!("{self}");
                }
                // Use the tape fully, which means shifted 0 is not relevant, wait until first 1 moves over tape limit
                let zeros = self.tape_shifted.trailing_zeros();
                if zeros >= 64 {
                    // dbg!(zeros, self.high_bound);
                    self.low_bound = MIDDLE_BIT_U128 as i32;
                } else if zeros > 0 {
                    self.low_bound += zeros as i32;
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
            }
            // TODO Does it really matter if high_bound passes middle bit?
            if self.high_bound > MIDDLE_BIT_U128 as u32 {
                self.high_bound -= 1;
            }
            self.tape_shifted >> 1
        };

        true
    }
}

impl Default for Tape128 {
    fn default() -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128 as u32,
            low_bound: MIDDLE_BIT_U128 as i32,
            high_bound: MIDDLE_BIT_U128 as u32,
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
