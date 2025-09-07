//! This tape uses a different approach to store the data of the tape. \
//! Instead of having a cell for each symbol, repeated patterns are stored.
//!
//! Example: Instead of storing 01111111_11111111111111110010011, the data can
//! be stored in a vec of patterns: \
//! 1: pattern: 0 size 1 repeat 1 \
//! 2: pattern: 1 size 1 repeat 23 \
//! 3: pattern: 001 size 3 repeat 2, \
//! 4: pattern: 1 size 1 repeat 1 \
//!
//! While the logic to maintain the tape is quite complex, the memory benefits are great. \
//! The runtime is about 3 times longer than with a normal tape, so it should only be used when necessary.
//!
//! This is still in the alpha development phase:
//! * No self-ref acceleration implemented yet.
//! * No macros stored. Tape should recognize macros. 
//! * It seems inefficient to run this tape for the first steps,
//! instead it could be after x steps.
//! 
//! # Example
//! Taken from BB5 Max with its 47,176,870 steps.
//! Step: 47000000, Cells: 12250 x 1 then 110010010010010010011 \
//! The tape has a length of 12,295 used cells. This translates roughly into 1,5 KB of data
//! and as such is not an issue for a computer. \
//! However, as a pattern this looks like this: \
//! 1: pattern: 1 size 1 repeat 12252 \
//! 2: pattern: 001 size 3 repeat 6, \
//! 3: pattern: 1 size 1 repeat 1 \
//! Each pattern description uses merely 32 byte and as such only 96 bytes are used plus the vector overhead.
//!
//! The real benefit occurs when multiple steps can be done in a group. \
//! Here A0: 1RB and D1: 1LD are repeating themselves. \
//! Having a pattern now allows to skip these 12252 steps and just move \
//! the head to the new position and increase the step count. \
//! This reduces the number of steps to be evaluated by 99,8%!
//!
//! The pattern '001' can used to group the steps E1: 0LA, A1: 1LC and C1: 0LE
//! (Planned functionality)
//!
//! # Example
//! For Bb4Max 1RB1LB_1LA0LC_---1LD_1RD0RA the first steps are explained.
//!
//! Initially the pattern is just one 0. Pattern 0: 0..0: 0 size: 1 repeat: 1
//!
//!  1. A0 1RB: 00000001←00000000
//!  - Set Symbol: Change pattern to 1 repeat 1
//!  - Move right: Pattern is at end, extend pattern: 10 repeat 1
//!  - Pattern 0: 0..2: 10 size: 2 repeat: 1

//!  2. B0 ILA: 00000000→11000000
//!  - Set Symbol: Change pattern to 11 repeat 1
//!  - Merge pattern to 1 repeat 2
//!  - Move left: stays in pattern
//!  - Pattern 0: 0..2: 1 size: 1 repeat: 2
//!
//!  3. A1 1LB: 00000000→01100000
//!  - Set Symbol: Symbol does not change, do nothing
//!  - Move left: Tape too small, extend left with new pattern 0
//!  - Pattern 0: -1..0: 0 size: 1 repeat: 1
//!  - Pattern 1: 0..2: 1 size: 1 repeat: 2
//!
//!  4. B0 1LA: 00000000→01110000
//!  - Set Symbol: Symbol of pattern 0 can be changed
//!  - Move left: Tape too small, extend left pattern 0
//!  - Pattern 0: -2..0: 01 size: 2 repeat: 1
//!  - Pattern 1: 0..2: 1 size: 1 repeat: 2
//!
//!  5. A0 1RB: 00000001←11100000
//!  - Set Symbol: Symbol of pattern 0 can be changed -> 11
//!  - Merge pattern to 1 repeat 2
//!  - Move left: stays in pattern
//!  - Pattern 0: -2..0: 1 size: 1 repeat: 2
//!  - Pattern 1: 0..2: 1 size: 1 repeat: 2
//!  - At this point this is not merged as it only happens at tape end points.
//!
//!  6. B1 0LC: 00000000→10110000
//!  - Set Symbol: pattern 0 can be changed from 1 repeat 1 to 10
//!  - Move left: stays in pattern
//!  - Pattern 0: -2..0: 10 size: 2 repeat: 1
//!  - Pattern 1: 0..2: 1 size: 1 repeat: 2
//!
//!  7. C1 1LD: 00000000→01011000
//!  - Set Symbol: no change
//!  - Move left: Tape too small, extend left with new pattern 0
//!  - !Note only pattern size 1 gets extended. Pattern size 3 not yet defined.
//!  - Pattern 0: -3..-2: 0 size: 1 repeat: 1
//!  - Pattern 1: -2..0: 10 size: 2 repeat: 1
//!  - Pattern 2: 0..2: 1 size: 1 repeat: 2
//!
//!  8. D0 1RD: 00000001←10110000
//!  - Set Symbol: pattern 0 can be changed
//!  - Move right: go to next pattern
//!  - Pattern 0: -3..-2: 1 size: 1 repeat: 1
//!  - Pattern 1: -2..0: 10 size: 2 repeat: 1
//!  - Pattern 2: 0..2: 1 size: 1 repeat: 2
//!
//!  9. D1 0RA: 00000010←01100000
//!  - Set Symbol: pattern 1 can be changed and merged
//!  - Move right: stays in pattern
//!  - Pattern 0: -3..-2: 1 size: 1 repeat: 1
//!  - Pattern 1: -2..0: 0 size: 1 repeat: 2
//!  - Pattern 2: 0..2: 1 size: 1 repeat: 2
//!
//! 10. A0 1RB: 00000101←11000000
//!  - Set Symbol: pattern 1 can be changed
//!  - Move right: go to next pattern
//!  - Pattern 0: -3..-2: 1 size: 1 repeat: 1
//!  - Pattern 1: -2..0: 01 size: 2 repeat: 1
//!  - Pattern 2: 0..2: 1 size: 1 repeat: 2
//!
//! 11. B1 0LC: 00000010→10100000
//!  - Set Symbol: pattern 2 can be changed
//!  - Move left: go to previous pattern
//!  - Pattern 0: -3..-2: 1 size: 1 repeat: 1
//!  - Pattern 1: -2..0: 01 size: 2 repeat: 1
//!  - Pattern 2: 0..2: 01 size: 2 repeat: 1
//!
//! 12 C1 1LD: 00000001→01010000
//!

