use std::ops::Range;

use crate::{
    config::{StepSmall, TAPE_SIZE_INIT_CELLS},
    html,
    transition_binary::TransitionBinary,
};

pub const TAPE_SIZE_BIT_U128: u32 = 128;
pub const TAPE_SIZE_HALF_128: u32 = TAPE_SIZE_BIT_U128 / 2;
pub const TAPE_SIZE_FOURTH_128: u32 = TAPE_SIZE_BIT_U128 / 4;
pub const TAPE_SIZE_FOURTH_UPPER_128: u32 = TAPE_SIZE_BIT_U128 / 4 + TAPE_SIZE_HALF_128;
pub const MIDDLE_BIT_U128: u32 = TAPE_SIZE_BIT_U128 / 2 - 1;
pub const POS_HALF_U128: u128 = 1 << MIDDLE_BIT_U128;
pub const TL_POS_START_128: usize = TAPE_SIZE_INIT_CELLS / 32 / 2 - 2;
// const LOW32_SWITCH_U128: usize = MIDDLE_BIT_U128 - TAPE_SIZE_FOURTH;
pub const LOW32_SWITCH_U128: u32 = MIDDLE_BIT_U128 - TAPE_SIZE_FOURTH_128;
pub const HIGH32_SWITCH_U128: u32 = MIDDLE_BIT_U128 + TAPE_SIZE_FOURTH_128;
pub const CLEAR_LOW63_00BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_00000000;
pub const CLEAR_LOW31_00BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_FFFFFFFF_00000000;
pub const CLEAR_LOW63_32BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_FFFFFFFF;
pub const CLEAR_HIGH95_64BITS_U128: u128 = 0xFFFFFFFF_00000000_FFFFFFFF_FFFFFFFF;
pub const CLEAR_HIGH127_96BITS_U128: u128 = 0x00000000_FFFFFFFF_FFFFFFFF_FFFFFFFF;
pub const CLEAR_HIGH127_64BITS_U128: u128 = 0x00000000_00000000_FFFFFFFF_FFFFFFFF;
pub const FILTER_HIGH_BITS_U128: u128 = 0xFFFFFFFF_FFFFFFFF_00000000_00000000;
pub const FILTER_HIGH_BITS_INCLUDING_HEAD_U128: u128 = 0xFFFFFFFF_FFFFFFFF_80000000_00000000;
pub const FILTER_HIGH_MIDDLE_BITS_U128: u128 = 0x00000000_FFFFFFFF_00000000_00000000;
pub const FILTER_LOW_MIDDLE_BITS_U128: u128 = 0x00000000_00000000_FFFFFFFF_00000000;
pub const FILTER_LOW_BITS_U128: u128 = 0x00000000_00000000_FFFFFFFF_FFFFFFFF;

pub const TAPE_SIZE_BIT_U64: StepSmall = 64;
pub const MIDDLE_BIT_U64: StepSmall = TAPE_SIZE_BIT_U64 / 2 - 1;
pub const POS_HALF_U64: u64 = 1 << MIDDLE_BIT_U64;
pub const FILTER_HIGH_BITS_INCLUDING_HEAD_U64: u64 = 0xFFFFFFFF_80000000;
pub const FILTER_LOW_BITS_U64: u64 = 0x00000000_FFFFFFFF;

pub const TAPE_DISPLAY_RANGE_128: std::ops::Range<usize> =
    TL_POS_START_128 - 1..TL_POS_START_128 + 5;

#[derive(Debug, Default, Clone, Copy)]
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
    fn to_binary_split_html_string(&self, tr: &TransitionBinary) -> String;
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

    fn to_binary_split_html_string(&self, tr: &TransitionBinary) -> String {
        if tr.is_halt() {
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
    fn to_binary_split_html_string(&self, tr: &TransitionBinary) -> String;
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

    fn to_binary_split_html_string(&self, tr: &TransitionBinary) -> String {
        if tr.is_halt() {
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
        let mut s = Vec::new();
        for cell_pack in self[range.start..range.end].iter() {
            s.push(format!("{cell_pack:08X}"));
        }

        s.join(" ")
    }
}

pub trait VecU64Ext {
    fn to_hex_string_range(&self, range: Range<usize>) -> String;
}

impl VecU64Ext for Vec<u64> {
    fn to_hex_string_range(&self, range: Range<usize>) -> String {
        let mut s = Vec::new();
        for cell_pack in self[range.start..range.end].iter() {
            s.push(format!("{cell_pack:016X}"));
        }

        s.join(" ")
    }
}
