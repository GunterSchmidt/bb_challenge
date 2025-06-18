use std::fmt::Display;

use crate::MAX_STATES;

pub const MAX_SYMBOLS: usize = 6;
pub const A: StateType = 1;
pub const B: StateType = 2;
pub const C: StateType = 3;
pub const D: StateType = 4;
pub const E: StateType = 5;
pub const F: StateType = 6;

/// tape cell storage size, can be any i/u available, e.g. u8, i32, with u16 seems fastest
/// CellType and MoveType affect Transition size, be carefull u16 makes nice 4 byte struct
pub type CellType = u16;
pub type MoveType = i8;
pub type StateType = u8;
pub type TransitionGenericArray = [[TransitionGeneric; MAX_SYMBOLS]; MAX_STATES + 1];
// pub type TransitionGenericArray1D = [TransitionGeneric; (MAX_STATES + 1) * 2];
// pub type TransitionGenericArray2D = [[TransitionGeneric; 2]; MAX_STATES + 1];

/// Default Unused for all transition fields. \
/// As this is an array a marker is required to identify the end of the used data, if n_states is not provided.
/// This uses less space than a separate field.
pub const TRANSITION_TABLE_GENERIC_DEFAULT: TransitionGenericArray = [[TransitionGeneric {
    symbol_write: SYMBOL_UNUSED,
    direction: 0,
    state_next: 0,
}; MAX_SYMBOLS];
    MAX_STATES + 1];
// pub const TRANSITION_ARRAY_1D_DEFAULT: TransitionGenericArray1D = [TransitionGeneric {
//     write_symbol: SYMBOL_UNUSED,
//     direction: 0,
//     state_next: 0,
// }; (MAX_STATES + 1) * 2];
// pub const TRANSITION_ARRAY_2D_UNUSED: TransitionGenericArray2D = [[TransitionGeneric {
//     write_symbol: 9,
//     direction: 0,
//     state_next: 0,
// }; 2]; MAX_STATES + 1];
pub const SYMBOL_UNDEFINED: CellType = u8::MAX as CellType;
pub const SYMBOL_UNUSED: CellType = u8::MAX as CellType - 1;
pub const DIR_RIGHT: MoveType = 1;
pub const DIR_LEFT: MoveType = -1;
pub const STATE_HOLD: u8 = 0;
pub const TRANSITION_HOLD: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNDEFINED,
    direction: 0,
    state_next: 0,
};
pub const TRANSITION_UNUSED: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNUSED,
    direction: 0,
    state_next: 0,
};

pub struct MachineDimensions {
    pub n_symbols: usize,
    pub n_states: usize,
}

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
    /// [symbol,direction,status]
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
            return TRANSITION_HOLD;
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
            _ => 0,
        };
        assert!(state_next <= MAX_STATES as u8);

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
        if value.as_bytes().len() != 3 {
            return Err("Transition must have length of 3");
        }
        Ok(TransitionGeneric::new(value.as_bytes().try_into().unwrap()))
    }
}

/// Displays the transition in Standard TM Text Format.
impl Display for TransitionGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.symbol_write == 9 || (self.state_next == 0 && self.symbol_write == 2) {
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
        write!(f, "{}{}{}", write_symbol, move_next, next_state)
    }
}

/// The transition table.
#[derive(Debug, Clone, Copy)]
pub struct TransitionTableGeneric {
    /// The transitions are stored in a two dimensional array with a dummy line, \
    /// where transition_table[1][2] represents the transition for A2 (state A, symbol 2).
    /// This is designed as an array for faster access in case it is used in a loop. Using a
    /// dummy line for state 0 allows to use the numerical state number (A=1) directly for field access.
    pub transition_table: TransitionGenericArray,
}

impl TransitionTableGeneric {
    /// Creates the transition table from the Standard TM Text Format \
    /// https://www.sligocki.com/2022/10/09/standard-tm-format.html
    pub fn from_standard_tm_text_format(transitions_text: &str) -> Result<Self, &'static str> {
        let mut transitions = TRANSITION_TABLE_GENERIC_DEFAULT;
        let transition_tuples: Vec<&str> = transitions_text.split('_').collect();
        let len_line = transition_tuples.first().unwrap().as_bytes().len();
        for (line, tuple) in transition_tuples.iter().enumerate() {
            // Check format
            if tuple.as_bytes().len() != len_line {
                return Err("Expected a format like '1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA'. The length of the separated transition lines is not identical.");
            }
            for (symbol, start) in (0..len_line).step_by(3).enumerate() {
                let transition = tuple.as_bytes()[start..start + 3].try_into().unwrap();
                transitions[line + 1][symbol] = TransitionGeneric::new(transition);
            }
        }

