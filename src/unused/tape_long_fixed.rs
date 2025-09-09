//! This tape works with a short_tape which is fixed, while the head moves. \
//! This works fine, but is 10-20% slower than tape_long_shifted (possibly due to more loads/saves to tape_long). \
//! It may serve its purpose when checking left and right arms of the tape, as the tape_short always shows the correct bits
//! and can easily be inserted into the related fields of tape_long.

use crate::{
    config::{MAX_TAPE_GROWTH_BLOCKS, TAPE_SIZE_INIT_CELL_BLOCKS},
    tape::{
        tape_utils::{TapeLongPositions, U128Ext, POS_HALF_U128, TL_POS_START_128},
        Tape,
    },
    transition_binary::TransitionBinary,
};

#[derive(Debug)]
pub struct TapeLongFixed {
    /// 128-bit tape to move the head fast. This tape is not shifted and always clean. It fits in the long_tape at tl_pos.
    tape_short: u128,
    /// Head in tape_short
    head: u128,
    #[cfg(feature = "enable_html_reports")]
    /// Indication where the start cell has moved. Used to identify the apex.
    pos_head_short: i64,
    // pos_head_min: i64,
    // pos_head_max: i64,
    /// Vec of u32 blocks, where each u32 holds 32 cells.
    tape_long: Vec<u64>,
    /// tl_pos represents the start of the 128 tape in the long tape (covering four u32 cell blocks)
    tl_pos: usize,
    /// High bound in tape_long, this is the rightmost value.
    tl_high_bound: usize,
    /// Low bound in tape_long, this is the leftmost value.
    tl_low_bound: usize,
    /// Tape size limit in number of u32 blocks
    tape_size_limit_u64_blocks: u32,
}

impl TapeLongFixed {
    /// Shifts the pos in the long tape one to left and checks Vec dimensions. \
    /// Here the vector needs to be expanded at the beginning and the data must be shifted.
    /// # Returns
    /// False if tape could not be expanded. The caller must react on this an end the decider. \
    /// This could be a Result Err, but for performance this is just a bool.
    #[must_use]
    #[inline(always)]
    fn shift_pos_to_left_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos == self.tl_low_bound {
            if self.tl_pos == 0 {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 0 -> pos 40
                let mut grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len());
                let old_len = self.tape_long.len();
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by
                    > self.tape_size_limit_u64_blocks as usize
                {
                    grow_by = self.tape_size_limit_u64_blocks as usize + self.tl_low_bound
                        - self.tape_long.len();
                    if grow_by == 0 {
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "debug_tape"))]
                {
                    println!(
                        "  Tape Resize at start, len {} -> {}",
                        self.tape_long.len(),
                        self.tape_long.len() + grow_by
                    );
                }
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

        true
    }

    /// Shifts the pos in the long tape one to right and checks Vec dimensions. \
    /// Here the vector can easily be expanded at the end.
    /// # Returns
    /// False if tape could not be expanded. The caller must react on this an end the decider. \
    /// This could be a Result Err, but for performance this is just a bool.
    #[inline(always)]
    fn shift_pos_to_right_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos + 2 > self.tl_high_bound {
            self.tl_high_bound += 1;
            if self.tl_high_bound == self.tape_long.len() {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 96 -> pos 96
                let mut grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len()) as isize;
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by as usize
                    > self.tape_size_limit_u64_blocks as usize
                {
                    grow_by = self.tape_size_limit_u64_blocks as isize + self.tl_low_bound as isize
                        - self.tape_long.len() as isize;
                    if grow_by <= 0 {
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "debug_tape"))]
                {
                    println!(
                        "  Tape Resize at end: {} -> {}",
                        self.tape_long.len(),
                        self.tape_long.len() + grow_by as usize
                    );
                }
                self.tape_long
                    .resize(self.tape_long.len() + grow_by as usize, 0);
            }
        }
        self.tl_pos += 1;

        true
    }

    #[must_use]
    #[inline(always)]
    pub fn shift_tape_long_head_dir_left(&mut self) -> bool {
        // shift LEFT
        self.head <<= 1;

        if self.head == 0 {
            // head moved out of 64 bit range, move tape
            // save low bytes
            self.tape_long[self.tl_pos + 1] = self.tape_short as u64;
            // println!(
            //     "save low: {:016X} {}",
            //     self.tape_short as u64,
            //     (self.tape_short as u64).to_binary_split_string()
            // );
            self.tape_short >>= 64;
            // shift long tape
            if !self.shift_pos_to_left_checked() {
                return false;
            }
            // load high bytes
            self.tape_short |= (self.tape_long[self.tl_pos] as u128) << 64;

            self.head = POS_HALF_U128 << 1;
            #[cfg(feature = "enable_html_reports")]
            {
                self.pos_head_short = -1;
            }

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            {
                let range = self.tl_low_bound..self.tl_high_bound + 1;
                println!(
                    "  Tape Long Shift Left  TL P{}: tape {:?}",
                    self.tl_pos,
                    crate::tape::tape_utils::VecU64Ext::to_hex_string_range(&self.tape_long, range)
                );
                print!("");
            }
        } else {
            #[cfg(feature = "enable_html_reports")]
            {
                self.pos_head_short -= 1;
            }
            // if self.pos_head_min > self.pos_head_short {
            //     self.pos_head_min -= 1;
            // }
        }

        true
    }

    #[must_use]
    #[inline(always)]
    pub fn shift_tape_long_head_dir_right(&mut self) -> bool {
        // shift RIGHT
        self.head >>= 1;

        if self.head == 0 {
            // head moved out of 64 bit range, move tape
            // head moved out of 64 bit range, move tape
            // save high bytes
            self.tape_long[self.tl_pos] = (self.tape_short >> 64) as u64;
            self.tape_short <<= 64;
            // shift long tape
            if !self.shift_pos_to_right_checked() {
                return false;
            }
            // load low bytes
            self.tape_short |= self.tape_long[self.tl_pos + 1] as u128;

            self.head = POS_HALF_U128;
            #[cfg(feature = "enable_html_reports")]
            {
                self.pos_head_short = 0;
            }

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            {
                let range = self.tl_low_bound..self.tl_high_bound + 1;
                println!(
                    "  Tape Long Shift Right  TL P{}: tape {:?}",
                    self.tl_pos,
                    crate::tape::tape_utils::VecU64Ext::to_hex_string_range(&self.tape_long, range)
                );
                print!("");
            }
        } else {
            #[cfg(feature = "enable_html_reports")]
            {
                self.pos_head_short += 1;
            }
            // if self.pos_head_max < self.pos_head_short {
            //     self.pos_head_max += 1;
            // }
        }

        true
    }
}

