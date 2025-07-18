use std::ops::Range;

use crate::{
    config::{
        StepTypeSmall, MAX_TAPE_GROWTH_BLOCKS, TAPE_SIZE_INIT_CELLS, TAPE_SIZE_INIT_CELL_BLOCKS,
    },
    html,
    transition_symbol2::TransitionSymbol2,
};

pub const TAPE_SIZE_BIT_U128: usize = 128;
pub const TAPE_SIZE_HALF_128: usize = TAPE_SIZE_BIT_U128 / 2;
pub const TAPE_SIZE_FOURTH_128: usize = TAPE_SIZE_BIT_U128 / 4;
pub const TAPE_SIZE_FOURTH_UPPER_128: usize = TAPE_SIZE_BIT_U128 / 4 + TAPE_SIZE_HALF_128;
pub const MIDDLE_BIT_U128: usize = TAPE_SIZE_BIT_U128 / 2 - 1;
pub const POS_HALF_U128: u128 = 1 << MIDDLE_BIT_U128;
pub const TL_POS_START_128: usize = TAPE_SIZE_INIT_CELLS / 32 / 2 - 2;
// const LOW32_SWITCH_U128: usize = MIDDLE_BIT_U128 - TAPE_SIZE_FOURTH;
pub const LOW32_SWITCH_U128: usize = MIDDLE_BIT_U128 - TAPE_SIZE_FOURTH_128;
pub const HIGH32_SWITCH_U128: usize = MIDDLE_BIT_U128 + TAPE_SIZE_FOURTH_128;
pub const CLEAR_LOW63_00BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_00000000;
pub const CLEAR_LOW63_32BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_FFFFFFFF;
pub const CLEAR_HIGH95_64BITS_U128: u128 = 0xFFFFFFFF_00000000_FFFFFFFF_FFFFFFFF;
pub const FILTER_HIGH_BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_00000000;
pub const FILTER_LOW_BITS_U128: u128 = 0x00000000_00000000_FFFFFFFF_FFFFFFFF;

pub const TAPE_SIZE_BIT_U64: StepTypeSmall = 64;
pub const MIDDLE_BIT_U64: StepTypeSmall = TAPE_SIZE_BIT_U64 / 2 - 1;
pub const POS_HALF_U64: u64 = 1 << MIDDLE_BIT_U64;

// #[cfg(all(debug_assertions, feature = "bb_debug"))]
pub const TAPE_DISPLAY_RANGE_128: std::ops::Range<usize> =
    TL_POS_START_128 - 1..TL_POS_START_128 + 5;

/// The tape_long is a ```Vec<u32>``` which allows to copy top or bottom 32 bit of u128 tape_shifted
/// into the long tape when a bound is reached.
/// TODO The tape has an initial size of e.g. 128 u64 which is 1024 Byte or 8192 tape cells.
/// The size will double every time its limit is reached. E.g it doubles 1x times to get a size of 256 or 16284 cells,
/// which is the size for BB5 Max tape length.
/// Once 131072 u64 is reached (1 MB), it will grow by 1 MB each time.
/// Here the head is moving within the tape, the tape does not shift at all.
// TODO limit access, pub removal
#[derive(Debug)]
pub struct TapeLong {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    /// The tape is 128 bit wide, but since data is shifted to the long tape, it may be 'dirty', meaning it
    /// does not contain the correct cell values. The cell values will be shifted in only if required. \
    /// In case the head goes left for a large number of steps, the bits next to the head will only be loaded
    /// one step before. The tape is clean as long as it never reached the outer limits.
    pub tape_shifted: u128,
    /// Indication where the original pos_middle has moved within tape_shifted. Used to load data from long_tape.
    pub pos_middle: usize,
    /// Vec of u32 blocks, where each u32 holds 32 cells.
    pub tape_long: Vec<u32>,
    /// tl_pos represents the start of the 128 tape in the long tape (covering four u32 cell blocks)
    tl_pos: usize,
    /// High bound in tape_long, this is the rightmost value.
    tl_high_bound: usize,
    /// Low bound in tape_long, this is the leftmost value.
    tl_low_bound: usize,
    /// Tape size limit in number of u32 blocks
    tape_size_limit_u32_blocks: u32,
}

