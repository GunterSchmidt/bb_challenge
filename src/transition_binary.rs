//! This file contains the transition for binary Turing machines. \
//! Contrary to a more natural setup, this transition does not use Enums. Instead the data
//! is packed into one byte. This allows a number of very fast bit operations for comparison,
//! addition and array index access. \
//! While the actual data is in one byte only, i16 is used, as this seems faster when the CPU
//! needs to convert the number into a 32 bit number internally. \
//! With only 2 bytes for each transition, a Turing Machine can be stored in just 24 bytes
//! (5 * 2 transitions and 2 dummy transitions.). This reduces the memory footprint considerably. \
//! In debug mode the transition is also carried as chars to allow an easy understanding of the data in the debugger. \
//! See struct [TransitionBinary] for details.

use crate::config::MAX_STATES;
use crate::machine_binary::MachineBinary;
use crate::machine_generic::TransitionGeneric;

/// Number format to represent a transition (lower 8 bit used for state, symbol and direction).
/// Can be any type other than i8/u8 (potential extra info per transition). i16 seems fastest.
pub type TransitionType = i16;
/// Number format for direction which is either -1 or 1. Can be any iXX type, i16 seems fastest.
pub type DirectionType = i16;
pub const TRANSITION_SYM2_UNUSED: TransitionBinary = TransitionBinary {
    transition: TRANSITION_BINARY_UNUSED,
    #[cfg(debug_assertions)]
    text: ['_', '_', '_'],
};
// This is the undefined hold ('---'), where no last symbol is written.
pub const TRANSITION_SYM2_HOLD: TransitionBinary = TransitionBinary {
    transition: TRANSITION_HOLD,
    #[cfg(debug_assertions)]
    text: ['-', '-', '-'],
};
/// Initialize transition with A0 as start
pub const TRANSITION_SYM2_START: TransitionBinary = TransitionBinary {
    transition: TRANSITION_0RA,
    #[cfg(debug_assertions)]
    text: ['0', 'R', 'A'],
};
pub const TRANSITIONS_FOR_A0: [TransitionBinary; 2] = [
    TransitionBinary {
        transition: 196,
        #[cfg(debug_assertions)]
        text: ['0', 'R', 'B'],
    },
    TransitionBinary {
        transition: 197,
        #[cfg(debug_assertions)]
        text: ['1', 'R', 'B'],
    },
];

// const FILTER_SYMBOL: TransitionType = 0b0010_0001;
const FILTER_SYMBOL_0_1: TransitionType = 0b0000_0001;
// const FILTER_SYMBOL_UNDEFINED: TransitionType = 0b0010_0000;
const FILTER_DIR: TransitionType = 0b1100_0000;
pub const FILTER_STATE: TransitionType = 0b0001_1110;
const FILTER_ARRAY_ID: TransitionType = 0b0001_1111;
pub const TRANSITION_HOLD: TransitionType = DIRECTION_UNDEFINED; // | SYMBOL_UNDEFINED | STATE_HOLD;
pub const TRANSITION_BINARY_UNUSED: TransitionType = 0b0000_0000; // 0b1010_0001;
pub const TRANSITION_0RA: TransitionType = 0b1100_0010;
const SYMBOL_ZERO: TransitionType = 0b0000_0000;
const SYMBOL_ONE: TransitionType = 0b0000_0001;
// const SYMBOL_UNDEFINED: TransitionType = 0b0010_0000;
const DIRECTION_UNDEFINED: TransitionType = 0b1000_0000;
const TO_RIGHT: TransitionType = 0b1100_0000;
const TO_LEFT: TransitionType = 0b0100_0000;
pub const STATE_HOLD_SYM2: TransitionType = 0;

// TODO doc
// TODO change Undefined: Symbol only bit 0 (0 and 1), Direction holds undefined. Symbol and Direction are either both defined or undefined.
// TODO state could be limited to 8 values, then two bits (4,5) would be free for other uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionBinary {
    /// - symbol:     bits 0,5: write symbol, allows check with just 0b0000_0001 if undefined not relevant;
    ///   in combination with state the last 5 bits directly give the transition array id. \
    ///   value 0,1 or 2 for undefined, TODO old 3 for unused (for unused transitions in array)
    /// - direction:  bits 6, 7: direction \
    ///   value right 3, left 1 or 2 for undefined (because -2 = 0, not change in direction), calculated with -2 to get direction.
    /// - next state: bits 1-4: The value is naturally doubled for faster array id calculation \
    ///   value state or 0 for hold
    pub transition: TransitionType,
    /// transition as text for debugging
    #[cfg(debug_assertions)]
    pub text: [char; 3],
    // /// symbol 0,1 or 2 for undefined. 9 represents unused and is used to determine the number of states.
    // pub write_symbol: CellType,
    // /// direction +1, left -1 or 0 for undefined
    // pub direction: MoveType,
    // /// Next state of the machine, 0 represents halt signal
    // pub state_next: u8,
}