impl Tape for TapeLongFixed {
    fn new(config: &crate::config::Config) -> Self {
        Self {
            tape_size_limit_u64_blocks: config.tape_size_limit_u32_blocks().div_ceil(2),
            ..Default::default()
        }
    }

    fn clear(&mut self) {
        self.tape_short = 0;
        self.head = POS_HALF_U128;
        #[cfg(feature = "enable_html_reports")]
        {
            self.pos_head_short = 0;
        }

        self.tape_long.clear();
        self.tape_long.resize(TAPE_SIZE_INIT_CELL_BLOCKS, 0);
        self.tl_pos = TL_POS_START_128;
        self.tl_low_bound = TL_POS_START_128;
        self.tl_high_bound = TL_POS_START_128 + 1;
    }

    fn count_ones(&self) -> u32 {
        // // fill tape long, no requires mut
        // self.tape_long[self.tl_pos] = (self.tape_short >> 64) as u64;
        // self.tape_long[self.tl_pos + 1] = self.tape_short as u64;

        let mut ones = self.tape_short.count_ones();
        for n in self.tape_long[self.tl_low_bound..self.tl_pos].iter() {
            ones += n.count_ones();
        }
        for n in self.tape_long[self.tl_pos + 2..self.tl_high_bound + 1].iter() {
            ones += n.count_ones();
        }
        ones
    }

    fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_short & self.head) != 0) as usize
    }

    fn is_left_empty(&self) -> bool {
        todo!()
    }

    fn is_right_empty(&self) -> bool {
        todo!()
    }

    fn left_64_bit(&self) -> u64 {
        todo!()
    }

    fn right_64_bit(&self) -> u64 {
        todo!()
    }

    #[cfg(feature = "enable_html_reports")]
    fn pos_middle_print(&self) -> i64 {
        self.pos_head_short
    }

    fn set_current_symbol(&mut self, transition: TransitionBinary) {
        if transition.is_symbol_one() {
            self.tape_short |= self.head
        } else {
            self.tape_short &= !self.head
        };
    }

    // fn supports_speed_up(&self) -> bool {
    //     false
    // }

    fn tape_long_positions(&self) -> Option<TapeLongPositions> {
        Some(TapeLongPositions {
            tl_pos: self.tl_pos,
            tl_high_bound: self.tl_high_bound,
            tl_low_bound: self.tl_low_bound,
        })
    }

    #[cfg(feature = "enable_html_reports")]
    fn tape_shifted_clean(&self) -> u128 {
        // TODO tape_shifted from tape_long
        let pos = self.pos_head_short;
        if pos > 0 {
            self.tape_short << pos
        } else {
            self.tape_short >> -pos
        }
    }

    fn tape_size_cells(&self) -> u32 {
        ((self.tl_high_bound - self.tl_low_bound + 1) * 64) as u32
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit).
    #[must_use]
    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionBinary) -> bool {
        // println!(
        //     "{}, H: {} {}..{}",
        //     self.head.to_binary_split_string(),
        //     self.pos_head_short,
        //     self.pos_head_min,
        //     self.pos_head_max
        // );
        self.set_current_symbol(transition);
        if transition.is_dir_right() {
            self.shift_tape_long_head_dir_right()
        } else {
            self.shift_tape_long_head_dir_left()
        }
    }

    #[inline(always)]
    fn write_last_symbol(&mut self, transition: crate::transition_binary::TransitionBinary) {
        if !transition.is_symbol_undefined() {
            self.set_current_symbol(transition);
        }
    }
}

impl Default for TapeLongFixed {
    fn default() -> Self {
        Self {
            tape_short: 0,
            head: POS_HALF_U128,
            #[cfg(feature = "enable_html_reports")]
            pos_head_short: 0,
            // pos_head_min: 0,
            // pos_head_max: 0,
            tape_long: vec![0; TAPE_SIZE_INIT_CELL_BLOCKS],
            tl_pos: TL_POS_START_128,
            tl_low_bound: TL_POS_START_128,
            tl_high_bound: TL_POS_START_128 + 1,
            tape_size_limit_u64_blocks: u32::MAX,
        }
    }
}

impl std::fmt::Display for TapeLongFixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "enable_html_reports")]
        {
            write!(
                f,
                "{} P{:3}, B {}..{}",
                self.tape_short.to_binary_split_string(),
                self.pos_head_short,
                self.tl_low_bound,
                self.tl_high_bound,
            )
        }

        #[cfg(not(feature = "enable_html_reports"))]
        write!(
            f,
            "{}, B {}..{}",
            self.tape_short.to_binary_split_string(),
            self.tl_low_bound,
            self.tl_high_bound,
        )
    }
}