#![allow(unreachable_code)]
use std::fmt::Display;

use crate::{
    config::Config,
    tape::{
        tape_utils::{TapeLongPositions, U128Ext},
        Tape,
    },
    transition_binary::TransitionBinary,
};

/// Stores one repeated element of the tape. \
/// Example: 11101010101 showing the used tape. \
/// 1: pattern: 1 size 1 repeat 2 \
/// 2: pattern: 10 size 2 repeat 4 \
/// 3: pattern: 1 size 1 repeat 1, \
/// 3 is not necessary if 2 was repeat 5 as the extra 0 does not matter.
/// TODO remove assert once stable
#[derive(Debug, Clone, Copy)]
pub struct Pattern {
    /// The pattern itself, e.g. 1, 10, 100, 01 or else. Size is necessary if pattern starts with 0.
    pub pattern: u32,
    /// Size of the pattern, e.g. 01 = 2.
    pub size: u32,
    /// Number of times the pattern repeats. \
    /// Could be calculated as (pos_end - pos_start) / size.
    pub repeat: u32,
    /// Start of the pattern on the tape
    pub pos_start: i64,
    /// End of the pattern on the tape (exclusive)
    pub pos_end: i64,
}

impl Pattern {
    /// Extends the current tape by one cell to the left (new cells are always 0). \
    /// Returns a new pattern if current one cannot be extended.
    fn extend_left(&mut self) -> Option<Self> {
        match self.repeat {
            1 => match self.size {
                1 => {
                    if self.pattern == 0 {
                        self.repeat = 2;
                    } else {
                        // self.pattern = 0b01;
                        self.size = 2;
                    }
                    self.pos_start -= 1;
                    None
                }
                2 => {
                    // need new one
                    let p = Self {
                        pattern: 0,
                        size: 1,
                        repeat: 1,
                        pos_start: self.pos_start - 1,
                        pos_end: self.pos_start,
                    };
                    Some(p)
                }
                _ => todo!(),
            },
            _ => {
                if self.size == 1 {
                    if self.pattern == 0 {
                        self.repeat += 1;
                        self.pos_start -= 1;
                        None
                    } else {
                        // cannot change current pattern, create new one in front
                        let p = Self {
                            pattern: 0,
                            size: 1,
                            repeat: 1,
                            pos_start: self.pos_start - 1,
                            pos_end: self.pos_start,
                        };
                        Some(p)
                    }
                } else {
                    todo!()
                }
            }
        }
    }

