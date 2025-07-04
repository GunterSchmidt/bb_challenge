use std::fmt::Display;

use crate::{
    config::MAX_STATES,
    transition_generic::{CellType, StateType, TransitionGeneric, TransitionTableGeneric},
};

/// Number format to represent a transition (lower 8 bit used for state, symbol and direction).
/// Can be any type other than i8/u8 (potential extra info per transition). i16 seems fastest.
pub type TransitionType = i16;
/// Number format for direction which is either -1 or 1. Can be any iXX type, i16 seems fastest.
pub type DirectionType = i16;
/// Holds the transitions for one turing machine. \
/// This usually would be a table of 2 * n_states fields, having A0 A1 in the first line.
/// As state 0 is undefined and to avoid number shifting to access the data for each state, the line 0 is unused.
/// For faster access, the 2 field wide table is reduced to a single dimensional array, with access by state*2 + status,
/// e.g. C1 is field 3*2+1 = 7.
/// For performance reasons, this is an Array instead of a Vec.
pub type TransitionSym2Array1D = [TransitionSymbol2; (MAX_STATES + 1) * 2];
pub const TRANSITION_TABLE_SYM2_DEFAULT: TransitionSym2Array1D = [TransitionSymbol2 {
    transition: TRANSITION_UNUSED,
    #[cfg(debug_assertions)]
    text: ['_', '_', '_'],
}; (MAX_STATES + 1) * 2];
pub const TRANSITION_SYM2_UNUSED: TransitionSymbol2 = TransitionSymbol2 {
    transition: TRANSITION_UNUSED,
    #[cfg(debug_assertions)]
    text: ['_', '_', '_'],
};
// This is the undefined hold ('---'), where no last symbol is written.
pub const TRANSITION_SYM2_HOLD: TransitionSymbol2 = TransitionSymbol2 {
    transition: TRANSITION_HOLD,
    #[cfg(debug_assertions)]
    text: ['-', '-', '-'],
};
/// Initialize transition with A0 as start
pub const TRANSITION_SYM2_START: TransitionSymbol2 = TransitionSymbol2 {
    transition: TRANSITION_0RA,
    #[cfg(debug_assertions)]
    text: ['0', 'R', 'A'],
};
pub const TRANSITIONS_FOR_A0: [TransitionSymbol2; 2] = [
    TransitionSymbol2 {
        transition: 196,
        #[cfg(debug_assertions)]
        text: ['0', 'R', 'B'],
    },
    TransitionSymbol2 {
        transition: 197,
        #[cfg(debug_assertions)]
        text: ['1', 'R', 'B'],
    },
];

