use crate::{
    config::{MAX_STATES_GENERIC, MAX_SYMBOLS_GENERIC},
    transition_generic::{
        CellType, StateType, TransitionGeneric, TransitionTableGenericArray, SYMBOL_UNDEFINED,
        TRANSITION_TABLE_GENERIC_DEFAULT,
    },
};

pub struct MachineDimensions {
    pub n_symbols: usize,
    pub n_states: usize,
}

/// The transition table.
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
        symbol: CellType,
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

/// Returns a transition table from Standard TM Text Format.
impl TryFrom<&str> for MachineGeneric {
    type Error = &'static str;

    fn try_from(tm_text_format: &str) -> Result<Self, Self::Error> {
        Self::try_from_standard_tm_text_format(tm_text_format)
    }
}

/// Displays the transitions in a multiline table.
impl std::fmt::Display for MachineGeneric {
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
    use crate::transition_generic::{TransitionGeneric, B, J};

    use super::*;

    #[test]
    fn machine_2x2_6_4() {
        // 2x2-6-4
        let text = "1RB1LB_1LA1RZ";
        let table = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RZ").unwrap();
        let transition_b1 = table.transition_for_state_symbol(B, 1);
        println!("{}", table);
        assert_eq!(check_value, transition_b1);
    }

    #[test]
    fn machine_2x6_e9866() {
        // 2x6-e9866
        let text = "1RB2LA1RZ5LB5LA4LB_1LA4RB3RB5LB1LB4RA";
        let table = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("5LB").unwrap();
        let transition_b3 = table.transition_for_state_symbol(B, 3);
        println!("{}", table);
        assert_eq!(check_value, transition_b3);
    }

    #[test]
    fn machine_4x3_e12068() {
        // 4x3-e12068
        let text = "1RB0LB1RD_2RC2LA0LA_1LB0LA0LA_1RA0RA1RZ";
        let table = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("2RC").unwrap();
        let transition_b0 = table.transition_for_state_symbol(B, 0);
        println!("{}", table);
        let tm_format = table.to_standard_tm_text_format();
        println!("{}", tm_format);
        assert_eq!(check_value, transition_b0);
        assert_eq!(text, tm_format);
    }

    #[test]
    fn machine_10x2_green() {
        // 10x2-Green
        let text = "1LB1RZ_0LC1LC_0LD0LC_1LE1RA_0LF0LE_1LG1RD_0LH0LG_1LI1RF_0LJ0LI_1RJ1RH";
        let table = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RJ").unwrap();
        let transition_j0 = table.transition_for_state_symbol(J, 0);
        println!("{}", table);
        let tm_format = table.to_standard_tm_text_format();
        println!("{}", tm_format);
        assert_eq!(check_value, transition_j0);
        assert_eq!(text, tm_format);

        // let ts = TransitionTableSymbol2::try_from(table);
        // println!("\nConvert to Transition Symbol2: {:?}", ts);
    }

    #[test]
    fn machine_10x10_random() {
        let text = "8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        _8LB1RZ0LC1LC0LD9LC1LE1RA0LF0LE_4LG1RD0LH0LG6LI1RF0LJ0LI1RJ1RH\
        ";
        let table = MachineGeneric::try_from_standard_tm_text_format(text).unwrap();
        let check_value = TransitionGeneric::try_from("1RJ").unwrap();
        let transition_j8 = table.transition_for_state_symbol(J, 8);
        println!("{}", table);
        let tm_format = table.to_standard_tm_text_format();
        println!("{}", tm_format);
        assert_eq!(check_value, transition_j8);
        assert_eq!(text, tm_format);

        // let ts = TransitionTableSymbol2::try_from(table);
        // println!("\n as Transition Symbol2: {:?}", ts);
    }
}
