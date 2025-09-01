//! This is a Turing Machine. It is called Generic as it is working with different numbers of symbols and states. \
//! The limit is set to 10 symbols and 10 states, which should be sufficient for now. \
//! Currently this only serves as intermediate format to read machine data and convert it then to MachineBinary,
//! as the Busy Beaver Challenge only works with the symbols 0 and 1, which can be handled more efficiently.

use std::fmt::Display;

use crate::config::{MAX_STATES_GENERIC, MAX_SYMBOLS_GENERIC};

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
pub type SymbolType = u16;
pub type MoveType = i8;
pub type StateType = u8;

/// Undefined (the machine halts if it ever reaches an undefined transition), written as '---' for the transition.
pub const SYMBOL_UNDEFINED: SymbolType = u8::MAX as SymbolType;
/// Marker unused, as the data model uses an array with unused fields. Never exposed to user.
pub const SYMBOL_UNUSED: SymbolType = u8::MAX as SymbolType - 1;
pub const DIR_RIGHT: MoveType = 1;
pub const DIR_LEFT: MoveType = -1;
pub const DIR_UNDEFINED: MoveType = 0;
pub const STATE_HALT_GENERIC: StateType = 0;
pub const TRANSITION_HALT: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNDEFINED,
    direction: 0,
    state_next: 0,
};
pub const TRANSITION_GENERIC_UNUSED: TransitionGeneric = TransitionGeneric {
    symbol_write: SYMBOL_UNUSED,
    direction: 0,
    state_next: 0,
};

/// Return Type for dimensions()
pub struct MachineDimensions {
    pub n_symbols: usize,
    pub n_states: usize,
}

/// The Turing Machine, which is the transition table.
#[derive(Debug, Clone, Copy)]
pub struct MachineGeneric {
    /// The transitions are stored in a two dimensional array with a dummy line, \
    /// where transition_table\[1\]\[2\] represents the transition for A2 (state A, symbol 2).
    /// This is designed as an array for faster access in case it is used in a loop. Using a
    /// dummy line for state 0 allows to use the numerical state number (A=1) directly for field access.
    pub transitions: TransitionTableGenericArray,
}

impl MachineGeneric {
    /// Creates the transition table from the Standard TM Text Format \
    /// <https://www.sligocki.com/2022/10/09/standard-tm-format.html>
    pub fn try_from_standard_tm_text_format(transitions_text: &str) -> Result<Self, &'static str> {
        let mut transitions = TRANSITION_TABLE_GENERIC_DEFAULT;
        let transition_tuples: Vec<&str> = transitions_text.split('_').collect();
        if transition_tuples.len() > MAX_STATES_GENERIC {
            // println!("{:?}", transition_tuples);
            // println!("{}", transition_tuples.len());
            return Err("The number of table states exceeds the states set in MAX_STATES_GENERIC!");
        }
        let len_line = transition_tuples.first().unwrap().len();
        if len_line / 3 > MAX_SYMBOLS_GENERIC {
            return Err(
                "The number of table symbols exceeds the symbols set in MAX_SYMBOLS_GENERIC!",
            );
        }
        let mut max_symbol = 0;
        for (line, tuple) in transition_tuples.iter().enumerate() {
            // Check format
            if tuple.len() != len_line {
                return Err("Expected a format like '1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA'. The length of the separated transition lines is not identical.");
            }
            for (symbol, start) in (0..len_line).step_by(3).enumerate() {
                let transition = tuple.as_bytes()[start..start + 3].try_into().unwrap();
                transitions[line + 1][symbol] = TransitionGeneric::new(transition);
                if transitions[line + 1][symbol].symbol_write > max_symbol
                    && transitions[line + 1][symbol].symbol_write < SYMBOL_UNDEFINED
                {
                    max_symbol = transitions[line + 1][symbol].symbol_write;
                }
            }
        }

        // check if all references are available, e.g. 8LB requires als a table size 8.
        let t = Self { transitions };
        let dim = t.dimensions();
        if dim.n_symbols != max_symbol as usize + 1 {
            // This is not failsafe, as only line one is checked for completeness.
            // Should check all fields for unused, but seems overdone.
            eprintln!(
                "The max symbol used is {max_symbol}, but the table has symbol size {}.",
                dim.n_symbols
            );
            return Err("The max symbol used and the table symbol size do not match!");
        }

