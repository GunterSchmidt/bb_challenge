use crate::{
    config::{MAX_STATES, NUM_FIELDS},
    machine_generic::{MachineGeneric, NotableMachine, StateType, SymbolType},
    transition_binary::{TransitionBinary, TransitionType, TRANSITION_BINARY_UNUSED},
};
// use crate::{
//     data_provider::generator::create_all_transition_permutations,
//     transition_binary::{TransitionBinary, TransitionType, TRANSITION_UNUSED},
// };

/// Holds the transitions for one turing machine. \
/// This usually would be a table of 2 * n_states fields, having A0 A1 in the first line.
/// As state 0 is undefined and to avoid number shifting to access the data for each state, the line 0 is unused.
/// For faster access, the 2 field wide table is reduced to a single dimensional array, with access by state*2 + status,
/// e.g. C1 is field 3*2+1 = 7.
/// For performance reasons, this is an Array instead of a Vec.
pub type TransitionTableBinaryArray1D = [TransitionBinary; NUM_FIELDS];
pub const TRANSITION_TABLE_BINARY_DEFAULT: TransitionTableBinaryArray1D = [TransitionBinary {
    transition: TRANSITION_BINARY_UNUSED,
    #[cfg(debug_assertions)]
    text: ['_', '_', '_'],
}; NUM_FIELDS];
const FILTER_TABLE_N_STATES: TransitionType = 0b0000_1111;
const FILTER_TABLE_SELF_REF: TransitionType = 0b1100_0000;
// const FILTER_SELF_REF: TransitionType = 0b0000_0001_0000_0000;
const SELF_REF_NOT_CHECKED: TransitionType = 0b0000_0000;
const SELF_REF_SET_TRUE: TransitionType = 0b1000_0000;
const SELF_REF_SET_FALSE: TransitionType = 0b0100_0000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachineBinary {
    /// Transition\[0\] is used for additional information \
    /// n_states: bits 0-4: Always set with new() variants
    /// has_self_referencing_transition: bits 76: 00: not checked, 01: has none, 10: has some
    pub transitions: TransitionTableBinaryArray1D,
}

impl MachineBinary {
    /// Creates a new transition table and stores the n_states. \
    /// This is the correct fast approach.
    pub fn new_with_n_states(
        transitions: TransitionTableBinaryArray1D,
        n_states: usize,
    ) -> MachineBinary {
        let mut table = Self { transitions };
        table.set_n_states(n_states);
        table
    }

    /// Creates a new transition table and stores the n_states. \
    /// This is the correct fast approach when the transition table gets filled later.
    pub fn new_default(n_states: usize) -> MachineBinary {
        let mut table = Self {
            transitions: TRANSITION_TABLE_BINARY_DEFAULT,
        };
        table.set_n_states(n_states);
        table
    }

    /// Creates a new transition table and identifies the used n_states. \
    /// This is slow and should be avoided.
    pub fn new_eval_n_states(transitions: TransitionTableBinaryArray1D) -> MachineBinary {
        let mut table = Self { transitions };
        table.eval_set_n_states_slow();
        table
    }

    /// new from transitions as String tuple
    /// # Panics
    /// Panics if wrong format
    pub fn from_string_tuple(transitions_as_str: &[(&str, &str)]) -> Self {
        // convert to TM standard
        let mut v = Vec::new();
        for t in transitions_as_str {
            v.push(format!("{}{}", t.0, t.1));
        }
        let s = v.join("_");
        let tg = MachineGeneric::try_from_standard_tm_text_format(&s).expect("Wrong format");
        Self::try_from(tg).unwrap()
    }

    /// Creates the transition table from the Standard TM Text Format or returns an error. \
    /// <https://www.sligocki.com/2022/10/09/standard-tm-format.html>
    ///
    /// # Arguments
    /// * `standard_tm_text_format` - e.g. "1RB0LB_1LA0RA"
    ///
    /// # Examples
    /// ```
    /// # use bb_challenge::transition_symbol2::TransitionTableSymbol2;
    /// let tm_in = "1RB0LB_1LA0RA";
    /// let t = TransitionTableSymbol2::try_from_standard_tm_text_format(tm_in).unwrap();
    /// let tm_out = t.to_standard_tm_text_format();
    /// assert_eq!(tm_in, tm_out);
    /// ```
    pub fn try_from_standard_tm_text_format(
        standard_tm_text_format: &str,
    ) -> Result<Self, &'static str> {
        let tg = MachineGeneric::try_from_standard_tm_text_format(standard_tm_text_format)?;
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