    /// Extends the current tape by one cell to the right (new cells are always 0). \
    /// Returns None if the current pattern was extended or a new pattern which needs to be added to the end.
    pub fn extend_right(&mut self) -> Option<Self> {
        match self.repeat {
            1 => match self.size {
                1 => {
                    if self.pattern == 0 {
                        self.repeat = 2;
                    } else {
                        self.pattern = 0b10;
                        self.size = 2;
                    }
                    self.pos_end += 1;
                    None
                }
                2 => {
                    // need new one
                    let p = Self {
                        pattern: 0,
                        size: 1,
                        repeat: 1,
                        pos_start: self.pos_end,
                        pos_end: self.pos_end + 1,
                    };
                    Some(p)
                }
                _ => todo!(),
            },
            2 => {
                if self.pattern == 0 {
                    // If pattern == 0 then size must be 1, just extend
                    self.repeat += 1;
                    self.pos_end += 1;
                    None
                } else {
                    // need new one
                    let p = Self {
                        pattern: 0,
                        size: 1,
                        repeat: 1,
                        pos_start: self.pos_end,
                        pos_end: self.pos_end + 1,
                    };
                    Some(p)
                }
            }
            _ => {
                // need new one
                let p = Self {
                    pattern: 0,
                    size: 1,
                    repeat: 1,
                    pos_start: self.pos_end,
                    pos_end: self.pos_end + 1,
                };
                Some(p)
            }
        }
    }

    /// Returns the symbol at the defined position. \
    /// This does not test if the pos_head falls into the range of the tape.
    pub fn get_symbol_at(&self, pos_head: i64) -> usize {
        // TODO replace with if
        match self.size {
            1 => self.pattern as usize,
            _ => {
                let shift = (pos_head - self.pos_start) as u32 % self.size;
                let filter = 1 << (self.size - shift - 1);
                (self.pattern & filter != 0) as usize
            }
        }
    }

    /// Returns the symbol at the defined position or None if the head position is outside of the tape dimensions. \
    pub fn get_symbol_at_checked(&self, pos_head: i64) -> Option<usize> {
        if pos_head < self.pos_end && pos_head >= self.pos_start {
            Some(self.get_symbol_at(pos_head))
        } else {
            None
        }
    }

    pub fn count_ones(&self) -> u32 {
        self.pattern.count_ones() * self.repeat as u32
    }

    pub fn tape_size_cells(&self) -> usize {
        (self.pos_end - self.pos_start) as usize
    }

    /// Converts the patterns into a tape with bits where the head moves.
    pub fn as_tape_fixed(&self) -> u128 {
        let mut tape: u128 = 0;
        let repeat = self.repeat.min(128 / self.size);
        let mut last_size = self.size;
        let mut shift = 128;
        for _ in 0..repeat {
            shift -= last_size;
            tape |= (self.pattern as u128) << shift;
            last_size = self.size;
        }

        tape
    }

    /// Returns the pattern as u128, beginning on the top. \
    /// start_at: position between pos_start and pos_end
    pub fn as_tape_fixed_from(&self, start_at: i64) -> u128 {
        let mut tape: u128 = 0;
        let len = (self.pos_end - start_at) as u32;
        let tape_size_remaining = if len % self.size != 0 {
            // here now the last relevant bits of pattern are required
            let bits = len % self.size;
            let shift = 32 - bits;
            tape = ((self.pattern << shift) >> (shift)) as u128;
            128 - bits
        } else {
            128
        };
        let repeat = self.repeat.min(len / self.size).min(128);
        let mut last_size = self.size;
        let mut shift = tape_size_remaining;
        for _i in 0..repeat {
            // if last_size > shift {
            //     println!()
            // }
            shift -= last_size;
            tape |= (self.pattern as u128) << shift;
            last_size = self.size;
        }

        tape
    }

    /// Returns a string of all cells
    pub fn to_full_string(&self) -> String {
        let digits = self.size as usize;
        let p = format!("{:0digits$b}", self.pattern);

        p.repeat(self.repeat as usize)
    }