impl TransitionBinary {
    /// New transition from human readable format, e.g. 1RB, 1RZ or ---. \
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
    /// TODO make try_from and possibly new without Result
    pub fn try_new(transition_text: [u8; 3]) -> Result<Self, TransitionError> {
        // let symbol_char = transition_text[0];
        // let direction_char = transition_text[1];
        let state_char = transition_text[2];
        // let mut is_undefined = false;

        // Symbol
        let mut transition_bits = match transition_text[0] {
            b'0' | 0 => 0,
            b'1' | 1 => SYMBOL_ONE,
            // No undefined here
            b'-' => return Ok(TRANSITION_SYM2_HOLD), // SYMBOL_UNDEFINED,
            _ => return Err(TransitionError::InvalidSymbol(transition_text[0])),
        };

        match state_char {
            // Numeric 0 or char Z means Hold State
            // This does nothing, because it would be 0 which it already is.
            0 | b'Z' => {
                // if transition_bits == SYMBOL_UNDEFINED {
                //     return Ok(TRANSITION_SYM2_HOLD);
                // }
                // transition_bits |= STATE_HOLD; // is 0 anyway
            }
            // Numeric states (number from array)
            1..=9 => {
                if state_char > MAX_STATES as u8 {
                    return Err(TransitionError::StateOutOfRange(state_char));
                } else {
                    transition_bits |= (state_char as TransitionType) << 1;
                }
            }
            // Numeric states (char)
            b'1'..=b'9' => {
                let num_state = state_char - b'0';
                if num_state > MAX_STATES as u8 {
                    return Err(TransitionError::StateOutOfRange(num_state));
                } else {
                    transition_bits |= (num_state as TransitionType) << 1;
                }
            }
            // Alphabetic states A-Y (up to MAX_STATES)
            b'A'..=b'Y' => {
                let num_state = state_char - b'A' + 1;
                if num_state > MAX_STATES as u8 {
                    return Err(TransitionError::StateOutOfRange(num_state));
                }
                transition_bits |= (num_state as TransitionType) << 1;
            }
            // '-' is an error as it cannot be undefined if symbol is not undefined also.
            // If symbol is defined, 0 or 'Z' are expected as hold char.
            _ => return Err(TransitionError::InvalidStateChar(state_char)),
        }

        // direction
        match transition_text[1] {
            b'L' | 1 => transition_bits |= TO_LEFT,
            b'R' | 0 => transition_bits |= TO_RIGHT,
            // b'-' => transition_bits |= DIRECTION_UNDEFINED, // Undefined direction for non-halt transitions
            // '-' is an error as it cannot be undefined if symbol is not undefined also.
            _ => return Err(TransitionError::InvalidDirection(transition_text[1])),
        };

        #[cfg(debug_assertions)]
        {
            let mut t = Self {
                transition: transition_bits,
                text: ['_'; 3],
            };
            let tx = t.to_string().into_bytes();
            t.text = [tx[0] as char, tx[1] as char, tx[2] as char];
            Ok(t)
        }

        #[cfg(not(debug_assertions))]
        Ok(Self {
            transition: transition_bits,
        })
    }

    // / Create transition from array, where all values are numbers only, not chars.
    // / This is faster than new and used for the generator, but only used during batch creation.
    // / Discarded for DRY principle.
    // / [symbol, direction, state]
    //     pub fn new_int(transition_data: [u8; 3]) -> Self {
    //         const MAX: u8 = MAX_STATES as u8 + 1;
    //         // special hold in case of array
    //         if transition_data[2] == 0 {
    //             return TRANSITION_SYM2_HOLD;
    //         }
    //         // symbol
    //         let mut transition: TransitionType = match transition_data[0] {
    //             0 => 0,
    //             1 => SYMBOL_ONE,
    //             _ => SYMBOL_UNDEFINED,
    //         };
    //         // direction
    //         match transition_data[1] {
    //             1 => transition |= TO_LEFT,
    //             0 => transition |= TO_RIGHT,
    //             _ => {
    //                 panic!("Direction can only be 0 or 1. Hold is only defined in state, field [2].")
    //             }
    //         };
    //         // state
    //         match transition_data[2] {
    //             1..MAX => transition |= (transition_data[2] as TransitionType) << 1,
    //             _ => {
    //                 panic!(
    //                     "Unknown value for state: {}. Only 0-{MAX_STATES} are allowed.",
    //                     transition_data[2]
    //                 )
    //             }
    //         };
    //
    //         Self {
    //             transition,
    //             #[cfg(debug_assertions)]
    //             text: [
    //                 (transition_data[0] + b'0') as char,
    //                 if transition_data[1] == 1 { 'L' } else { 'R' },
    //                 (transition_data[2] + b'A' - 1) as char,
    //             ],
    //         }
    //     }

