//! This crate hold a tape struct for 128-Bit tape_shifted with maximum usage. \

use std::fmt::Display;

use crate::{
    config::Config,
    tape::Tape,
    tape_utils::{U128Ext, MIDDLE_BIT_U128, POS_HALF_U128, TAPE_SIZE_BIT_U128},
    transition_symbol2::TransitionSymbol2,
};

/// The `tape_long` is a `Vec<u32>` which allows to copy top or bottom 32 bit of u128 tape_shifted
/// into the long tape when a bound is reached.
/// TODO The tape has an initial size of e.g. 128 u64 which is 1024 Byte or 8192 tape cells.
/// The size will double every time its limit is reached. E.g it doubles 1x times to get a size of 256 or 16284 cells,
/// which is the size for BB5 Max tape length.
/// Once 131072 u64 is reached (1 MB), it will grow by 1 MB each time.
/// Here the head is moving within the tape, the tape does not shift at all.
// TODO limit access, pub removal
#[derive(Debug)]
pub struct Tape128 {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    /// The tape is 128 bit wide, but since data is shifted to the long tape, it may be 'dirty', meaning it
    /// does not contain the correct cell values. The cell values will be shifted in only if required. \
    /// In case the head goes left for a large number of steps, the bits next to the head will only be loaded
    /// one step before. The tape is clean as long as it never reached the outer limits.
    tape_shifted: u128,
    /// Indication where the original pos_middle has moved within tape_shifted. Used to load data from long_tape.
    pos_middle: u32,
    // /// Vec of u32 blocks, where each u32 holds 32 cells.
    // pub tape_long: Vec<u32>,
    // /// tl_pos represents the start of the 128 tape in the long tape (covering four u32 cell blocks)
    // tl_pos: usize,
    /// High bound in tape_long, this is the leftmost bit which is a 1.
    high_bound: u32,
    /// Low bound in tape_long, this is the rightmost bit which is a 1.
    low_bound: i32,
    // /// Tape size limit in number of u32 blocks
    // tape_size_limit_u32_blocks: u32,
}

impl Tape128 {
    // /// Counts Ones for self referencing speed-up
    // #[inline(always)]
    // fn count_left(&self, symbol: usize) -> u32 {
    //     // count 1s starting from middle; get upper part
    //     let t = (self.tape_shifted >> 64) as u64;
    //     if symbol == 1 {
    //         t.trailing_ones() + 1
    //     } else {
    //         t.trailing_zeros() + 1
    //     }
    // }

    // /// Counts Ones for self referencing speed-up
    // #[inline(always)]
    // fn count_right(&self, symbol: usize) -> u32 {
    //     // count 1s starting from middle; get lower part
    //     let t = self.tape_shifted as u64;
    //     if symbol == 1 {
    //         t.leading_ones()
    //     } else {
    //         t.leading_zeros()
    //     }
    // }

    // pub fn tape_long_positions(&self) -> TapeLongPositions {
    //     TapeLongPositions {
    //         tl_pos: self.tl_pos,
    //         tl_high_bound: self.high_bound,
    //         tl_low_bound: self.low_bound,
    //     }
    // }

    // pub fn high_bound(&self) -> u32 {
    //     self.high_bound
    // }

    // pub fn low_bound(&self) -> u32 {
    //     self.low_bound as u32
    // }
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

    /// Returns the approximate tape size, which is actually not known exactly. \
    /// The high/low bound may indicate the actual used tape or may have shifted to the first 1 in that direction.
    #[inline(always)]
    fn tape_size(&self) -> u32 {
        self.high_bound - self.low_bound as u32 + 1
    }

    fn tape_shifted(&self) -> u128 {
        self.tape_shifted
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape bounds were reached.
    #[must_use]
    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionSymbol2) -> bool {
        self.set_current_symbol(transition);
        // if transition.is_dir_right() {
        //     self.tape_shifted <<= 1;
        //     self.pos_middle += 1;
        //     self.shift_tape_long_head_dir_right()
        // } else {
        //     self.tape_shifted >>= 1;
        //     self.pos_middle -= 1;
        //     self.shift_tape_long_head_dir_left()
        // }

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