    //     /// Sets the symbol at the defined position. \
    //     /// This does not test if the pos_head falls into the range.
    //     /// Return the symbol at the new position, if available.
    //     fn set_symbol_at(&mut self, pos_head: i64, symbol: usize, move_dir: MoveType) -> Option<usize> {
    //         match self.repeat {
    //             // no repeat, data can be changed
    //             1 => match self.size {
    //                 1 => {
    //                     // len 1, just change it
    //                     self.pattern = symbol as u32;
    //                     // len is 1, any movement is outside
    //                     None
    //                 }
    //                 2 => {
    //                     // calc shift from end to allow easy shift left
    //                     let mut shift = self.pos_end - 1 - pos_head;
    //                     let filter = 1 << shift;
    //                     // clear symbol
    //                     self.pattern &= !filter;
    //                     // set symbol
    //                     if symbol == 1 {
    //                         self.pattern |= filter;
    //                     }
    //                     // keep pattern because of merge
    //                     let pattern = self.pattern;
    //                     // merge
    //                     match self.pattern {
    //                         0 => {
    //                             self.repeat = 2;
    //                             self.pattern = 0;
    //                             self.size = 1;
    //                         }
    //                         3 => {
    //                             self.repeat = 2;
    //                             self.pattern = 1;
    //                             self.size = 1;
    //                         }
    //                         _ => {}
    //                     }
    //
    //                     // current symbol after move
    //                     if move_dir == DIR_LEFT {
    //                         if pos_head > self.pos_start {
    //                             shift += 1;
    //                         } else {
    //                             return None;
    //                         }
    //                     } else if pos_head + 1 < self.pos_end {
    //                         shift -= 1;
    //                     } else {
    //                         return None;
    //                     }
    //                     let filter = 1 << shift;
    //                     let sym = (pattern & filter != 0) as usize;
    //                     // return current symbol
    //                     Some(sym)
    //                 }
    //                 _ => todo!(),
    //             },
    //             2 => match self.size {
    //                 // repeat, but size is one, can be changed without changing total size
    //                 1 => {
    //                     if self.pattern as usize != symbol {
    //                         self.repeat = 1;
    //                         self.size = 2;
    //                         // calc shift from end to allow easy shift left
    //                         let shift = self.pos_end - 1 - pos_head;
    //                         let filter = 1 << shift;
    //                         if symbol == 1 {
    //                             self.pattern = filter;
    //                         } else {
    //                             self.pattern = !filter & 0b11;
    //                         }
    //                     }
    //                     // The single symbol is not like the symbol, and needed to be changed.
    //                     // Therefore the other symbol must be the opposite of the given symbol
    //                     // if it is in the range.
    //                     let new_pos = pos_head + move_dir as i64;
    //                     if new_pos >= self.pos_start && new_pos < self.pos_end {
    //                         Some(!symbol & 0b1)
    //                     } else {
    //                         None
    //                     }
    //                 }
    //                 _ => todo!(),
    //             },
    //             _ => todo!(),
    //         }
    //     }
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            pattern: 0,
            size: 1,
            repeat: 1,
            pos_start: 0,
            pos_end: 1,
        }
    }
}

impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits = self.size as usize;
        write!(
            f,
            "{}..{}: {:0digits$b} size: {} repeat: {}",
            self.pos_start, self.pos_end, self.pattern, self.size, self.repeat,
        )
    }
}

#[derive(Debug, Default)]
pub struct TapeCompact {
    patterns: Vec<Pattern>,
    /// Indication where the tape starts (first 1) in relation to start cell.
    /// This is negative if moving left from start cell.
    tape_start: i64,
    tape_end: i64,
    /// Indication where the head has moved in relation to start cell.
    pos_head: i64,
    /// Current pattern index of patterns at head_pos
    pattern_id: usize,
    /// Current symbol at head position. Avoids update if unchanged.
    curr_symbol: usize,
    tape_size_limit_cells: u32,
}

impl TapeCompact {
    /// merges all patterns and updates pattern_id
    fn check_merge(&mut self) {
        // merge consecutive identical patterns
        let mut i = self.patterns.len() - 1;
        while i > 0 {
            if self.patterns[i - 1].pattern == self.patterns[i].pattern
                && self.patterns[i - 1].size == self.patterns[i].size
            {
                self.patterns[i - 1].repeat += self.patterns[i].repeat;
                self.patterns[i - 1].pos_end = self.patterns[i].pos_end;
                if self.pattern_id >= i {
                    self.pattern_id -= 1;
                }
                self.patterns.remove(i);
            }
            i -= 1;
        }

        return;
        // TODO after this check more
        // combine 2 consecutive size-1 patterns if both are repeated
        if self.patterns.len() > 3 {
            let mut i = self.patterns.len() - 1;
            while i > 2 {
                if self.patterns[i - 2].size == 1
                    && self.patterns[i - 2].repeat < 10
                    && self.patterns[i].size == 1
                    && self.patterns[i - 2].pattern == self.patterns[i].pattern
                    && self.patterns[i - 2].repeat == self.patterns[i].repeat
                {
                    if self.patterns[i - 3].size == 1
                        && self.patterns[i - 3].repeat == 1
                        && self.patterns[i - 1].size == 1
                        && self.patterns[i - 3].pattern == self.patterns[i - 1].pattern
                        && self.patterns[i - 1].repeat == 1
                    {
                        let size_2 = self.patterns[i - 2].repeat as u32;
                        let filter = (1u32 << (size_2 + 1)) - 1;
                        dbg!(i, filter);
                        self.patterns[i - 3].size += size_2;
                        self.patterns[i - 3].pos_end = self.patterns[i].pos_end;
                        self.patterns[i - 3].pattern = (self.patterns[i - 3].pattern << size_2)
                            | self.patterns[i - 2].pattern & filter;
                        self.patterns[i - 3].repeat *= 2;

                        self.patterns.drain(i - 2..i + 1);
                        self.set_pattern_id();
                    }
                }
                i -= 1;
            }
        }
    }