    /// Returns the transition table as formatted table (for print output).
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

    /// Returns the transition table as formatted table (for print output).
    pub fn to_table_html_string(&self, show_header_0_1: bool) -> String {
        let states = self.n_states();
        let mut s = String::from("<table>\n");
        // table header, state 0 and 1 could be stated
        // line begins with state as letter
        if show_header_0_1 {
            s.push_str("  <tr>\n");
            s.push_str("    <th></th>\n");
            s.push_str("    <th>0</th>\n");
            s.push_str("    <th>1</th>\n");
            s.push_str("  </tr>\n");
        }

        // rows
        for (i, t) in self
            .transitions
            .iter()
            .skip(2)
            .step_by(2)
            .enumerate()
            .take(states)
        {
            s.push_str("  <tr>\n");
            s.push_str("    <td>");
            s.push((i as u8 + b'A') as char);
            s.push_str("</td>\n");
            // transition 0
            s.push_str("    <td>");
            s.push_str(&t.to_string());
            s.push_str("</td>\n");
            // transition 1
            s.push_str("    <td>");
            s.push_str(&self.transitions[(i + 1) * 2 + 1].to_string());
            s.push_str("</td>\n");
            s.push_str("  </tr>\n");
        }
        s.push_str("</table>\n");

        s
    }

    pub fn last_used_field_id_in_array(&self) -> usize {
        self.n_states() * 2 + 1
    }

    /// Returns a new transition table with the order of the elements reversed,
    /// like the transitions would have been build starting from last field.
    /// For BB5 A0 is swapped with E1, A1 with E0 etc.
    pub fn reversed(&self) -> Self {
        let mut rev = *self;
        // add plus two to adjust for empty fields
        let n = self.n_states();
        let last = n * 2 + 3;
        // loop over half of the elements
        for i in 2..2 + n {
            rev.transitions.swap(i, last - i);
        }

        rev
    }

    /// Returns the transition for the array id, which is state * 2 + symbol. A0 = 2.
    pub fn transition(&self, array_id: usize) -> TransitionBinary {
        self.transitions[array_id]
    }

    pub fn transition_start(&self) -> TransitionBinary {
        self.transitions[2]
    }

    // Returns the transition for state (numeric A=1, B=2 etc.) and read symbol.
    pub fn transition_for_state_symbol(
        &self,
        state: StateType,
        symbol: SymbolType,
    ) -> TransitionBinary {
        self.transitions[state as usize * 2 + symbol as usize]
    }

    pub fn transitions_all(&self) -> TransitionTableBinaryArray1D {
        self.transitions
    }

    /// This is minimal slower than with provided n_states
    pub fn transitions_used_eval(&self) -> &[TransitionBinary] {
        let last = self.n_states() * 2 + 2;
        &self.transitions[2..last]
    }

    /// Returns the used section of the transition table, which is from 2..n_states * 2 + 2.
    pub fn transitions_used(&self, n_states: usize) -> &[TransitionBinary] {
        &self.transitions[2..n_states * 2 + 2]
    }

    /// Calculates the id for forward rotating or backward rotating transitions.
    pub fn calc_id(&self, as_forward: bool) -> u64 {
        let n_states = self.n_states();
        let tr_permutations = TransitionBinary::create_all_transition_permutations(n_states);
        if as_forward {
            Self::calc_id_forward(&self, &tr_permutations)
        } else {
            Self::calc_id_backward(&self, &tr_permutations)
        }
    }