    // pub fn get_n_states(transitions: &[TransitionSymbol2]) -> usize {
    //     for (i, t) in transitions[4..].iter().enumerate().step_by(2) {
    //         if t.is_unused() {
    //             return i / 2 + 1;
    //         }
    //     }
    //     MAX_STATES
    // }

    pub fn is_dir_right(&self) -> bool {
        self.transition & FILTER_DIR == TO_RIGHT
    }

    pub fn is_dir_left(&self) -> bool {
        self.transition & FILTER_DIR == TO_LEFT
    }

    /// Returns the array_id for a 1D array, which is state * 2 + symbol.
    /// This only works for self referencing transitions, as the written symbol = tape read symbol.
    pub fn self_ref_array_id(&self) -> usize {
        (self.transition & FILTER_ARRAY_ID) as usize
    }

    /// This only works for self referencing transitions, as the written symbol = tape read symbol.
    pub fn self_ref_array_id_to_field_name(&self) -> String {
        MachineBinary::array_id_to_field_name(self.self_ref_array_id())
    }

    /// returns direction for left = -1, for right 1
    pub fn direction(&self) -> DirectionType {
        ((self.transition & FILTER_DIR) >> 6) as DirectionType - 2
    }

    /// returns direction with left = 1, right 3, undefined = 2
    pub fn direction_unmodified(&self) -> TransitionType {
        (self.transition & FILTER_DIR) >> 6
    }

    /// Returns the direction as char (L,R,-).
    pub fn direction_to_char(&self) -> char {
        match self.transition & FILTER_DIR {
            TO_LEFT => 'L',
            TO_RIGHT => 'R',
            _ => '-',
        }
    }

    pub fn state(&self) -> TransitionType {
        (self.transition & FILTER_STATE) >> 1
    }

    /// returns the state doubled as usize for array access
    pub fn state_x2(&self) -> usize {
        (self.transition & FILTER_STATE) as usize
    }

    /// Returns the state as char (A,B,C,...)
    pub fn state_to_char(&self) -> char {
        if self.transition & FILTER_STATE == 0 {
            'Z'
        } else {
            (((self.transition & FILTER_STATE) >> 1) as u8 + b'A' - 1) as char
        }
    }

    /// returns only 0 or 1, not undefined
    pub fn symbol(&self) -> TransitionType {
        self.transition & FILTER_SYMBOL_0_1
    }

    // /// returns 0, 1, or undefined
    // pub fn symbol_full(&self) -> TransitionType {
    //     self.transition & FILTER_SYMBOL
    // }

    /// returns only 0 or 1, not undefined
    pub fn symbol_usize(&self) -> usize {
        (self.transition & FILTER_SYMBOL_0_1) as usize
    }

    pub fn has_next_state_a(&self) -> bool {
        (self.transition & FILTER_STATE) == 0b0000_0000_0000_0010
    }

    pub fn is_halt(&self) -> bool {
        self.transition & FILTER_STATE == STATE_HOLD_SYM2
    }

    // pub fn is_self_ref(&self) -> bool {
    //     self.transition & FILTER_SELF_REF != 0
    // }

    pub fn is_symbol_one(&self) -> bool {
        self.transition & FILTER_SYMBOL_0_1 != 0
    }

    pub fn is_symbol_zero(&self) -> bool {
        self.transition & FILTER_SYMBOL_0_1 == 0
    }

    pub fn is_symbol_undefined(&self) -> bool {
        // Filter on direction is correct, as direction and symbol are always together defined or undefined.
        self.transition & FILTER_DIR == DIRECTION_UNDEFINED
    }

    pub fn is_unused(&self) -> bool {
        self.transition == TRANSITION_BINARY_UNUSED
    }

    // pub fn set_as_self_ref(&mut self) {
    //     self.transition |= FILTER_SELF_REF;
    // }