    fn set_pattern_id(&mut self) {
        let p = self
            .patterns
            .iter()
            .enumerate()
            .find(|(_, p)| p.pos_end > self.pos_head)
            .unwrap();
        self.pattern_id = p.0;
    }

    // pub fn tape_size_cells(&self) -> u64 {
    //     (self.tape_end - self.tape_start) as u64
    // }

    fn set_current_symbol_and_move(&mut self, transition: TransitionBinary) -> bool {
        // check merge
        if transition.is_dir_right() {
            if self.pos_head == self.tape_start {
                // head was at tape_start, merge tape at this point
                self.check_merge();
                #[cfg(all(debug_assertions, feature = "debug_tape"))]
                println!("  check merge patterns");
            }
        } else if self.pos_head + 1 == self.tape_end {
            // head was at tape_start, merge tape at this point
            self.check_merge();
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("  check merge patterns");
        }

        let cur_pat = &mut self.patterns[self.pattern_id];
        let new_symbol = transition.symbol_usize();
        let next_symbol = match cur_pat.repeat {
            // no repeat, data can be changed
            1 => match cur_pat.size {
                1 => {
                    // len 1, just change it
                    cur_pat.pattern = new_symbol as u32;
                    // len is 1, any movement is outside
                    None
                }
                2 => {
                    // calc shift from end to allow easy shift left
                    let mut shift = cur_pat.pos_end - 1 - self.pos_head;
                    let filter = 1 << shift;
                    // clear symbol
                    cur_pat.pattern &= !filter;
                    // set symbol
                    if new_symbol == 1 {
                        cur_pat.pattern |= filter;
                    }
                    // store pattern because of merge
                    let pattern = cur_pat.pattern;
                    // merge, e.g. 11 repeat 1 -> 1 repeat 2
                    match cur_pat.pattern {
                        0 => {
                            cur_pat.repeat = 2;
                            cur_pat.pattern = 0;
                            cur_pat.size = 1;
                        }
                        3 => {
                            cur_pat.repeat = 2;
                            cur_pat.pattern = 1;
                            cur_pat.size = 1;
                        }
                        _ => {}
                    }

                    // get current symbol after move
                    if transition.is_dir_left() {
                        if self.pos_head > cur_pat.pos_start {
                            shift += 1;
                            let filter = 1 << shift;
                            let sym = (pattern & filter != 0) as usize;
                            // return current symbol
                            Some(sym)
                        } else {
                            None
                        }
                    } else if self.pos_head + 1 < cur_pat.pos_end {
                        shift -= 1;
                        let filter = 1 << shift;
                        let sym = (pattern & filter != 0) as usize;
                        // return current symbol
                        Some(sym)
                    } else {
                        None
                    }
                }
                _ => todo!(),
            },
            2 => match cur_pat.size {
                // Repeat is 2, but size is 1, can be changed without changing total size
                // as then we have e.g. 10 and repeat 1.
                1 => {
                    // should not be called if unchanged
                    assert!(cur_pat.pattern as usize != new_symbol);
                    cur_pat.repeat = 1;
                    cur_pat.size = 2;
                    // calc shift from end to allow easy shift left
                    let shift = cur_pat.pos_end - 1 - self.pos_head;
                    let filter = 1 << shift;
                    if new_symbol == 1 {
                        cur_pat.pattern = filter;
                    } else {
                        cur_pat.pattern = !filter & 0b11;
                    }
                    // The single symbol is not like the symbol, and needed to be changed.
                    // Therefore the other symbol must be the opposite of the given symbol
                    // if it is in the range.
                    let new_pos = self.pos_head + transition.direction() as i64;
                    if new_pos >= cur_pat.pos_start && new_pos < cur_pat.pos_end {
                        Some(!new_symbol & 0b1)
                    } else {
                        None
                    }
                }
                // TODO any higher would require a split. Possibly better to create a full new set.
                _ => {
                    self.split_current(new_symbol);
                    None
                }
            },
            _ => {
                // This requires a pattern split and only works if the pattern is only one symbol.
                // No next_symbol as it always falls out of cur_pat.
                // should not be called if unchanged
                assert!(cur_pat.pattern as usize != new_symbol);
                self.split_current(new_symbol);
                None
            }
        };

        // get current symbol
        self.pos_head += transition.direction() as i64;
        self.curr_symbol = match next_symbol {
            Some(symbol) => symbol,
            None => {
                // symbol not in current_pattern, move head
                if transition.is_dir_right() {
                    if self.pos_head == self.tape_end {
                        // extend tape
                        if let Some(new_pattern) = self.patterns[self.pattern_id].extend_right() {
                            self.patterns.push(new_pattern);
                            self.pattern_id += 1;
                        }
                        self.tape_end += 1;
                        // check tape size limit
                        if self.tape_size_cells() >= self.tape_size_limit_cells {
                            self.curr_symbol = 0;
                            return false;
                        }

                        0
                    } else {
                        self.pattern_id += 1;
                        // if self.pos_head - 1 == self.tape_start {
                        //     // head was at tape_start, merge tape at this point
                        //     self.check_merge();
                        //     #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        //     println!("  check merge");
                        // }

                        self.patterns[self.pattern_id].get_symbol_at(self.pos_head)
                    }
                } else {
                    // move head left
                    if self.pos_head < self.tape_start {
                        // extend tape
                        if let Some(new_pattern) = self.patterns[self.pattern_id].extend_left() {
                            self.patterns.insert(0, new_pattern);
                            // pattern_id remains as -1 is the new 0
                            // self.pattern_id += 1;
                        }
                        // remove, seems old code
                        // // since it could not be merged it is just a simple 0
                        // let p = Pattern {
                        //     pattern: 0,
                        //     size: 1,
                        //     repeat: 1,
                        //     pos_start: self.tape_start - 1,
                        //     pos_end: self.tape_start,
                        // };
                        self.tape_start -= 1;
                        // check tape size limit
                        if self.tape_size_cells() >= self.tape_size_limit_cells {
                            self.curr_symbol = 0;
                            return false;
                        }

                        0
                    } else {
                        self.pattern_id -= 1;
                        // if self.pos_head + 2 == self.tape_end {
                        //     // head was at tape end, merge tape at this point
                        //     self.check_merge();
                        //     #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        //     println!("  check merge");
                        // }

                        self.patterns[self.pattern_id].get_symbol_at(self.pos_head)
                    }
                }
            }
        };

        true
    }