    /// Calculates the id from the given transitions
    pub fn calc_id_forward(table: &MachineBinary, tr_permutations: &[TransitionBinary]) -> u64 {
        let n_states = table.n_states();
        let mut id: u64 = 0;
        let n2 = n_states as u32 * 2;
        let last_pos = tr_permutations.len() - 1;

        for (i, tr) in table.transitions_used(n_states).iter().enumerate() {
            // println!("{i}: {tr}");
            let pos = if tr.is_halt() {
                last_pos
            } else {
                tr_permutations
                    .iter()
                    .position(|p| p.transition == tr.transition)
                    .unwrap()
            };

            id += (pos * tr_permutations.len().pow(n2 - (n2 - i as u32))) as u64;
        }

        id
    }

    pub fn calc_id_backward(table: &MachineBinary, tr_permutations: &[TransitionBinary]) -> u64 {
        let n_states = table.n_states();
        let mut id: u64 = 0;
        let n2 = n_states as u32 * 2;
        let last_pos = tr_permutations.len() - 1;

        for (i, tr) in table.transitions_used(n_states).iter().enumerate() {
            // println!("{i}: {tr}");
            let pos = if tr.is_halt() {
                last_pos
            } else {
                tr_permutations
                    .iter()
                    .position(|p| p.transition == tr.transition)
                    .unwrap()
            };

            id += (pos * tr_permutations.len().pow(n2 - i as u32 - 1)) as u64;
        }

        id
    }

    /// Returns the number of (states, symbols) used. \
    #[inline]
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

    pub fn get_self_referencing_transitions(&self) -> Vec<TransitionBinary> {
        let mut v = Vec::new();
        for (id, t) in self.transitions_used_eval().iter().enumerate() {
            if t.self_ref_array_id() == id + 2 {
                v.push(*t);
            }
        }

        v
    }

    /// Returns true if at least one self-referencing transition exists (D1 1LD). \
    /// Slightly slower then [has_self_referencing_transition_store_result] if called repeatedly.
    pub fn has_self_referencing_transition(&self) -> bool {
        match self.transitions[0].transition & FILTER_TABLE_SELF_REF {
            SELF_REF_SET_TRUE => true,
            SELF_REF_SET_FALSE => false,
            SELF_REF_NOT_CHECKED => {
                for (id, t) in self.transitions_used_eval().iter().enumerate() {
                    if t.self_ref_array_id() == id + 2 {
                        return true;
                    }
                }
                false
            }
            _ => panic!("Logic error: Self Ref value not allowed"),
        }
    }

    /// Checks and returns if this table has at least one self referencing transition. \
    /// Stores the result internally and is much faster on second check.
    pub fn has_self_referencing_transition_store_result(&mut self) -> bool {
        match self.transitions[0].transition & FILTER_TABLE_SELF_REF {
            SELF_REF_SET_TRUE => true,
            SELF_REF_SET_FALSE => false,
            SELF_REF_NOT_CHECKED => {
                for (id, t) in self.transitions_used_eval().iter().enumerate() {
                    if t.self_ref_array_id() == id + 2 {
                        self.transitions[0].transition |= SELF_REF_SET_TRUE;
                        return true;
                    }
                }
                self.transitions[0].transition |= SELF_REF_SET_FALSE;
                false
            }
            _ => panic!("Logic error: Self Ref value not allowed"),
        }
    }

    pub fn clear_has_self_referencing_transition(&mut self) {
        self.transitions[0].transition &= !FILTER_TABLE_SELF_REF;
    }

    // Returns the machine table field name from the transition array id in an 1D-array, e.g. 2 -> A0.
    pub fn array_id_to_field_name(arr_id: usize) -> String {
        let state = ((arr_id / 2) as u8 + b'A' - 1) as char;
        let symbol = ((arr_id & 1) as u8 + b'0') as char;
        format!("{state}{symbol}")
    }
}

impl Default for MachineBinary {
    fn default() -> Self {
        Self {
            transitions: TRANSITION_TABLE_BINARY_DEFAULT,
        }
    }
}

/// Creates the transition table from the Standard TM Text Format or returns an error. \
/// <https://www.sligocki.com/2022/10/09/standard-tm-format.html>
///
/// # Arguments
/// * `standard_tm_text_format` - e.g. "1RB0LB_1LA0RA"
///
/// # Examples
/// ```
/// # use bb_challenge::transition_symbol2::TransitionTableSymbol2;
/// let tm_in = "1RB0LB_1LA0RA";
/// let t = TransitionTableSymbol2::try_from(tm_in).unwrap();
/// let tm_out = t.to_standard_tm_text_format();
/// assert_eq!(tm_in, tm_out);
/// ```
impl TryFrom<&str> for MachineBinary {
    type Error = &'static str;