        Ok(Self {
            transition_table: transitions,
        })
    }

    pub fn to_standard_tm_text_format(&self) -> String {
        let mut transition_texts = Vec::new();
        let dim = self.dimensions_slow();
        for state_line in self.transition_table.iter().skip(1).take(dim.n_states) {
            let mut s = String::new();
            for transition in state_line.iter().take(dim.n_symbols) {
                s.push_str(format!("{transition}").as_str());
            }
            transition_texts.push(s);
        }

        transition_texts.join("_")
    }

    // Returns the transition for state (numeric A=1, B=2 etc.) and read symbol.
    pub fn transition_for_state_symbol(
        &self,
        state: StateType,
        symbol: CellType,
    ) -> TransitionGeneric {
        self.transition_table[state as usize][symbol as usize]
    }

    /// Returns the number of (states, symbols) used.
    /// Returns the highest used symbol, e.g. 1 for machines writing only 0 and 1.
    pub fn dimensions_slow(&self) -> MachineDimensions {
        let mut max_symbols = MAX_SYMBOLS;
        for (symbol, transition) in self.transition_table[1].iter().enumerate() {
            if transition.is_unused() {
                max_symbols = symbol;
                break;
            }
        }
        let mut n_states = MAX_STATES;
        for (line, transition_line) in self.transition_table.iter().skip(1).enumerate() {
            if transition_line[0].is_unused() {
                n_states = line;
                break;
            }
        }

        MachineDimensions {
            n_symbols: max_symbols,
            n_states,
        }
    }

    // Checks if this is a bb_challenge machine with only symbols 0 and 1.
    pub fn has_two_symbols(&self) -> bool {
        for (symbol, transition) in self.transition_table[1].iter().enumerate() {
            if transition.is_unused() {
                return symbol == 2;
            }
        }

        false
    }
}

// /// Creates a transition table from a string in format '1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA'.
// impl TryFrom<&str> for TransitionTableGeneric {
//     type Error = &'static str;
//
//     fn try_from(transitions_text: &str) -> Result<Self, Self::Error> {
//         let mut transitions = TRANSITION_ARRAY_DEFAULT;
//         let transition_pairs: Vec<&str> = transitions_text.split('_').collect();
//         for (i, t) in transition_pairs.iter().enumerate() {
//             // Check format
//             if t.as_bytes().len() != 6 {
//                 return Err("Expected a format like '1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA'");
//             }
//             transitions[i * 2] = TransitionGeneric::new(t.as_bytes()[0..3].try_into().unwrap());
//             transitions[i * 2 + 1] = TransitionGeneric::new(t.as_bytes()[0..3].try_into().unwrap());
//         }
//
//         Ok(Self { transitions })
//     }
// }

/// Displays the transitions in a multiline table.
impl Display for TransitionTableGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dim = self.dimensions_slow();
        let mut s = String::with_capacity(dim.n_symbols * 4 + 2 * (dim.n_states + 1));
        // write table header 0  1  2 etc.
        for symbol in 0..dim.n_symbols {
            s.push_str("   ");
            s.push((symbol as u8 + b'0') as char);
        }
        s.push('\n');
        // write table lines
        for (state_no, transition_line) in self
            .transition_table
            .iter()
            .enumerate()
            .skip(1)
            .take(dim.n_states)
        {
            // status as letter
            s.push(((state_no - 1) as u8 + b'A') as char);
            // transitions
            for transition in transition_line.iter().take(dim.n_symbols) {
                s.push(' ');
                s.push_str(&transition.to_string());
            }
            if state_no < dim.n_states {
                s.push('\n');
            }
        }
        write!(f, "{s}")
    }
}

// TODO Possible rewrite for u8 to print symbol, state as char (.to_char)
// pub trait U64Ext {
//     #[allow(dead_code)] // required for debugging
//     fn to_binary_split_string(&self) -> String;
// }
//
// impl U64Ext for u64 {
//     fn to_binary_split_string(&self) -> String {
//         format!(
//             "{:024b}_{:08b} {:08b}_{:024b}",
//             self >> 40,
//             (self >> 32) as u8,
//             (self >> 24) as u8,
//             (*self as u32) & 0b0000_0000_1111_1111_1111_1111_1111_1111,
//         )
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_2x2_6_4() {
        // 2x2-6-4
        let text = "1RB1LB_1LA1RZ";
        let table = TransitionTableGeneric::from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RZ").unwrap();
        let transition_b1 = table.transition_for_state_symbol(B, 1);
        println!("{}", table);
        assert_eq!(check_value, transition_b1);
    }

    #[test]
    fn test_machine_2x6_e9866() {
        // 2x6-e9866
        let text = "1RB2LA1RZ5LB5LA4LB_1LA4RB3RB5LB1LB4RA";
        let table = TransitionTableGeneric::from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("5LB").unwrap();
        let transition_b3 = table.transition_for_state_symbol(B, 3);
        println!("{}", table);
        assert_eq!(check_value, transition_b3);
    }

    #[test]
    fn test_machine_4x3_e12068() {
        // 4x3-e12068
        let text = "1RB0LB1RD_2RC2LA0LA_1LB0LA0LA_1RA0RA1RZ";
        let table = TransitionTableGeneric::from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("2RC").unwrap();
        let transition_b0 = table.transition_for_state_symbol(B, 0);
        println!("{}", table);
        let tm_format = table.to_standard_tm_text_format();
        println!("{}", tm_format);
        assert_eq!(check_value, transition_b0);
        assert_eq!(text, tm_format);
    }
}