    /// This creates all transition permutations for one field, e.g. \
    /// 0RA, 1RA, 0LA, 1LA, --- for BB1 \
    /// The number can be calculated by (4 * n_states + 1), e.g. 21 for BB5. \
    /// Keep this order as 0RB is expected on pos 3.
    pub fn create_all_transition_permutations(n_states: usize) -> Vec<TransitionBinary> {
        let mut transitions = Vec::new();
        let mut tr: [u8; 3];

        // all to right
        for i in 1..=n_states {
            // tr as symbol, direction, next state
            tr = [0, 0, i as u8];
            transitions.push(TransitionBinary::try_new(tr).unwrap());
            // write symbol
            tr[0] = 1;
            transitions.push(TransitionBinary::try_new(tr).unwrap());
        }
        // all to left
        for i in 1..=n_states {
            // tr as symbol, direction, next state
            tr = [0, 1, i as u8];
            transitions.push(TransitionBinary::try_new(tr).unwrap());
            // write symbol
            tr[0] = 1;
            transitions.push(TransitionBinary::try_new(tr).unwrap());
        }
        // hold as last transition
        transitions.push(TRANSITION_SYM2_HOLD);

        transitions
    }
}

impl Default for TransitionBinary {
    fn default() -> Self {
        TRANSITION_SYM2_UNUSED
    }
}

// TODO Own Error Type or Simple Error crate
impl TryFrom<&str> for TransitionBinary {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 3 {
            return Err("Transition must have length of 3".to_string());
        }
        let r = TransitionBinary::try_new(value.as_bytes().try_into().unwrap());
        match r {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl From<&TransitionGeneric> for TransitionBinary {
    fn from(tg: &TransitionGeneric) -> Self {
        // quick check if undefined hold
        if tg.direction == 0 {
            return TRANSITION_SYM2_HOLD;
        }

        // symbol
        let mut t_new: i16 = match tg.symbol_write {
            0 => SYMBOL_ZERO,
            1 => SYMBOL_ONE,
            _ => panic!("Symbol incorrect, must not happen."),
        };

        // direction
        match tg.direction {
            -1 => t_new |= TO_LEFT,
            1 => t_new |= TO_RIGHT,
            _ => panic!("Direction incorrect, must not happen."),
        };

        // state
        if let 1..9 = tg.state_next {
            t_new |= (tg.state_next as i16) << 1;
        };
        // else 0 for hold

        #[cfg(not(debug_assertions))]
        return Self { transition: t_new };

        // add transition as chars to simplify debugging
        #[cfg(debug_assertions)]
        {
            let mut tx = Self {
                transition: t_new,
                // fill with dummy value
                text: ['_', '_', '_'],
            };
            // format with formatter
            let text = format!("{tx}");
            tx.text = text.chars().collect::<Vec<_>>().try_into().unwrap();

            tx
        }
    }
}

/// Displays the transition in standard format, e.g. 1RB
impl std::fmt::Display for TransitionBinary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.transition {
            TRANSITION_HOLD => write!(f, "---"),
            TRANSITION_BINARY_UNUSED => write!(f, "   "),
            _ => {
                let write_symbol = match self.transition & FILTER_SYMBOL_0_1 {
                    0 => '0',
                    SYMBOL_ONE => '1',
                    _ => '-',
                };
                let direction = match self.transition & FILTER_DIR {
                    TO_LEFT => 'L',
                    TO_RIGHT => 'R',
                    _ => return write!(f, "---"),
                };
                let next_state = self.state_to_char();
                write!(f, "{write_symbol}{direction}{next_state}")
            }
        }
    }
}

pub trait TransitionTypeExt {
    #[allow(dead_code)] // required for debugging
    fn to_binary_split_string(&self) -> String;
}

impl TransitionTypeExt for TransitionType {
    fn to_binary_split_string(&self) -> String {
        format!(
            "{:024b}_{:08b} {:08b}_{:024b}",
            self >> 40,
            (self >> 32) as u8,
            (self >> 24) as u8,
            (*self as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransitionError {
    InvalidSymbol(u8),
    InvalidDirection(u8),
    InvalidStateChar(u8),
    StateOutOfRange(u8),
}
impl std::error::Error for TransitionError {}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::InvalidSymbol(s) => {
                write!(f, "Invalid symbol: '{}'", *s as char)
            }
            TransitionError::InvalidDirection(d) => {
                write!(f, "Invalid direction: '{}'", *d as char)
            }
            TransitionError::InvalidStateChar(s) => {
                write!(f, "Invalid state character: '{}'", *s as char)
            }
            TransitionError::StateOutOfRange(s) => {
                write!(f, "State {s} out of range (max {MAX_STATES})")
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {

    use crate::machine_binary::MachineBinary;

    use super::*;

    #[test]
    fn transitions_for_A0_correctly_defined() {
        let mut t = MachineBinary::new_default(1);
        t.transitions[2] = TRANSITIONS_FOR_A0[0];
        t.transitions[3] = TRANSITIONS_FOR_A0[1];
        let tm_in = "0RB1RB";
        let tm_out = t.to_standard_tm_text_format();
        assert_eq!(tm_in, tm_out);
    }
}