    // TODO move to Pattern again and return vec of patterns and pattern_id shift
    fn split_current(&mut self, new_symbol: usize) {
        let cur_pat = &mut self.patterns[self.pattern_id];
        let mut p_new = *cur_pat;
        let insert_id;
        match cur_pat.size {
            1 => {
                if self.pos_head == cur_pat.pos_start {
                    // head at start
                    cur_pat.repeat = 1;
                    cur_pat.pos_end = cur_pat.pos_start + 1;
                    cur_pat.pattern = new_symbol as u32;

                    p_new.repeat -= 1;
                    p_new.pos_start += 1;
                    insert_id = self.pattern_id + 1;
                } else if self.pos_head == cur_pat.pos_end - 1 {
                    // head at end
                    cur_pat.repeat -= 1;
                    cur_pat.pos_end = cur_pat.pos_end - 1;
                    self.pattern_id += 1;
                    insert_id = self.pattern_id;

                    p_new.repeat = 1;
                    p_new.pos_start = p_new.pos_end - 1;
                    p_new.pattern = new_symbol as u32;
                } else {
                    // head in the middle
                    let mut p = *cur_pat;
                    p.pos_end = self.pos_head;
                    p.repeat = (p.pos_end - p.pos_start) as u32;

                    cur_pat.repeat = 1;
                    cur_pat.pos_start = self.pos_head;
                    cur_pat.pos_end = self.pos_head + 1;
                    cur_pat.pattern = new_symbol as u32;

                    p_new.pos_start = cur_pat.pos_end;
                    p_new.repeat = (p_new.pos_end - p_new.pos_start) as u32;

                    self.patterns.insert(self.pattern_id, p);
                    self.pattern_id += 1;
                    insert_id = self.pattern_id + 1;
                }
                // add new pattern
                if insert_id + 1 > self.patterns.len() {
                    self.patterns.push(p_new);
                } else {
                    self.patterns.insert(insert_id, p_new);
                }
            }
            _ => {
                // size > 1
                if self.pos_head == cur_pat.pos_start {
                    // head at start
                    let p = Pattern {
                        // remove obsolete part
                        pattern: (cur_pat.pattern << (32 - cur_pat.size)) >> (32 - cur_pat.size),
                        size: cur_pat.size - 1,
                        repeat: 1,
                        pos_start: cur_pat.pos_start + 1,
                        pos_end: cur_pat.pos_start + cur_pat.size as i64,
                    };

                    cur_pat.repeat = 1;
                    cur_pat.size = 1;
                    cur_pat.pos_end = cur_pat.pos_start + 1;
                    cur_pat.pattern = new_symbol as u32;

                    p_new.repeat -= 1;
                    p_new.pos_start = p.pos_end;
                    insert_id = self.pattern_id + 2;

                    if self.patterns.len() > self.pattern_id + 1 {
                        self.patterns.insert(self.pattern_id + 1, p);
                    } else {
                        self.patterns.push(p);
                    }
                } else if self.pos_head == cur_pat.pos_end - 1 {
                    // head at end
                    todo!();
                    cur_pat.repeat -= 1;
                    cur_pat.pos_end = cur_pat.pos_end - 1;
                    self.pattern_id += 1;
                    insert_id = self.pattern_id;

                    p_new.repeat = 1;
                    p_new.pos_start = p_new.pos_end - 1;
                    p_new.pattern = new_symbol as u32;
                } else {
                    // head in the middle
                    // split repeated, so only one pattern remains, then split that pattern -> up to 5 Pattern
                    todo!();
                    let mut p = *cur_pat;
                    p.pos_end = self.pos_head;
                    p.repeat = (p.pos_end - p.pos_start) as u32;

                    cur_pat.repeat = 1;
                    cur_pat.pos_start = self.pos_head;
                    cur_pat.pos_end = self.pos_head + 1;
                    cur_pat.pattern = new_symbol as u32;

                    p_new.pos_start = cur_pat.pos_end;
                    p_new.repeat = (p_new.pos_end - p_new.pos_start) as u32;

                    self.patterns.insert(self.pattern_id, p);
                    self.pattern_id += 1;
                    insert_id = self.pattern_id + 1;
                }
                // add new pattern
                if insert_id + 1 > self.patterns.len() {
                    self.patterns.push(p_new);
                } else {
                    self.patterns.insert(insert_id, p_new);
                }
            }
        }
    }