impl TapeLong {
    pub fn new(tape_size_limit_u32_blocks: u32) -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128,
            tape_long: vec![0; TAPE_SIZE_INIT_CELL_BLOCKS],
            tl_pos: TL_POS_START_128,
            tl_low_bound: TL_POS_START_128,
            tl_high_bound: TL_POS_START_128 + 3,
            tape_size_limit_u32_blocks,
        }
    }

    // resets the decider for a different machine
    pub fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128;

        self.tape_long.clear();
        self.tape_long.resize(TAPE_SIZE_INIT_CELL_BLOCKS, 0);
        self.tl_pos = TL_POS_START_128;
        self.tl_low_bound = TL_POS_START_128;
        self.tl_high_bound = TL_POS_START_128 + 3;
    }

    /// Counts Ones for self referencing speed-up
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

    /// Counts Ones for self referencing speed-up
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

    // TODO correct data
    #[inline(always)]
    pub fn count_ones(&self) -> StepTypeSmall {
        println!("WARNING: This count is incorrect");
        // TODO tape shifted needs to be shifted in the middle and tape long loaded
        let mut ones = self.tape_shifted.count_ones();
        if self.is_tape_extended() {
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
    pub fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
    }

    /// Tape shifted is clean (contains the correct cell values) as long the bounds have not been breached.
    #[inline(always)]
    pub fn is_tape_extended(&self) -> bool {
        self.tl_high_bound - self.tl_low_bound > 3
    }

    #[inline(always)]
    pub fn pos_middle(&self) -> usize {
        self.pos_middle
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    pub fn set_current_symbol(&mut self, transition: TransitionSymbol2) {
        if transition.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    /// Shifts the pos in the long tape one to left and checks Vec dimensions. \
    /// Here the vector needs to be expanded at the beginning and the data must be shifted.
    /// # Returns
    /// False if tape could not be expanded. The caller must react on this an end the decider. \
    /// This could be a Result Err, but for performance this is just a bool.
    #[must_use]
    #[inline(always)]
    pub fn shift_pos_to_left_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos == self.tl_low_bound {
            if self.tl_pos == 0 {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 0 -> pos 40
                let mut grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len());
                let old_len = self.tape_long.len();
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by
                    > self.tape_size_limit_u32_blocks as usize
                {
                    grow_by = self.tape_size_limit_u32_blocks as usize + self.tl_low_bound
                        - self.tape_long.len();
                    if grow_by == 0 {
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
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
    pub fn shift_pos_to_right_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos + 4 > self.tl_high_bound {
            self.tl_high_bound += 1;
            if self.tl_high_bound == self.tape_long.len() {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 96 -> pos 96
                let mut grow_by = MAX_TAPE_GROWTH_BLOCKS.min(self.tape_long.len()) as isize;
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by as usize
                    > self.tape_size_limit_u32_blocks as usize
                {
                    grow_by = self.tape_size_limit_u32_blocks as isize + self.tl_low_bound as isize
                        - self.tape_long.len() as isize;
                    if grow_by <= 0 {
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    println!(
                        "  Tape Resize at end: {} -> {}",
                        self.tape_long.len(),
                        self.tape_long.len() + grow_by
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
        // normal shift LEFT -> tape moves right
        if self.pos_middle == LOW32_SWITCH_U128 {
            // save high bytes
            if !self.shift_pos_to_left_checked() {
                return false;
            }

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

        true
    }

    #[must_use]
    #[inline(always)]
    pub fn shift_tape_long_head_dir_right(&mut self) -> bool {
        // normal shift RIGHT -> tape moves left

        if self.pos_middle == HIGH32_SWITCH_U128 {
            // save high bytes
            if !self.shift_pos_to_right_checked() {
                return false;
            }

            // The shift is right, so tape_shifted wanders left -> store high 32 bits.
            // if self.tl_pos >= self.tape_long.len() {
            //     println!(
            //         "\n *** Error shift: TL len {}, tl_pos {}, tl_high_bound {}",
            //         self.tape_long.len(),
            //         self.tl_pos,
            //         self.tl_high_bound
            //     );
            //     return false;
            // }
            self.tape_long[self.tl_pos] = (self.tape_shifted >> TAPE_SIZE_FOURTH_UPPER_128) as u32;

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            println!(
                "  RIGHT SAVE HIGH P{}-{}: tape wanders left -> {:?}",
                self.pos_middle,
                self.tl_pos,
                self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
            );

            self.pos_middle = MIDDLE_BIT_U128;

            // Load low 32 bit
            // if self.tl_pos + 2 >= self.tape_long.len() {
            //     println!(
            //         "Error shift: TL len {}, tl_pos +2 {}",
            //         self.tape_long.len(),
            //         self.tl_pos + 2 // "Step {}: {}, {}",
            //                         // self.step_no, self.tr, self.transition_table
            //     );
            //     // } else {
            // }
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

        true
    }

    #[inline(always)]
    pub fn tape_length_blocks(&self) -> usize {
        self.tl_high_bound - self.tl_low_bound + 1
    }

    pub fn tape_long_positions(&self) -> TapeLongPositions {
        TapeLongPositions {
            tl_pos: self.tl_pos,
            tl_high_bound: self.tl_high_bound,
            tl_low_bound: self.tl_low_bound,
        }
    }

    #[inline(always)]
    pub fn tape_shifted(&self) -> u128 {
        self.tape_shifted
    }

    /// Returns the approximate tape size, which grows by 32 steps
    #[inline(always)]
    pub fn tape_size(&self) -> u32 {
        ((self.tl_high_bound - self.tl_low_bound + 1) * 32) as u32
    }

    pub fn tape_size_limit_u32_blocks(&self) -> u32 {
        self.tape_size_limit_u32_blocks
    }

    pub fn tl_high_bound(&self) -> usize {
        self.tl_high_bound
    }

    pub fn tl_low_bound(&self) -> usize {
        self.tl_low_bound
    }

    pub fn tl_pos(&self) -> usize {
        self.tl_pos
    }

    pub fn set_tl_pos(&mut self, new_pos: usize) {
        // assert!(new_pos >= self.tl_low_bound);
        // // if new_pos + 3 >= self.tl_high_bound {
        // //     println!("Bound error");
        // //     return;
        // // }
        // assert!(new_pos + 3 <= self.tl_high_bound);
        self.tl_pos = new_pos;
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit).
    #[must_use]
    #[inline(always)]
    pub fn update_tape_single_step(&mut self, transition: TransitionSymbol2) -> bool {
        self.set_current_symbol(transition);
        if transition.is_dir_right() {
            self.tape_shifted <<= 1;
            self.pos_middle += 1;
            self.shift_tape_long_head_dir_right()
        } else {
            self.tape_shifted >>= 1;
            self.pos_middle -= 1;
            self.shift_tape_long_head_dir_left()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TapeLongPositions {
    /// tl_pos represents the start of the 128 tape in the long tape (covering four u32 cell blocks)
    pub tl_pos: usize,
    /// High bound in tape_long, this is the rightmost value.
    pub tl_high_bound: usize,
    /// Low bound in tape_long, this is the leftmost value.
    pub tl_low_bound: usize,
}

pub trait U64Ext {
    #[allow(dead_code)] // required for debugging
    fn to_binary_split_string(&self) -> String;
    fn to_binary_split_html_string(&self, tr: &TransitionSymbol2) -> String;
}

impl U64Ext for u64 {
    fn to_binary_split_string(&self) -> String {
        format!(
            "{:024b}_{:08b} {:08b}_{:024b}",
            self >> 40,
            (self >> 32) as u8,
            (self >> 24) as u8,
            (*self as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
        )
    }

    fn to_binary_split_html_string(&self, tr: &TransitionSymbol2) -> String {
        if tr.is_hold() {
            // TO DO In case the last symbol is written (1RZ instead of ---), it is not colored.
            return self.to_binary_split_string();
        }
        if tr.is_dir_left() {
            let n = format!("{:08b}", (*self >> 24) as u8);
            let t = format!(
                "{}<span class=\"{}\">{}</span>{}",
                &n[0..1],
                html::CLASS_CHANGED_POSITION,
                &n[1..2],
                &n[2..8]
            );
            format!(
                "{:024b}_{:08b}&rarr;{t}_{:024b}",
                self >> 40,
                (self >> 32) as u8,
                (*self as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
            )
        } else {
            let n = format!("{:08b}", (*self >> 32) as u8);
            let t = format!(
                "{}<span class=\"{}\">{}</span>",
                &n[0..7],
                html::CLASS_CHANGED_POSITION,
                &n[7..8]
            );
            format!(
                "{:024b}_{t}&larr;{:08b}_{:024b}",
                self >> 40,
                (self >> 24) as u8,
                (*self as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
            )
        }
    }
}

pub trait U128Ext {
    #[allow(dead_code)] // required for debugging
    fn to_binary_split_string_half(&self) -> String;
    fn to_binary_split_string(&self) -> String;
    fn to_binary_split_html_string(&self, tr: &TransitionSymbol2) -> String;
}

impl U128Ext for u128 {
    fn to_binary_split_string_half(&self) -> String {
        let n64 = (self >> 32) as u64;
        format!(
            "{:024b}_{:08b} {:08b}_{:024b}",
            n64 >> 40,
            (n64 >> 32) as u8,
            (n64 >> 24) as u8,
            (n64 as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
        )
    }

    fn to_binary_split_string(&self) -> String {
        format!(
            "{:032b}_{:024b}_{:08b}*{:08b}_{:024b}_{:032b}",
            (*self >> 96) as u32,
            (*self >> 72) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
            (*self >> 64) as u8,
            (*self >> 56) as u8,
            ((*self >> 32) as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
            *self as u32,
        )
    }

    fn to_binary_split_html_string(&self, tr: &TransitionSymbol2) -> String {
        if tr.is_hold() {
            // TO DO In case the last symbol is written (1RZ instead of ---), it is not colored.
            return self.to_binary_split_string();
        }
        if tr.is_dir_left() {
            let n = format!("{:08b}", (*self >> 56) as u8);
            let t = format!(
                "{}<span class=\"{}\">{}</span>{}",
                &n[0..1],
                html::CLASS_CHANGED_POSITION,
                &n[1..2],
                &n[2..8]
            );
            format!(
                "{:032b}_{:024b}_{:08b}&rarr;{t}_{:024b}_{:032b}",
                (*self >> 96) as u32,
                (*self >> 72) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
                (*self >> 64) as u8,
                ((*self >> 32) as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
                *self as u32,
            )
        } else {
            let n = format!("{:08b}", (*self >> 64) as u8);
            let t = format!(
                "{}<span class=\"{}\">{}</span>",
                &n[0..7],
                html::CLASS_CHANGED_POSITION,
                &n[7..8]
            );
            format!(
                "{:032b}_{:024b}_{t}&larr;{:08b}_{:024b}_{:032b}",
                (*self >> 96) as u32,
                (*self >> 72) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
                (*self >> 56) as u8,
                ((*self >> 32) as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
                *self as u32,
            )
        }
    }
}

pub trait VecU32Ext {
    fn to_hex_string_range(&self, range: Range<usize>) -> String;
}

impl VecU32Ext for Vec<u32> {
    fn to_hex_string_range(&self, range: Range<usize>) -> String {
        let mut s = String::new();
        for cell_pack in self[range.start..range.end - 1].iter() {
            s.push_str(format!("{cell_pack:08X}, ").as_str());
        }
        s.push_str(format!("{:08X}", &self[range.end - 1]).as_str());

        s
    }
}
