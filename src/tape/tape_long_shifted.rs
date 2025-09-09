//! This crate hosts functionality around the tape written by the Turing machines. \
//! [TapeLong] should be used in cases where work is done with a 128-bit `tape_shifted` as
//! it contains all the logic to update the tape correctly.
//!
//! Generally the work on the tape is split in two tapes, a `tape_shifted` of (usually) 128 bit, as Rust supports this natively
//! and a `long_tape` which is a `Vec<u32>` to cater everything not fitting in the short tape. \
//! The idea is to rapidly work with the tape_shifted (stack memory, register) and only access the `long_tape` (heap memory)
//! when it is absolutely necessary. Both tapes use 1 bit for a cell, allowing to write 0 and 1 only but saving a lot of memory. \
//! The 128-Bit `tape_shifted` can only work in the middle 64-Bit without requiring access to the `long_tape`, but this surprisingly
//! often allows iterating over a couple hundred steps reducing the access to the slow heap memory to a minimum. \
//! The `tape_shifted` always shifts underneath the head, so the head is always positions on bit 63 (or 31 in case of 64-bit) while
//! the `long_tape` never shifts. In case it is not long enough, it will be expanded at the beginning or end. \
//! This requires delicate logic to keep the data in sync, which will be explained here.
//!
//! To give a real life example, the bb_challenge file machine Id: 30605 1RB0RZ_0RC0RA_1RD0LE_0LC1RC_1LC0RA is used,
//! which can be created with [crate::examples::cycler_undecided::bb_challenge_cycler_undecided_to_html].
//!
//! Unfortunately the html doc is not wide enough, but for this example requires the full 128_bit. \
//! The tape will bounce around a bit: \
//! Step 1 shows the tape after it has been executed. It also shows the movement of the 'clean' section: P:64 meaning
//! the tape shifted one bit to the left (as the head moved right). \
//! TL P30 30..33 means: The current position of tape_shifted within the long tape is 30. As the tape is 128 bit long, four u32 are required, spanning ids 30..33 (inclusive). \
//! 30..33 is low_bound..high_bound, showing the used part of the tape (where actually the tape was changed). The long_tape has a bit room to the left and right as expanding the
//! tape at the beginning requires shifting of all elements which is quite expensive. \
//! Step     1 A0 1RB: 00000000000000000000000000000000_000000000000000000000000_00000001←00000000_000000000000000000000000_00000000000000000000000000000000 P: 64 TL P30 30..33 \
//! Step     2 B0 0RC: 00000000000000000000000000000000_000000000000000000000000_00000010←00000000_000000000000000000000000_00000000000000000000000000000000 P: 65 TL P30 30..33 \
//! Step     3 C0 1RD: 00000000000000000000000000000000_000000000000000000000000_00000101←00000000_000000000000000000000000_00000000000000000000000000000000 P: 66 TL P30 30..33 \
//! Step     4 D0 0LC: 00000000000000000000000000000000_000000000000000000000000_00000010→10000000_000000000000000000000000_00000000000000000000000000000000 P: 65 TL P30 30..33 \
//! Step     5 C1 0LE: 00000000000000000000000000000000_000000000000000000000000_00000001→00000000_000000000000000000000000_00000000000000000000000000000000 P: 64 TL P30 30..33 \
//! ...  \
//! Step   472 B1 0RA: 00000000000000000000000000000000_000011111110101010101010_10101010←01010101_000000000000000000000000_00000000000000000000000000000000 P: 85 TL P30 30..33 \
//! Step   473 A0 1RB: 00000000000000000000000000000000_000111111101010101010101_01010101←10101010_000000000000000000000000_00000000000000000000000000000000 P: 86 TL P30 30..33 \
//! Step   474 B1 0RA: 00000000000000000000000000000000_001111111010101010101010_10101010←01010100_000000000000000000000000_00000000000000000000000000000000 P: 87 TL P30 30..33 \
//! Step   475 A0 1RB: 00000000000000000000000000000000_011111110101010101010101_01010101←10101000_000000000000000000000000_00000000000000000000000000000000 P: 88 TL P30 30..33 \
//! Step   476 B1 0RA: 00000000000000000000000000000000_111111101010101010101010_10101010←01010000_000000000000000000000000_00000000000000000000000000000000 P: 89 TL P30 30..33 \
//! Step   477 A0 1RB: 00000000000000000000000000000001_111111010101010101010101_01010101←10100000_000000000000000000000000_00000000000000000000000000000000 P: 90 TL P30 30..33 \
//! Step   478 B1 0RA: 00000000000000000000000000000011_111110101010101010101010_10101010←01000000_000000000000000000000000_00000000000000000000000000000000 P: 91 TL P30 30..33 \
//! Step   479 A0 1RB: 00000000000000000000000000000111_111101010101010101010101_01010101←10000000_000000000000000000000000_00000000000000000000000000000000 P: 92 TL P30 30..33 \
//! Step   480 B1 0RA: 00000000000000000000000000001111_111010101010101010101010_10101010←00000000_000000000000000000000000_00000000000000000000000000000000 P: 93 TL P30 30..33 \
//! Step   481 A0 1RB: 00000000000000000000000000011111_110101010101010101010101_01010101←00000000_000000000000000000000000_00000000000000000000000000000000 P: 94 TL P30 30..33 \
//! At this point the tape is shifted 32 bits to the left (63 -> 95), meaning every further shift left will move out bits to nirvana. Therefore now the
//! upper 32 bits are saved into the tape. The lower bits of the tape will now always filled with 0, not showing the correct value. But that data is not required yet as
//! the head reads as bit 63 and we want to shift in as late as possible. \
//! This also shows how efficient the logic is as the long tape is not used until this point. \
//! 00000000000000000000000000111111 is now saved to tape_long[30] and the tape_shifted is relatively moved one cell block to P31, which now allows the
//! top 32 bits to be moved out without getting lost. \
//!   RIGHT SAVE HIGH P95-31: tape wanders left -> "00000000, 00000000, 0000003F, 00000000, 00000000, 00000000" \
//! Similarly the right side of the tape will now be filled with zeros on each shift. To allow safe shifting the the left, the 32 bits next to the head
//! are filled with the stored data of the `long_tape`, which is currently still 0. \
//! This may feel counterintuitive as not the lowest 32 bit are filled. This logic will read the data only when required and that is when the head reaches
//! the 'dirty' section. \
//! For tape_shifted this actually means the bits are only for sure correct on the second 32-bit section when the head moves right or on the third
//! 32-bit section when the tape moves left. \
//!   RIGHT LOAD LOW  P63-31: tape wanders left -> "00000000, 00000000, 0000003F, 00000000, 00000000, 00000000" \
//! Step   482 B0 0RC: 00000000000000000000000000111111_101010101010101010101010_10101010←00000000_000000000000000000000000_00000000000000000000000000000000 P: 63 TL P31 30..34 \
//! Step   483 C0 1RD: 00000000000000000000000001111111_010101010101010101010101_01010101←00000000_000000000000000000000000_00000000000000000000000000000000 P: 64 TL P31 30..34 \
//! Step   484 D0 0LC: 00000000000000000000000000111111_101010101010101010101010_10101010→10000000_000000000000000000000000_00000000000000000000000000000000 P: 63 TL P31 30..34 \
//! ... \
//! Step 513 0LE: P31-34 00000000000000000000000000000000_000000000000000000000001_11111101→00101010_101010101010101010101000_00000000000000000000000000000000 Next E0 \
//! Step 514 1LC: P31-33 00000000000000000000000000000000_000000000000000000000000_11111110→11010101_010101010101010101010100_00000000000000000000000000000000 Next C1 \
//! Step 515 0LE: P31-32 00000000000000000000000000000000_000000000000000000000000_01111111→00101010_101010101010101010101010_00000000000000000000000000000000 Next E0 \
//!   LEFT  SAVE HIGH P31-30: tape wanders right -> "00000000, 00000000, 0000003F, 00000000, 00000000, 00000000" \
//!   LEFT  LOAD HIGH P63-30: tape wanders right -> "00000000, 00000000, 0000003F, 00000000, 00000000, 00000000" \
//! Here the bits of the long tape are moved in again, marked in bold, but since these were not actually shifted out, this is not seen here. \
//! Step 516 1LC: P30-63 00000000000000000000000000000000_000000000000000000000000_00**111111**→11010101_010101010101010101010101_00000000000000000000000000000000 Next C1 \
//! Step 517 0LE: P30-62 00000000000000000000000000000000_000000000000000000000000_00011111→10101010_101010101010101010101010_10000000000000000000000000000000 Next E1 \
//! Step 518 0RA: P30-63 00000000000000000000000000000000_000000000000000000000000_00111110←01010101_010101010101010101010101_00000000000000000000000000000000 Next A0 \
//! ... \
//! Step  1735 A0 1RB: 01111111110101010101010101010101_010101010101010101010101_01010101←10100000_000000000000000000000000_00000000000000000000000000000000 P: 94 TL P31 30..35 \
//!   RIGHT SAVE HIGH P95-32: tape wanders left -> "00000000, 00000000, 000000FF, FFAAAAAA, 54000000, 40000000" \
//!   RIGHT LOAD LOW  P63-32: tape wanders left -> "00000000, 00000000, 000000FF, FFAAAAAA, 54000000, 40000000" \
//! Here one can clearly see, how some of the ones get 'lost' when shifting left and shifting right again. In step 1732 9 leading ones are seen,
//! in step 1742 only 4 are left.
//! Step  1736 B1 0RA: 11111111101010101010101010101010_101010101010101010101010_10101010←01000000_000000000000000000000000_00000000000000000000000000000000 P: 63 TL P32 30..35 \
//! Step  1737 A0 1RB: 11111111010101010101010101010101_010101010101010101010101_01010101←10000000_000000000000000000000000_00000000000000000000000000000000 P: 64 TL P32 30..35 \
//! Step  1738 B1 0RA: 11111110101010101010101010101010_101010101010101010101010_10101010←00000000_000000000000000000000000_00000000000000000000000000000000 P: 65 TL P32 30..35 \
//! Step  1739 A0 1RB: 11111101010101010101010101010101_010101010101010101010101_01010101←00000000_000000000000000000000000_00000000000000000000000000000000 P: 66 TL P32 30..35 \
//! Step  1740 B0 0RC: 11111010101010101010101010101010_101010101010101010101010_10101010←00000000_000000000000000000000000_00000000000000000000000000000000 P: 67 TL P32 30..35 \
//! Step  1741 C0 1RD: 11110101010101010101010101010101_010101010101010101010101_01010101←00000000_000000000000000000000000_00000000000000000000000000000000 P: 68 TL P32 30..35 \
//! Step  1742 D0 0LC: 01111010101010101010101010101010_101010101010101010101010_10101010→10000000_000000000000000000000000_00000000000000000000000000000000 P: 67 TL P32 30..35 \
//! Step  1743 C1 0LE: 00111101010101010101010101010101_010101010101010101010101_01010101→00000000_000000000000000000000000_00000000000000000000000000000000 P: 66 TL P32 30..35 \
//! ... \
//! Step  1776 E0 1LC: 00000000000000000000000000000000_000111101010101010101010_10101010→11010101_010101010101010101010101_01000000000000000000000000000000 P: 33 TL P32 30..35 \
//! Step  1777 C1 0LE: 00000000000000000000000000000000_**00001111**0101010101010101_01010101→00101010_101010101010101010101010_10100000000000000000000000000000 P: 32 TL P32 30..35 \
//!  LEFT  SAVE HIGH P31-31: tape wanders right -> "00000000, 00000000, 000000FF, FFAAAAAA, 54000000, 50000000" \
//!  LEFT  LOAD HIGH P63-31: tape wanders right -> "00000000, 00000000, 000000FF, FFAAAAAA, 54000000, 50000000" \
//! Only here the missing ones are loaded again. This is why one can see these jumps in the tape. \
//! Step  1778 E0 1LC: 00000000000000000000000000000000_**111111111**010101010101010_10101010→11010101_010101010101010101010101_01010000000000000000000000000000 P: 63 TL P31 30..35 \
//! Step  1779 C1 0LE: 00000000000000000000000000000000_011111111101010101010101_01010101→00101010_101010101010101010101010_10101000000000000000000000000000 P: 62 TL P31 30..35 \