    // Returns the 128 bit relevant part with head moving (head position not indicated). For display purposes.
    pub fn as_tape_fixed(&self) -> u128 {
        // TODO longer tapes, but how, if it is fixed, then it is actually shifted out. Could jump 64.
        let shift = (self.pos_head + 1) / 64;
        let start_show = shift * 64 - 64;
        let end_show = start_show + 127;
        let mut tape: u128 = 0;
        let mut start = 0;
        for p in self
            .patterns
            .iter()
            .filter(|p| p.pos_end > start_show && p.pos_start < end_show)
        {
            let pos_start;
            let t = if p.pos_start < start_show {
                pos_start = (start_show - p.pos_start) as usize;
                p.as_tape_fixed_from(start_show)
            } else {
                pos_start = 0;
                p.as_tape_fixed()
            };
            // println!("  t     = {}", t.to_binary_split_string());
            tape |= t >> start;
            // println!("  tape  = {}", tape.to_binary_split_string());
            start += p.tape_size_cells() - pos_start;
            // if start >= 128 {
            //     println!("shift start: {start}");
            // }
        }
        let f = self
            .patterns
            .iter()
            .find(|p| p.pos_end > start_show && p.pos_start < end_show);
        let tape_start = match f {
            Some(p) => p.pos_start,
            None => {
                panic!("logic error");
            }
        };

        let shift_tape = 64 + (tape_start % 64);
        // println!("shift tape: {shift_tape}");
        tape >> shift_tape
    }

    /// Returns the 128 bit relevant part with tape moving (head position fixed at bit 63). For display purposes.
    pub fn as_tape_shifted(&self) -> u128 {
        // TODO longer tapes
        let tape = self.as_tape_fixed();
        if self.pos_head >= 0 {
            tape << self.pos_head % 64
        } else {
            tape >> -self.pos_head % 64
        }
    }

    /// Returns a string of all cells
    pub fn to_full_string(&self) -> String {
        let mut s = String::new();
        for p in self.patterns.iter() {
            s.push_str(&p.to_full_string());
        }

        s
    }
}