// const FILTER_SYMBOL: TransitionType = 0b0010_0001;
const FILTER_SYMBOL_0_1: TransitionType = 0b0000_0001;
// const FILTER_SYMBOL_UNDEFINED: TransitionType = 0b0010_0000;
const FILTER_DIR: TransitionType = 0b1100_0000;
pub(crate) const FILTER_STATE: TransitionType = 0b0001_1110;
const FILTER_ARRAY_ID: TransitionType = 0b0001_1111;
const FILTER_SELF_REF: TransitionType = 0b0000_0001_0000_0000;
const FILTER_TABLE_SELF_REF: TransitionType = 0b1000_0000;
const FILTER_TABLE_N_STATES: TransitionType = 0b0000_1111;
// TODO why?
pub const TRANSITION_HOLD: TransitionType = DIRECTION_UNDEFINED; // | SYMBOL_UNDEFINED | STATE_HOLD;
pub const TRANSITION_UNUSED: TransitionType = 0b0000_0000; // 0b1010_0001;
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
pub struct TransitionSymbol2 {
    /// - symbol:     bits 0,5: write symbol, allows check with just 0b0000_0001 if undefined not relevant;
    ///   in combination with state the last 5 bits directly give the transition array id. \
    ///   value 0,1 or 2 for undefined, TODO old 3 for unused (for unused transitions in array)
    /// - direction:  bits 6, 7: direction \
    ///   value right 3, left 1 or 2 for undefined, calculated with -2 to get direction.
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

impl TransitionSymbol2 {
    /// New transition from human readable format, e.g. 1RB, 1RZ or ---. \
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
    /// TODO make try_from and possibly new without Result
    pub fn new(transition_text: [u8; 3]) -> Result<Self, TransitionError> {
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

    /// Create transition from array, where all values are numbers only, not chars.
    /// This is faster than new and used for the generator, but only used during batch creation.
    /// Discarded for DRY principle.
    /// [symbol, direction, state]
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

    /// returns the array_id for a 1D array, which is state * 2 + symbol
    pub fn array_id(&self) -> usize {
        (self.transition & FILTER_ARRAY_ID) as usize
    }

    /// returns direction for left = -1, for right 1
    pub fn direction(&self) -> DirectionType {
        ((self.transition & FILTER_DIR) >> 6) as DirectionType - 2
    }

    /// returns direction for left = -1, for right 1
    pub fn direction_unmodified_todo(&self) -> TransitionType {
        self.transition & FILTER_DIR
    }

    pub fn state(&self) -> TransitionType {
        (self.transition & FILTER_STATE) >> 1
    }

    /// returns the state doubled as usize for array access
    pub fn state_x2(&self) -> usize {
        (self.transition & FILTER_STATE) as usize
    }

    // TODO all state conversions centralized
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

    pub fn is_hold(&self) -> bool {
        self.transition & FILTER_STATE == STATE_HOLD_SYM2
    }

    pub fn is_self_ref(&self) -> bool {
        self.transition & FILTER_SELF_REF != 0
    }

    pub fn is_symbol_one(&self) -> bool {
        self.transition & FILTER_SYMBOL_0_1 != 0
    }

    pub fn is_symbol_undefined(&self) -> bool {
        // Filter on direction is correct, as direction and symbol are always together defined or undefined.
        self.transition & FILTER_DIR == 0
    }

    pub fn is_unused(&self) -> bool {
        self.transition == TRANSITION_UNUSED
    }

    pub fn set_as_self_ref(&mut self) {
        self.transition |= FILTER_SELF_REF;
    }
}

impl Default for TransitionSymbol2 {
    fn default() -> Self {
        TRANSITION_SYM2_UNUSED
    }
}

// TODO Own Error Type or Simple Error crate
impl TryFrom<&str> for TransitionSymbol2 {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.as_bytes().len() != 3 {
            return Err("Transition must have length of 3".to_string());
        }
        let r = TransitionSymbol2::new(value.as_bytes().try_into().unwrap());
        match r {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl From<&TransitionGeneric> for TransitionSymbol2 {
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
            let text = format!("{}", tx);
            tx.text = text.chars().collect::<Vec<_>>().try_into().unwrap();

            tx
        }
    }
}

impl Display for TransitionSymbol2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.transition {
            TRANSITION_HOLD => write!(f, "---"),
            TRANSITION_UNUSED => write!(f, "   "),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionTableSymbol2 {
    /// Transition[0] is used for additional information \
    /// n_states: bit 0-4
    /// has_self_referencing_transition: bit 7
    pub transitions: TransitionSym2Array1D,
}

impl TransitionTableSymbol2 {
    /// Creates a new transition table and stores the n_states. \
    /// This is the correct fast approach.
    pub fn new_with_n_states(
        transitions: TransitionSym2Array1D,
        n_states: usize,
    ) -> TransitionTableSymbol2 {
        let mut table = Self { transitions };
        table.set_n_states(n_states);
        table
    }

    /// Creates a new transition table and stores the n_states. \
    /// This is the correct fast approach when the transition table gets filled later.
    pub fn new_default(n_states: usize) -> TransitionTableSymbol2 {
        let mut table = Self {
            transitions: TRANSITION_TABLE_SYM2_DEFAULT,
        };
        table.set_n_states(n_states);
        table
    }

    /// Creates a new transition table and identifies the used n_states. \
    /// This is slow and should be avoided.
    pub fn new_eval_n_states(transitions: TransitionSym2Array1D) -> TransitionTableSymbol2 {
        let mut table = Self { transitions };
        table.eval_set_n_states_slow();
        table
    }