    fn try_from(tm_text_format: &str) -> Result<Self, Self::Error> {
        let tg = MachineGeneric::try_from_standard_tm_text_format(tm_text_format)?;
        let t = Self::try_from(tg)?;

        Ok(t)
    }
}

impl TryFrom<MachineGeneric> for MachineBinary {
    type Error = &'static str;

    fn try_from(table: MachineGeneric) -> Result<Self, Self::Error> {
        let dim = table.dimensions();
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
        let mut transitions = TRANSITION_TABLE_BINARY_DEFAULT;
        for (i, t) in table
            .transitions
            .iter()
            .enumerate()
            .skip(1)
            .take(dim.n_states)
        {
            transitions[i * 2] = TransitionBinary::from(&t[0]);
            transitions[i * 2 + 1] = TransitionBinary::from(&t[1]);
            // transitions[i * 2] = (&t[0]).into();
        }
        transitions[0].transition |= dim.n_states as TransitionType;

        Ok(Self { transitions })
    }
}

impl std::fmt::Display for MachineBinary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_standard_tm_text_format())
    }
}

/// Some notable machines
/// Builds certain default machines which may be used for testing.
/// SA: <https://www.scottaaronson.com/papers/bb.pdf>#[derive(Debug)]
pub enum NotableMachineBinary {
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

impl NotableMachineBinary {
    pub fn machine(&self) -> MachineBinary {
        let n = match self {
            NotableMachineBinary::BB3Rado => NotableMachine::BB3Rado,
            NotableMachineBinary::BB3Max => NotableMachine::BB3Max,
            NotableMachineBinary::BB4Max => NotableMachine::BB4Max,
            NotableMachineBinary::BB5Max => NotableMachine::BB5Max,
            NotableMachineBinary::BB2MaxAronson => NotableMachine::BB2MaxAronson,
            NotableMachineBinary::BB3MaxAronson => NotableMachine::BB3MaxAronson,
            // other older ones, check if useful
            // //endless,no halt
            // "BB4_28051367" => "1LB1RC_0LC0LD_0RD0LA_1RA0RA",
            // //endless
            // "BB3_SINUS" => "1RC0LB_1LA---_0LA0RA",
            // //wrong halt count
            // "BB3_TEST" => "1RB0LB_1LC1RB_---1LA",
            NotableMachineBinary::BB5Steps105 => NotableMachine::BB5Steps105,
            NotableMachineBinary::EndlessSimple => NotableMachine::EndlessSimple,
        };
        let mg = n.machine();

        //         let transitions_text = match self {
        //             NotableMachineBinary::BB3Rado => "1LB1RC_1RA1LB_1RB1LZ",
        //             NotableMachineBinary::BB3Max => "1RB---_1LB0RC_1LC1LA",
        //             NotableMachineBinary::BB4Max => "1RB1LB_1LA0LC_---1LD_1RD0RA",
        //             NotableMachineBinary::BB5Max => "1RB1LC_1RC1RB_1RD0LE_1LA1LD_---0LA",
        //             NotableMachineBinary::BB2MaxAronson => "1RB1LB_1LA---",
        //             NotableMachineBinary::BB3MaxAronson => "1LB---_1RB0LC_1RC1RA",
        //             // other older ones, check if useful
        //             // //endless,no halt
        //             // "BB4_28051367" => "1LB1RC_0LC0LD_0RD0LA_1RA0RA",
        //             // //endless
        //             // "BB3_SINUS" => "1RC0LB_1LA---_0LA0RA",
        //             // //wrong halt count
        //             // "BB3_TEST" => "1RB0LB_1LC1RB_---1LA",
        //             NotableMachineBinary::BB5Steps105 => "1RB1LC_0LB1LA_1RD1LB_1RE0RD_0RA---",
        //             NotableMachineBinary::EndlessSimple => "0RA---",
        //         };
        //

        MachineBinary::try_from(mg).unwrap()
    }
}