impl Tape for TapeCompact {
    fn new(config: &Config) -> Self {
        Self {
            patterns: vec![Pattern::default()],
            tape_end: 1,
            tape_size_limit_cells: config.tape_size_limit_cells(),
            ..Default::default()
        }
    }

    fn clear(&mut self) {
        self.patterns.clear();
        self.patterns.push(Pattern::default());
        self.tape_start = 0;
        self.tape_end = 1;
        self.pattern_id = 0;
        self.pos_head = 0;
        self.curr_symbol = 0;
    }

    fn count_ones(&self) -> u32 {
        self.patterns.iter().map(|p| p.count_ones()).sum()
    }

    fn get_current_symbol(&self) -> usize {
        // let p = self
        //     .patterns
        //     .iter()
        //     // && p.pos_start <= self.pos_head this test is left out as the patterns are in order
        //     .find(|p| p.pos_end >= self.pos_head)
        //     .unwrap();

        // self.patterns[self.pattern_id].get_symbol_at(self.pos_head)
        self.curr_symbol as usize
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

    fn set_current_symbol(&mut self, _transition: TransitionBinary) {
        panic!("Do not use");
    }

    fn tape_long_positions(&self) -> Option<TapeLongPositions> {
        todo!()
    }

    fn tape_size_cells(&self) -> u32 {
        (self.tape_end - self.tape_start) as u32
    }

    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionBinary) -> bool {
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!(" move for {transition}, curr symbol {}", self.curr_symbol);
        // for (i, p) in self.patterns.iter().enumerate() {
        //     println!(" Pattern {i:>2}: {p}");
        // }

        if self.curr_symbol == transition.symbol_usize() {
            // symbol has not changed -> tape will not change, just move the head
            if transition.is_dir_right() {
                self.pos_head += 1;
                if self.pos_head == self.tape_end {
                    if let Some(new_pattern) = self.patterns[self.pattern_id].extend_right() {
                        self.patterns.push(new_pattern);
                        self.pattern_id += 1;
                        if self.tape_size_cells() > self.tape_size_limit_cells {
                            return false;
                        }
                    }
                    self.curr_symbol = 0;
                    self.tape_end += 1;
                } else {
                    // self.pos_head += 1;
                    // get current symbol
                    self.curr_symbol =
                        match self.patterns[self.pattern_id].get_symbol_at_checked(self.pos_head) {
                            Some(s) => s,
                            None => {
                                self.pattern_id += 1;
                                self.patterns[self.pattern_id].get_symbol_at(self.pos_head)
                            }
                        };
                }
            } else {
                if self.pos_head == self.tape_start {
                    if let Some(new_pattern) = self.patterns[0].extend_left() {
                        self.patterns.insert(0, new_pattern);
                        // pattern id remains same in this case
                        if self.tape_size_cells() > self.tape_size_limit_cells {
                            return false;
                        }
                    }
                    self.curr_symbol = 0;
                    self.pos_head -= 1;
                    self.tape_start -= 1;
                } else {
                    self.pos_head -= 1;
                    // get current symbol
                    self.curr_symbol =
                        match self.patterns[self.pattern_id].get_symbol_at_checked(self.pos_head) {
                            Some(s) => s,
                            None => {
                                self.pattern_id -= 1;
                                self.patterns[self.pattern_id].get_symbol_at(self.pos_head)
                            }
                        };
                }
            }
            true
        } else {
            // changed symbol
            self.set_current_symbol_and_move(transition)
        }
    }

    fn write_last_symbol(&mut self, transition: TransitionBinary) {
        if !transition.is_symbol_undefined() && self.curr_symbol != transition.symbol_usize() {
            self.set_current_symbol_and_move(transition);
        }
    }

    #[cfg(feature = "enable_html_reports")]
    fn pos_middle_print(&self) -> i64 {
        self.pos_head
    }

    #[cfg(feature = "enable_html_reports")]
    fn tape_shifted_clean(&self) -> u128 {
        // TODO allow fixed and shifted output
        self.as_tape_shifted()
    }
}

impl Display for TapeCompact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{}, P {} {}..{}",
            self.as_tape_fixed().to_binary_split_string(),
            self.pos_head,
            self.tape_start,
            self.tape_end,
        )?;
        let mut s = Vec::new();
        for (i, p) in self.patterns.iter().enumerate() {
            s.push(format!("  Pattern {i}: {}", p));
        }
        write!(f, "{}", s.join("\n"))
    }
}
