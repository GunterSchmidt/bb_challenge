use std::fmt::Display;

use crate::config::{MAX_STATES_GENERIC, MAX_SYMBOLS_GENERIC};

pub const A: StateType = 1;
pub const B: StateType = 2;
pub const C: StateType = 3;
pub const D: StateType = 4;
pub const E: StateType = 5;
pub const F: StateType = 6;
pub const G: StateType = 7;
pub const H: StateType = 8;
pub const I: StateType = 9;
pub const J: StateType = 10;

/// tape cell storage size, can be any i/u available, e.g. u8, i32, with u16 seems fastest
/// CellType and MoveType affect Transition size, be careful, u16 makes nice 4 byte struct
pub type CellType = u16;
pub type MoveType = i8;
pub type StateType = u8;
pub type TransitionTableGenericArray =
    [[TransitionGeneric; MAX_SYMBOLS_GENERIC]; MAX_STATES_GENERIC + 1];

/// Default Unused for all transition fields. \
/// As this is an array a marker is required to identify the end of the used data, if n_states is not provided.
/// This uses less space than a separate field.
pub const TRANSITION_TABLE_GENERIC_DEFAULT: TransitionTableGenericArray = [[TransitionGeneric {
    symbol_write: SYMBOL_UNUSED,
    direction: 0,
    state_next: 0,
}; MAX_SYMBOLS_GENERIC];
    MAX_STATES_GENERIC + 1];

/// Undefined (the machine halts if it ever reaches an undefined transition), written as '---' for the transition.
pub const SYMBOL_UNDEFINED: CellType = u8::MAX as CellType;
/// Marker unused, as the data model uses an array with unused fields. Never exposed to user.
pub const SYMBOL_UNUSED: CellType = u8::MAX as CellType - 1;
pub const DIR_RIGHT: MoveType = 1;
pub const DIR_LEFT: MoveType = -1;
pub const DIR_UNDEFINED: MoveType = 0;
pub const STATE_HALT_GENERIC: StateType = 0;
pub const TRANSITION_HALT: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNDEFINED,
    direction: 0,
    state_next: 0,
};
pub const TRANSITION_UNUSED: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNUSED,
    direction: 0,
    state_next: 0,
};

/// This generic Transition can handle Turing machines, which write symbols up to 6 (MAX_SYMBOLS). \
/// It is designed to be a human understandable format and used during parsing. It is not designed to work with
/// for running the machine or deciders as there are more efficient formats. This library is generally limited to
/// the two symbols 0 and 1 for the bb_challenge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionGeneric {
    /// symbol 0,1,2.. or 255 for undefined. 254 represents unused and is used to determine the number of states.
    pub symbol_write: CellType,
    /// direction +1, left -1 or 0 for undefined
    pub direction: MoveType,
    /// Next state of the machine, 0 represents halt signal
    pub state_next: StateType,
}

impl TransitionGeneric {
    /// New transition from human readable format, e.g. 1RB, 1RZ or --- used in bb_challenge, which is String.as_bytes[0..3]. \
    /// \[symbol,direction,status\]
    /// With first char the symbol to write on the tape, can be 0, 1 or any other char as undefined. \
    /// The distinction between 0,1 and undefined is relevant in the last transition. \
    /// 0,1 will write and hold in the last transition, undefined will only hold. \
    /// This results in a different 'number of ones' count. \
    /// BBChallenge uses both notations. \
    /// Second car is L or R for direction or any other for undefined. \
    /// L,R or undefined is irrelevant in last transition, in any other step it must be defined. \
    /// Third car is next state, it can be denoted as number 1-9, or letter A-Y. 0 or Z represent halt. \
    /// This is the main halt condition. Numbers are used for the downloadable seeds.
    pub fn new(transition: [u8; 3]) -> Self {
        assert!(transition.len() == 3);
        // special hold in case of array
        if transition[2] == 0 {
            return TRANSITION_HALT;
        }
        let write_symbol = match transition[0] {
            0..=9 => transition[0] as CellType,
            b'0'..=b'9' => (transition[0] - b'0') as CellType,
            _ => SYMBOL_UNDEFINED,
        };
        let direction = match transition[1] {
            1 | b'L' => DIR_LEFT,
            0 | b'R' => DIR_RIGHT,
            _ => 0,
        };
        let state_next = match transition[2] {
            1..9 => transition[2],
            b'1'..b'9' => transition[2] - b'0',
            b'A'..b'Y' => transition[2] - b'A' + 1,
            // b'-' | b'Z' => 0,
            _ => STATE_HALT_GENERIC,
        };
        assert!(state_next <= MAX_STATES_GENERIC as u8);

        Self {
            symbol_write: write_symbol,
            direction,
            state_next,
        }
    }

    pub fn is_unused(&self) -> bool {
        self.symbol_write == SYMBOL_UNUSED
    }
}

impl Default for TransitionGeneric {
    fn default() -> Self {
        TRANSITION_UNUSED
    }
}

// TODO Own Error Type or Simple Error crate
impl TryFrom<&str> for TransitionGeneric {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 3 {
            return Err("Transition must have length of 3");
        }
        Ok(TransitionGeneric::new(value.as_bytes().try_into().unwrap()))
    }
}

/// Displays the transition in Standard TM Text Format.
impl Display for TransitionGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_unused()
            || (self.state_next == STATE_HALT_GENERIC && self.direction == DIR_UNDEFINED)
        {
            return write!(f, "---");
        }
        let write_symbol = match self.symbol_write {
            0..=9 => (self.symbol_write as u8 + b'0') as char,
            _ => '-',
        };
        let move_next = match self.direction {
            -1 => 'L',
            1 => 'R',
            _ => '-',
        };
        let next_state = if self.state_next == 0 {
            'Z'
        } else {
            (self.state_next + 64) as char
        };
        write!(f, "{write_symbol}{move_next}{next_state}")
    }
}