use crate::{
    config::{Config, StepBig, MAX_TAPE_GROWTH_BLOCKS, TAPE_SIZE_INIT_CELL_BLOCKS},
    tape::{
        tape_utils::{
            TapeLongPositions, U128Ext, CLEAR_HIGH127_96BITS_U128, CLEAR_HIGH95_64BITS_U128,
            CLEAR_LOW31_00BITS_U128, CLEAR_LOW63_00BITS_U128, CLEAR_LOW63_32BITS_U128,
            HIGH32_SWITCH_U128, LOW32_SWITCH_U128, MIDDLE_BIT_U128, POS_HALF_U128,
            TAPE_SIZE_FOURTH_UPPER_128, TAPE_SIZE_HALF_128, TL_POS_START_128,
        },
        Tape, TapeAcceleration,
    },
    transition_binary::TransitionBinary,
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
pub struct TapeLongShifted {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    /// The tape is 128 bit wide, but since data is shifted to the long tape, it may be 'dirty', meaning it
    /// does not contain the correct cell values. The cell values will be shifted in only if required. \
    /// In case the head goes left for a large number of steps, the bits next to the head will only be loaded
    /// one step before. The tape is clean as long as it never reached the outer limits.
    pub tape_shifted: u128,
    /// Indication where the original pos_middle has moved within tape_shifted. Used to load data from long_tape.
    pub pos_middle: u32,
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

impl TapeLongShifted {
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

    #[cfg(feature = "enable_html_reports")]
    /// Returns tape_shifted with the correct bits set (taken from tape_long).
    fn get_clean_tape_shifted(&self) -> u128 {
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("{}", self.long_tape_to_string());
        let mut ts = self.tape_shifted;
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("shifted org:  {}", ts.to_binary_split_string());
        // shift tape back to fill correctly
        // update dirty section
        #[allow(clippy::comparison_chain)]
        if self.pos_middle < MIDDLE_BIT_U128 {
            // Here bits 63-32 are clean, all other are potentially dirty.
            let shift = MIDDLE_BIT_U128 - self.pos_middle;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            dbg!(shift, self.pos_middle);
            ts <<= shift;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted mid:  {}", ts.to_binary_split_string());
            ts &= CLEAR_HIGH127_96BITS_U128;
            // bits 127-96 are always stored
            ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            // bits 95-64 may not be stored
            // if self.tl_high_bound > self.tl_pos + 4 {
            //     //  unclear if this is necessary and/or correct
            // If this is called for each step, then there is no need to update the middle part, it is filled correctly (This would require ts to be written back to self.tape_shifted).
            // The problem is, that it is unclear, if the tape_long even is filled. The conditions need to be checked in more detail.
            // Problematic only in large jumps, but those jumps should fill tape_shifted fully.
            //     // let test = ts;
            //     ts &= CLEAR_HIGH95_64BITS_U128;
            //     ts |= (self.tape_long[self.tl_pos + 1] as u128) << 64;
            //     // if ts != test {
            //     //     println!("changed");
            //     // }
            // }
            if self.tl_high_bound > self.tl_pos + 3 {
                ts &= CLEAR_LOW31_00BITS_U128;
                ts |= self.tape_long[self.tl_pos + 3] as u128;
            }
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted fil:  {}", ts.to_binary_split_string());
            // shift back to original position
            ts >>= shift;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted bck:  {}", ts.to_binary_split_string());
            // now the first bits are dirty again, fill them
            let mut extra = self.tape_long[self.tl_pos] as u64;
            if self.tl_pos > self.tl_low_bound {
                extra |= (self.tape_long[self.tl_pos - 1] as u64) << 32;
            }
            // no clean as only missing bits are added
            ts |= ((extra >> shift) as u128) << 96;
        } else if self.pos_middle > MIDDLE_BIT_U128 {
            // Here bits 95-64 are clean, all other are potentially dirty.
            let shift = self.pos_middle - MIDDLE_BIT_U128;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            dbg!(shift, self.pos_middle);
            ts >>= shift;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted mid:  {}", ts.to_binary_split_string());
            // bits 31-0 are always stored
            ts &= CLEAR_LOW31_00BITS_U128;
            ts |= self.tape_long[self.tl_pos + 3] as u128;
            // bits 63-32 may not be stored
            // if self.tl_low_bound + 2 < self.tl_pos {
            //     //  unclear if this is necessary and/or correct
            //     let test = ts;
            //     ts &= CLEAR_LOW63_32BITS_U128;
            //     ts |= (self.tape_long[self.tl_pos + 2] as u128) << 32;
            //     if ts != test {
            //         println!("changed");
            //     }
            // }
            // load bits 127-96 if tape has wandered
            if self.tl_pos > self.tl_low_bound {
                ts &= CLEAR_HIGH127_96BITS_U128;
                ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            }
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted fil:  {}", ts.to_binary_split_string());
            // shift back to original position
            ts <<= shift;
            // now the last bits are dirty again, fill them
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted bck:  {}", ts.to_binary_split_string());
            let mut extra = (self.tape_long[self.tl_pos + 3] as u64) << 32;
            if self.tl_pos + 3 < self.tl_high_bound {
                extra |= self.tape_long[self.tl_pos + 4] as u64;
            }
            ts |= (extra >> (32 - shift)) as u128;
        } else {
            // In the middle, both middle u32 are clean, one of them just loaded.
            // Also position matches tape_long, just load both outer u32.
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            dbg!(self.tl_pos);
            ts &= CLEAR_HIGH127_96BITS_U128;
            ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            ts &= CLEAR_LOW31_00BITS_U128;
            ts |= self.tape_long[self.tl_pos + 3] as u128;
        }
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("shifted end:  {}\n", ts.to_binary_split_string());

        ts
    }

    /// Shifts and cleans tape_shifted, so it can be inserted in tape_long.
    pub fn get_clean_tape_shifted_for_tape_long(&self) -> u128 {
        // This is the same logic as in get_clean_tape_shifted, but only until fil
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("{}", self.long_tape_to_string());
        let mut ts = self.tape_shifted;
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("shifted org:  {}", ts.to_binary_split_string());
        // shift tape back to fill correctly
        // update dirty section
        #[allow(clippy::comparison_chain)]
        if self.pos_middle < MIDDLE_BIT_U128 {
            // Here bits 63-32 are clean, all other are potentially dirty.
            let shift = MIDDLE_BIT_U128 - self.pos_middle;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            dbg!(shift, self.pos_middle);
            ts <<= shift;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted mid:  {}", ts.to_binary_split_string());
            ts &= CLEAR_HIGH127_96BITS_U128;
            // bits 127-96 are always stored
            ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            // bits 95-64 may not be stored
            // if self.tl_high_bound > self.tl_pos + 4 {
            //     //  unclear if this is necessary and/or correct
            // If this is called for each step, then there is no need to update the middle part, it is filled correctly (This would require ts to be written back to self.tape_shifted).
            // The problem is, that it is unclear, if the tape_long even is filled. The conditions need to be checked in more detail.
            // Problematic only in large jumps, but those jumps should fill tape_shifted fully.
            //     // let test = ts;
            //     ts &= CLEAR_HIGH95_64BITS_U128;
            //     ts |= (self.tape_long[self.tl_pos + 1] as u128) << 64;
            //     // if ts != test {
            //     //     println!("changed");
            //     // }
            // }
            if self.tl_high_bound > self.tl_pos + 3 {
                ts &= CLEAR_LOW31_00BITS_U128;
                ts |= self.tape_long[self.tl_pos + 3] as u128;
            }
            // println!("shifted fil:  {}", ts.to_binary_split_string());
        } else if self.pos_middle > MIDDLE_BIT_U128 {
            // Here bits 95-64 are clean, all other are potentially dirty.
            let shift = self.pos_middle - MIDDLE_BIT_U128;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            dbg!(shift, self.pos_middle);
            ts >>= shift;
            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!("shifted mid:  {}", ts.to_binary_split_string());
            // bits 31-0 are always stored
            ts &= CLEAR_LOW31_00BITS_U128;
            ts |= self.tape_long[self.tl_pos + 3] as u128;
            // bits 63-32 may not be stored
            // if self.tl_low_bound + 2 < self.tl_pos {
            //     //  unclear if this is necessary and/or correct
            //     let test = ts;
            //     ts &= CLEAR_LOW63_32BITS_U128;
            //     ts |= (self.tape_long[self.tl_pos + 2] as u128) << 32;
            //     if ts != test {
            //         println!("changed");
            //     }
            // }
            // load bits 127-96 if tape has wandered
            if self.tl_pos > self.tl_low_bound {
                ts &= CLEAR_HIGH127_96BITS_U128;
                ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            }
            // println!("shifted fil:  {}", ts.to_binary_split_string());
        } else {
            // In the middle, both middle u32 are clean, one of them just loaded.
            // Also position matches tape_long, just load both outer u32.
            dbg!(self.tl_pos);
            ts &= CLEAR_HIGH127_96BITS_U128;
            ts |= (self.tape_long[self.tl_pos] as u128) << 96;
            ts &= CLEAR_LOW31_00BITS_U128;
            ts |= self.tape_long[self.tl_pos + 3] as u128;
        }
        #[cfg(all(debug_assertions, feature = "debug_tape"))]
        println!("shifted end:  {}\n", ts.to_binary_split_string());

        // self.tape_long[self.tl_pos] = (ts >> 96) as u32;
        // self.tape_long[self.tl_pos + 1] = (ts >> 64) as u32;
        // self.tape_long[self.tl_pos + 2] = (ts >> 32) as u32;
        // self.tape_long[self.tl_pos + 3] = ts as u32;

        ts
    }

    /// Tape shifted is clean (contains the correct cell values) as long the bounds have not been breached.
    #[inline(always)]
    pub fn is_tape_extended(&self) -> bool {
        self.tl_high_bound - self.tl_low_bound > 3
    }

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
                    > self.tape_size_limit_u32_blocks as usize
                {
                    grow_by = self.tape_size_limit_u32_blocks as usize + self.tl_low_bound
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
        // normal shift LEFT -> tape moves right
        if self.pos_middle == LOW32_SWITCH_U128 {
            // save high bytes
            if !self.shift_pos_to_left_checked() {
                return false;
            }

            // The shift is left, so tape_shifted wanders right -> store low 32 bits.
            self.tape_long[self.tl_pos + 3] = self.tape_shifted as u32;

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!(
                "  LEFT  SAVE HIGH P{}-{}: tape wanders right -> {:?}",
                self.pos_middle,
                self.tl_pos,
                crate::tape::tape_utils::VecU32Ext::to_hex_string_range(
                    &self.tape_long,
                    crate::tape::tape_utils::TAPE_DISPLAY_RANGE_128
                )
            );

            self.pos_middle = MIDDLE_BIT_U128;

            // load high bytes
            self.tape_shifted = (self.tape_shifted & CLEAR_HIGH95_64BITS_U128)
                | ((self.tape_long[self.tl_pos + 1] as u128) << TAPE_SIZE_HALF_128);

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            {
                println!(
                    "  ALoad {}",
                    crate::tape::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                );
                println!(
                    "  LEFT  LOAD HIGH P{}-{}: tape wanders right -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    crate::tape::tape_utils::VecU32Ext::to_hex_string_range(
                        &self.tape_long,
                        crate::tape::tape_utils::TAPE_DISPLAY_RANGE_128
                    )
                );
                print!("");
            }
            // let _x = self.get_clean_tape_shifted();
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

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            println!(
                "  RIGHT SAVE HIGH P{}-{}: tape wanders left -> {:?}",
                self.pos_middle,
                self.tl_pos,
                crate::tape::tape_utils::VecU32Ext::to_hex_string_range(
                    &self.tape_long,
                    crate::tape::tape_utils::TAPE_DISPLAY_RANGE_128
                )
            );

            self.pos_middle = MIDDLE_BIT_U128;

            // Load low 32 bit
            // if self.tl_pos + 2 >= self.tape_long.len() {
            //     println!(
            //         "Error shift: TL len {}, tl_pos +2 {}",
            //         self.tape_long.len(),
            //         self.tl_pos + 2 // "Step {}: {}, {}",
            //                         // self.step_no, tr, transition_table
            //     );
            //     // } else {
            // }
            self.tape_shifted = (self.tape_shifted & CLEAR_LOW63_32BITS_U128)
                | ((self.tape_long[self.tl_pos + 2] as u128) << 32);

            #[cfg(all(debug_assertions, feature = "debug_tape"))]
            {
                use crate::tape::tape_utils::{VecU32Ext as _, TAPE_DISPLAY_RANGE_128};

                println!(
                    "  ALoad {}",
                    crate::tape::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                );
                println!(
                    "  RIGHT LOAD LOW  P{}-{}: tape wanders left -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );
                print!("");
            }
            // let _x = self.get_clean_tape_shifted();
        }

        true
    }

    //     pub fn tape_size_limit_u32_blocks(&self) -> u32 {
    //         self.tape_size_limit_u32_blocks
    //     }

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

    // /// Normally the four current bytes of the tape are dirty. This uses tape_shifted to update the tape, so it is fully correct.
    // Probably wrong
    // pub fn update_long_tape_with_tape_shifted(&mut self) {
    //     println!("{}", self.long_tape_to_string());
    //     println!(
    //         "tape_shifted: {}",
    //         self.tape_shifted.to_binary_split_string()
    //     );
    //     println!("Pos middle = {}", self.pos_middle);
    //     let mut ts = self.tape_shifted;
    //     // move tape to middle to match tape long positioning
    //     let shift = MIDDLE_BIT_U128 as isize - self.pos_middle as isize;
    //     ts <<= shift;
    //     // update from changed section of tape_shifted
    //     if self.pos_middle <= MIDDLE_BIT_U128 {
    //         self.tape_long[self.tl_pos + 2] = (ts >> 32) as u32;
    //     } else {
    //         self.tape_long[self.tl_pos + 1] = (ts >> 64) as u32;
    //     }
    //     println!("shifted new:  {}", ts.to_binary_split_string());
    //     println!("{}", self.long_tape_to_string());
    // }

    pub fn long_tape_to_string(&self) -> String {
        let mut cell_blocks = Vec::new();
        for (i, cell_block) in self.tape_long[self.tl_low_bound..self.tl_pos]
            .iter()
            .enumerate()
        {
            let s = format!("Pos {}: {cell_block:032b}", self.tl_low_bound + i);
            cell_blocks.push(s);
        }

        cell_blocks.push("tape long for tape_shifted:".to_string());
        for (i, cell_block) in self.tape_long[self.tl_pos..self.tl_pos + 4]
            .iter()
            .enumerate()
        {
            let s = format!(
                "Pos {}: {cell_block:032b} = {cell_block:08X}",
                self.tl_pos + i
            );
            cell_blocks.push(s);
        }

        if self.tl_high_bound > self.tl_pos + 3 {
            cell_blocks.push("".to_string());
            for (i, cell_block) in self.tape_long[self.tl_pos + 4..self.tl_high_bound + 1]
                .iter()
                .enumerate()
            {
                let s = format!("Pos {}: {cell_block:032b}", self.tl_pos + 4 + i);
                cell_blocks.push(s);
            }
        }

        cell_blocks.join("\n")
    }
}

impl Tape for TapeLongShifted {
    fn new(config: &Config) -> Self {
        Self {
            tape_size_limit_u32_blocks: config.tape_size_limit_u32_blocks(),
            ..Default::default()
        }
    }

    // resets the decider for a different machine
    #[inline(always)]
    fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128;

        self.tape_long.clear();
        self.tape_long.resize(TAPE_SIZE_INIT_CELL_BLOCKS, 0);
        self.tl_pos = TL_POS_START_128;
        self.tl_low_bound = TL_POS_START_128;
        self.tl_high_bound = TL_POS_START_128 + 3;
    }

    /// Returns the ones which are set in the tape
    fn count_ones(&self) -> u32 {
        let ts = self.get_clean_tape_shifted_for_tape_long();

        // TODO tape shifted needs to be shifted in the middle and tape long loaded
        let mut ones = ts.count_ones();
        if self.is_tape_extended() {
            for n in self.tape_long[self.tl_low_bound..self.tl_pos].iter() {
                ones += n.count_ones();
            }
            for n in self.tape_long[self.tl_pos + 4..self.tl_high_bound + 1].iter() {
                ones += n.count_ones();
            }
        }
        ones
    }

    #[inline(always)]
    fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
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

    fn tape_long_positions(&self) -> Option<TapeLongPositions> {
        Some(TapeLongPositions {
            tl_pos: self.tl_pos,
            tl_high_bound: self.tl_high_bound,
            tl_low_bound: self.tl_low_bound,
        })
    }

    #[cfg(feature = "enable_html_reports")]
    fn tape_shifted_clean(&self) -> u128 {
        self.get_clean_tape_shifted()
    }

    /// Returns the approximate tape size, which grows by 32 steps
    #[inline(always)]
    fn tape_size_cells(&self) -> u32 {
        ((self.tl_high_bound - self.tl_low_bound + 1) * 32) as u32
    }

    /// Updates tape_shifted and tape_long.
    /// # Returns
    /// False if the tape could not be expanded (tape_size_limit).
    #[inline(always)]
    fn update_tape_single_step(&mut self, transition: TransitionBinary) -> bool {
        // set current symbol
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

    #[inline(always)]
    fn write_last_symbol(&mut self, transition: TransitionBinary) {
        if !transition.is_symbol_undefined() {
            self.set_current_symbol(transition);
        }
    }
}

impl TapeAcceleration for TapeLongShifted {
    fn update_tape_self_ref_speed_up(&mut self, tr: TransitionBinary, tr_field: usize) -> StepBig {
        let mut jump;
        // Check if self referencing, which speeds up the shift greatly.
        // Self referencing means also that the symbol does not change, ergo no need to update the fields
        if tr.self_ref_array_id() == tr_field {
            if tr.is_dir_right() {
                // normal shift RIGHT -> tape moves left

                // get jump within tape_shifted, which is only the lower part and thus a maximum of 63 bits
                jump = self.count_right(tr_field & 1);
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
                    let v32 = if tr_field & 1 == 0 { 0 } else { u32::MAX };
                    // head goes right, tape shifts left
                    // tl_pos + 2 is now a known required value v32, because that is what count_right just tested
                    let mut p = self.tl_pos() + 3;
                    let mut j = 1;
                    while p < self.tl_high_bound() && self.tape_long[p] == v32 {
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
                        let p_tmp = self.tl_pos() + 1;
                        self.tape_long[p_tmp] = tape_shifted_left_1;
                        self.set_tl_pos(p - 3);
                        // println!("before {}", self_shifted.to_binary_split_string());
                        self.tape_shifted = if tr_field & 1 == 0 {
                            0
                        } else {
                            CLEAR_LOW63_00BITS_U128
                        };
                        // println!("filled {}", self_shifted.to_binary_split_string());
                        self.pos_middle = HIGH32_SWITCH_U128;
                        jump = j * 32;
                        // shift in low bits (low part is already cleared)
                        self.tape_shifted |= (self.tape_long[self.tl_pos() + 3] as u128) << 32;
                        // println!("fill 2 {}", self_shifted.to_binary_split_string());
                        long_jump = true;
                    }
                }
                if !long_jump {
                    if self.pos_middle + jump > HIGH32_SWITCH_U128 {
                        jump = HIGH32_SWITCH_U128 - self.pos_middle;
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  jump right adjusted {jump}");
                    }
                    self.pos_middle += jump;

                    // shift tape
                    // self.set_current_symbol(); not required as it does not change
                    self.tape_shifted <<= jump;
                }
                // #[cfg(feature = "enable_html_reports")]
                // if self.write_html_step_limit > 0 {
                //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                //     let s = format!(
                //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                //         self.num_steps,
                //         self.tl_pos,
                //         self_long[tl_pos_min_1],
                //         self_long[self.tl_pos],
                //         self_long[self.tl_pos + 1],
                //         self_long[self.tl_pos + 2],
                //         self_long[self.tl_pos + 3],
                //         self_long[self.tl_pos + 4],
                //     );
                //     self.write_html_p(&s);
                // }

                self.shift_tape_long_head_dir_right()
            } else {
                // normal shift LEFT -> tape moves right
                jump = self.count_left(tr_field & 1);
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                // Interestingly, the version with the long_jump logic runs faster.
                let mut long_jump = false;
                if jump == 33 && LOW32_SWITCH_U128 - 1 + jump == self.pos_middle {
                    // check further for larger jump
                    // compare depending on symbol
                    let v32 = if tr_field & 1 == 0 { 0 } else { u32::MAX };
                    // head goes left, tape shifts right
                    // tl_pos + 1 is known required value v32, because that is what count_left just tested
                    let mut p = self.tl_pos();
                    let mut j = 1;
                    while p >= self.tl_low_bound() && self.tape_long[p] == v32 {
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
                        let p_tmp = self.tl_pos() + 2;
                        self.tape_long[p_tmp] = tape_shifted_left_2;
                        self.set_tl_pos(p);
                        // println!("before {}", self_shifted.to_binary_split_string());
                        self.tape_shifted = if tr_field & 1 == 0 {
                            0
                        } else {
                            u64::MAX as u128
                        };
                        // println!("filled {}", self_shifted.to_binary_split_string());
                        self.pos_middle = LOW32_SWITCH_U128;
                        jump = j * 32;
                        // shift in high bits (high part is already cleared)
                        self.tape_shifted |=
                            (self.tape_long[self.tl_pos()] as u128) << TAPE_SIZE_HALF_128;
                        // println!("fill 2 {}", self_shifted.to_binary_split_string());
                        long_jump = true;
                    }
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
                }
                // #[cfg(feature = "enable_html_reports")]
                // if self.write_html_step_limit > 0 && self.num_steps < self.write_html_step_limit
                // {
                //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                //     let s = format!(
                //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                //         self.num_steps,
                //         self.tl_pos,
                //         self_long[tl_pos_min_1],
                //         self_long[self.tl_pos],
                //         self_long[self.tl_pos + 1],
                //         self_long[self.tl_pos + 2],
                //         self_long[self.tl_pos + 3],
                //         self_long[self.tl_pos + 4],
                //     );
                //     self.write_html_p(&s);
                // }

                self.shift_tape_long_head_dir_left()
            };
        } else {
            let r = self.update_tape_single_step(tr);
            jump = r as StepBig;
        }
        jump
    }
}

impl Default for TapeLongShifted {
    fn default() -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128,
            tape_long: vec![0; TAPE_SIZE_INIT_CELL_BLOCKS],
            tl_pos: TL_POS_START_128,
            tl_low_bound: TL_POS_START_128,
            tl_high_bound: TL_POS_START_128 + 3,
            tape_size_limit_u32_blocks: u32::MAX,
        }
    }
}

impl std::fmt::Display for TapeLongShifted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} P{:3}, B {}..{}",
            self.tape_shifted.to_binary_split_string(),
            self.pos_middle,
            self.tl_high_bound,
            self.tl_low_bound
        )
    }
}