    // new from transitions as String tuple
    pub fn from_string_tuple(transitions_as_str: &[(&str, &str)]) -> Self {
        // convert to TM standard
        let mut v = Vec::new();
        for t in transitions_as_str {
            v.push(format!("{}{}", t.0, t.1));
        }
        let s = v.join("_");
        let tg = TransitionTableGeneric::from_standard_tm_text_format(&s).expect("Wrong format");
        Self::try_from(tg).unwrap()
    }

    /// Creates the transition table from the Standard TM Text Format or returns an error. \
    /// https://www.sligocki.com/2022/10/09/standard-tm-format.html
    ///
    /// # Arguments
    /// * `standard_tm_text_format` - e.g. "1RB0LB_1LA0RA"
    ///
    /// # Examples
    /// ```
    /// # use bb_challenge::transition_symbol2::TransitionTableSymbol2;
    /// let tm_in = "1RB0LB_1LA0RA";
    /// let t = TransitionTableSymbol2::from_standard_tm_text_format(tm_in).unwrap();
    /// let tm_out = t.to_standard_tm_text_format();
    /// assert_eq!(tm_in, tm_out);
    /// ```
    pub fn from_standard_tm_text_format(
        standard_tm_text_format: &str,
    ) -> Result<Self, &'static str> {
        let tg = TransitionTableGeneric::from_standard_tm_text_format(standard_tm_text_format)?;
        let t = Self::try_from(tg)?;

        Ok(t)
    }

    /// Returns the transition table as standard TM Text format. Display returns this.
    pub fn to_standard_tm_text_format(&self) -> String {
        let mut transition_texts = Vec::new();
        // let n_states = self.n_states();
        for (i, transition) in self.transitions_used_eval().iter().enumerate().step_by(2) {
            let s = format!("{transition}{}", self.transition(i + 3));
            transition_texts.push(s);
        }

        transition_texts.join("_")
    }

    /// Returns the transition table as formatted table.
    pub fn to_table_string(&self, show_header_0_1: bool) -> String {
        let states = self.n_states();
        let mut s = String::new();
        // table header, state 0 and 1 could be stated
        // line begins with state as letter
        if show_header_0_1 {
            s.push_str("   0   1\n");
        }

        for (i, t) in self
            .transitions
            .iter()
            .skip(2)
            .step_by(2)
            .enumerate()
            .take(states)
        {
            s.push((i as u8 + b'A') as char);
            s.push(' ');
            // transition 0
            s.push_str(&t.to_string());
            s.push(' ');
            // transition 1
            s.push_str(&self.transitions[(i + 1) * 2 + 1].to_string());
            if i + 1 < states {
                s.push('\n');
            }
        }

        s
    }

    /// Returns the transition for the array id, which is state * 2 + symbol.
    pub fn transition(&self, array_id: usize) -> TransitionSymbol2 {
        self.transitions[array_id]
    }

    pub fn transition_start(&self) -> TransitionSymbol2 {
        self.transitions[2]
    }

    // Returns the transition for state (numeric A=1, B=2 etc.) and read symbol.
    pub fn transition_for_state_symbol(
        &self,
        state: StateType,
        symbol: CellType,
    ) -> TransitionSymbol2 {
        self.transitions[state as usize * 2 + symbol as usize]
    }

    pub fn transitions_all(&self) -> TransitionSym2Array1D {
        self.transitions
    }

    /// This is minimal slower than with provided n_states
    pub fn transitions_used_eval(&self) -> &[TransitionSymbol2] {
        let last = self.n_states() * 2 + 2;
        &self.transitions[2..last]
    }

    pub fn transitions_used(&self, n_states: usize) -> &[TransitionSymbol2] {
        &self.transitions[2..n_states * 2 + 2]
    }

    /// Returns the number of (states, symbols) used. \
    pub fn n_states(&self) -> usize {
        (self.transitions[0].transition & FILTER_TABLE_N_STATES) as usize
    }