        Ok(t)
    }

    pub fn to_standard_tm_text_format(&self) -> String {
        let mut transition_texts = Vec::new();
        let dim = self.dimensions();
        for state_line in self.transitions.iter().skip(1).take(dim.n_states) {
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
        symbol: SymbolType,
    ) -> TransitionGeneric {
        self.transitions[state as usize][symbol as usize]
    }

    /// Returns the number of (states, symbols) used. Symbol is the highest used symbol, e.g. 1 for machines writing only 0 and 1. \
    /// As this is evaluating the dimensions in a loop, this is comparatively slow and should not be used in extensive loops.
    pub fn dimensions(&self) -> MachineDimensions {
        let mut max_symbols = MAX_SYMBOLS_GENERIC;
        for (symbol, transition) in self.transitions[1].iter().enumerate() {
            if transition.is_unused() {
                max_symbols = symbol;
                break;
            }
        }
        let mut n_states = MAX_STATES_GENERIC;
        for (line, transition_line) in self.transitions.iter().skip(1).enumerate() {
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
        for (symbol, transition) in self.transitions[1].iter().enumerate() {
            if transition.is_unused() {
                return symbol == 2;
            }
        }

        false
    }
}

/// Returns a transition table from Standard TM Text Format.
impl TryFrom<&str> for MachineGeneric {
    type Error = &'static str;

    fn try_from(tm_text_format: &str) -> Result<Self, Self::Error> {
        Self::try_from_standard_tm_text_format(tm_text_format)
    }
}

/// Displays the transitions in a multiline table.
impl Display for MachineGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dim = self.dimensions();
        let mut s = String::with_capacity(dim.n_symbols * 4 + 2 * (dim.n_states + 1));
        // write table header 0  1  2 etc.
        for symbol in 0..dim.n_symbols {
            s.push_str("   ");
            s.push((symbol as u8 + b'0') as char);
        }
        s.push('\n');
        // write table lines
        for (state_no, transition_line) in self
            .transitions
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

/// This generic transition can handle Turing machines, which write symbols up to [MAX_SYMBOLS_GENERIC]. \
/// It is designed to be a human understandable format and used during parsing. \
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionGeneric {
    /// symbol 0,1,2.. or 255 for undefined. 254 represents unused and is used to determine the number of states.
    pub symbol_write: SymbolType,
    /// direction +1, left -1 or 0 for undefined
    pub direction: MoveType,
    /// Next state of the machine, 0 represents halt signal, 1 = A, 2 = B, 3 = C ...
    pub state_next: StateType,
}

impl TransitionGeneric {
    /// New transition from different formats,
    /// - human readable: 1RB, 1RZ or --- used in bb_challenge, which is String.as_bytes[0..3]. \
    /// - bb_challenge file format (https://bbchallenge.org/method#format): 1,R,2
    ///
    /// \[symbol,direction,status\] with first char the symbol to write on the tape, can be 0, 1 or any other char as undefined. \
    /// The distinction between 0,1 and undefined is relevant in the last transition. \
    /// 0,1 will write and hold in the last transition, undefined will only hold. \
    /// This results in a different 'number of ones' count. \
    /// BBChallenge uses both notations. \
    /// Second char is L or R for direction or any other for undefined. \
    /// L,R or undefined is irrelevant in last transition, in any other step it must be defined. \
    /// Third char is next state, it can be denoted as number 1-9, char 1-9, or char A-Y. 0 or Z represent halt. \
    /// This is the main halt condition. Numbers are used for the downloadable seeds.
    pub fn new(transition: [u8; 3]) -> Self {
        assert!(transition.len() == 3);
        // special hold in case of array
        if transition[2] == 0 {
            return TRANSITION_HALT;
        }
        let write_symbol = match transition[0] {
            0..=9 => transition[0] as SymbolType,
            b'0'..=b'9' => (transition[0] - b'0') as SymbolType,
            _ => SYMBOL_UNDEFINED,
        };
        let direction = match transition[1] {
            1 | b'L' => DIR_LEFT,
            0 | b'R' => DIR_RIGHT,
            _ => DIR_UNDEFINED,
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
        TRANSITION_GENERIC_UNUSED
    }
}

impl TryFrom<&str> for TransitionGeneric {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 3 {
            return Err(
                "Transition must consist of exactly three characters: Symbol Direction State",
            );
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

/// Some notable machines
/// Builds certain default machines which may be used for testing.
/// SA: <https://www.scottaaronson.com/papers/bb.pdf>#[derive(Debug)]
pub enum NotableMachine {
    BB2MaxAronson,
    BB3MaxAronson,
    BB3Max,
    BB4Max,
    BB5Max,
    /// https://bbchallenge.org/story#bb5
    BB5Steps105,
    BB3Rado,
    EndlessSimple,
}

impl NotableMachine {
    pub fn machine(&self) -> MachineGeneric {
        let transitions_text = match self {
            NotableMachine::BB3Rado => "1LB1RC_1RA1LB_1RB1LZ",
            NotableMachine::BB3Max => "1RB---_1LB0RC_1LC1LA",
            NotableMachine::BB4Max => "1RB1LB_1LA0LC_---1LD_1RD0RA",
            NotableMachine::BB5Max => "1RB1LC_1RC1RB_1RD0LE_1LA1LD_---0LA",
            NotableMachine::BB2MaxAronson => "1RB1LB_1LA---",
            NotableMachine::BB3MaxAronson => "1LB---_1RB0LC_1RC1RA",
            // other older ones, check if useful
            // //endless,no halt
            // "BB4_28051367" => "1LB1RC_0LC0LD_0RD0LA_1RA0RA",
            // //endless
            // "BB3_SINUS" => "1RC0LB_1LA---_0LA0RA",
            // //wrong halt count
            // "BB3_TEST" => "1RB0LB_1LC1RB_---1LA",
            NotableMachine::BB5Steps105 => "1RB1LC_0LB1LA_1RD1LB_1RE0RD_0RA---",
            NotableMachine::EndlessSimple => "0RA---",
        };

        MachineGeneric::try_from_standard_tm_text_format(transitions_text).unwrap()
    }
}

impl TryFrom<&str> for NotableMachine {
    type Error = &'static str;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        // normalize string
        let s = name.replace("_", "").to_ascii_lowercase();

        let nm = match s.as_str() {
            "bb3max" => NotableMachine::BB3Max,
            "bb4max" => NotableMachine::BB4Max,
            "bb5max" => NotableMachine::BB5Max,
            _ => return Err("Not a valid machine name."),
        };

        Ok(nm)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn try_from_name() {
        let text = "BB3_MAX";
        let n = NotableMachine::try_from(text);
        assert!(n.is_ok());
    }

    #[test]
    fn machine_2x2_6_4() {
        // 2x2-6-4
        let text = "1RB1LB_1LA1RZ";
        let machine = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RZ").unwrap();
        let transition_b1 = machine.transition_for_state_symbol(B, 1);
        println!("{text}\n{}", machine);
        assert_eq!(check_value, transition_b1);
        let tm_format = machine.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }

    #[test]
    fn machine_2x6_e9866() {
        // 2x6-e9866
        let text = "1RB2LA1RZ5LB5LA4LB_1LA4RB3RB5LB1LB4RA";
        let machine = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("5LB").unwrap();
        let transition_b3 = machine.transition_for_state_symbol(B, 3);
        println!("{text}\n{}", machine);
        assert_eq!(check_value, transition_b3);
        let tm_format = machine.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }

    #[test]
    fn machine_4x3_e12068() {
        // 4x3-e12068
        let text = "1RB0LB1RD_2RC2LA0LA_1LB0LA0LA_1RA0RA1RZ";
        let machine = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("2RC").unwrap();
        let transition_b0 = machine.transition_for_state_symbol(B, 0);
        println!("{text}\n{}", machine);
        assert_eq!(check_value, transition_b0);
        let tm_format = machine.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }

    #[test]
    fn machine_10x2_green() {
        // 10x2-Green
        let text = "1LB1RZ_0LC1LC_0LD0LC_1LE1RA_0LF0LE_1LG1RD_0LH0LG_1LI1RF_0LJ0LI_1RJ1RH";
        let machine = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RJ").unwrap();
        let transition_j0 = machine.transition_for_state_symbol(J, 0);
        println!("{text}\n{}", machine);
        assert_eq!(check_value, transition_j0);
        let tm_format = machine.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }

    #[test]
    fn machine_10x10_random() {
        let text = "8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        ";
        let machine = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RJ").unwrap();
        let transition_j8 = machine.transition_for_state_symbol(J, 8);
        println!("{text}\n{}", machine);
        assert_eq!(check_value, transition_j8);
        let tm_format = machine.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }
}