    //     /// Returns the number of (states, symbols) used. \
    //     fn eval_n_states_slow(transitions: &TransitionSym2Array1D) -> usize {
    //         let mut n_states = MAX_STATES;
    //         for (line, transition) in transitions.iter().skip(4).step_by(2).enumerate() {
    //             if transition.is_unused() {
    //                 n_states = line + 1;
    //                 break;
    //             }
    //         }
    //
    //         n_states
    //     }

    /// Returns the number of (states, symbols) used. \
    fn eval_set_n_states_slow(&mut self) -> usize {
        for (line, transition) in self.transitions.iter().skip(4).step_by(2).enumerate() {
            if transition.is_unused() {
                self.set_n_states(line + 1);
                return line + 1;
            }
        }

        self.set_n_states(MAX_STATES);
        MAX_STATES
    }

    /// Sets the n_states in the first array element. Expects states not to be set, so only during initialization.
    fn set_n_states(&mut self, n_states: usize) {
        self.transitions[0].transition |= n_states as TransitionType;
    }

    pub fn has_self_referencing_transition(&self) -> bool {
        (self.transitions[0].transition & FILTER_TABLE_SELF_REF) != 0
    }

    pub fn eval_set_has_self_referencing_transition(&mut self) -> bool {
        for (id, t) in self.transitions_used_eval().iter().enumerate() {
            if t.array_id() == id + 2 {
                self.transitions[0].transition |= FILTER_TABLE_SELF_REF;
                return true;
            }
        }
        false
    }

    pub fn set_has_self_referencing_transition(&mut self) {
        self.transitions[0].transition |= FILTER_TABLE_SELF_REF;
    }

    pub fn clear_has_self_referencing_transition(&mut self) {
        self.transitions[0].transition &= !FILTER_TABLE_SELF_REF;
    }
}

impl Default for TransitionTableSymbol2 {
    fn default() -> Self {
        Self {
            transitions: TRANSITION_TABLE_SYM2_DEFAULT,
        }
    }
}

impl TryFrom<TransitionTableGeneric> for TransitionTableSymbol2 {
    type Error = &'static str;

    fn try_from(table: TransitionTableGeneric) -> Result<Self, Self::Error> {
        let dim = table.dimensions_slow();
        if dim.n_symbols != 2 {
            return Err("This transition format is only for transitions with symbols 0 and 1.");
        }
        // TODO allow 10 states
        if dim.n_states > MAX_STATES {
            return Err("This transition format is limited to MAX_STATES states.");
        }
        if dim.n_states > 7 {
            return Err("This transition format is limited to 7 states.");
        }
        let mut transitions = TRANSITION_TABLE_SYM2_DEFAULT;
        for (i, t) in table
            .transition_table
            .iter()
            .enumerate()
            .skip(1)
            .take(dim.n_states)
        {
            transitions[i * 2] = TransitionSymbol2::from(&t[0]);
            transitions[i * 2 + 1] = TransitionSymbol2::from(&t[1]);
            // transitions[i * 2] = (&t[0]).into();
        }
        transitions[0].transition |= dim.n_states as TransitionType;

        Ok(Self { transitions })
    }
}

// impl From<TransitionTableGeneric> for TransitionTableCompact {
//     fn from(table: TransitionTableGeneric) -> Self {
//         let mut transitions = TRANSITION_TABLE_COMPACT_1D_DEFAULT;
//         for (i, t) in table.transitions.iter().enumerate().skip(2) {
//             if t.is_unused() {
//                 break;
//             }
//             transitions[i] = t.into();
//         }
//
//         Self { transitions }
//     }
// }

impl Display for TransitionTableSymbol2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_standard_tm_text_format())
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

impl Display for TransitionError {
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
                write!(f, "State {} out of range (max {})", s, MAX_STATES)
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {

    use super::*;

    #[test]
    fn transitions_for_A0_correctly_defined() {
        let mut t = TransitionTableSymbol2::new_default(1);
        t.transitions[2] = TRANSITIONS_FOR_A0[0];
        t.transitions[3] = TRANSITIONS_FOR_A0[1];
        let tm_in = "0RB1RB";
        let tm_out = t.to_standard_tm_text_format();
        assert_eq!(tm_in, tm_out);
    }
}
