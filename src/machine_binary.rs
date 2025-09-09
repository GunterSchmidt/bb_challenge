//! The [MachineBinary] holds the transitions for one machine where the symbol can only be 0 or 1.
//! This is a single column array with an additional first line. The transition for state/symbol is
//! calculated by state * 2 + symbol with state A=1, so C1 would become 3*2+1 = transitions[7]. \
//! Transition[0] contains additional information, always the number of states used, see [MachineBinary],
//! therefore use either new or make sure the number of states are set.
//!
//! Use TryFrom to create a machine from Standard TM Text Format. \
//! A normalized ID can be calculated by calling calc_normalized_id, see [calc_normalized_id].

use std::{fmt::Display, u64};

use num_format::ToFormattedString;

use crate::{
    config::{IdNormalized, MAX_STATES},
    data_provider::enumerator::NUM_FIELDS,
    machine_generic::{MachineGeneric, NotableMachine, StateType, SymbolType},
    machine_info::MachineInfo,
    transition_binary::{TransitionBinary, TransitionType, TRANSITION_BINARY_UNUSED},
};
// use crate::{
//     data_provider::enumerator::create_all_transition_permutations,
//     transition_binary::{TransitionBinary, TransitionType, TRANSITION_UNUSED},
// };

/// Contains the transitions for one turing machine. \
/// This usually would be a table of 2 * n_states fields, having A0 A1 in the first line.
/// As state 0 is undefined and to avoid number shifting to access the data for each state, the line 0 is unused.
/// For faster access, the 2 field wide table is reduced to a single dimensional array, with access by state*2 + status,
/// e.g. C1 is field 3*2+1 = 7.
/// For performance reasons, this is an Array instead of a Vec.
pub type TransitionTableBinaryArray1D = [TransitionBinary; NUM_FIELDS];
pub const TRANSITION_TABLE_BINARY_DEFAULT: TransitionTableBinaryArray1D =
    [TRANSITION_BINARY_UNUSED; NUM_FIELDS];
const FILTER_TABLE_N_STATES: TransitionType = 0b0000_1111;
const FILTER_TABLE_SELF_REF: TransitionType = 0b1100_0000;
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
    // Use new only if the transition[0] already contains the n_states (usually only from enumerator).
    pub fn new(transitions: TransitionTableBinaryArray1D) -> Self {
        Self { transitions }
    }

    /// Creates a new machine and stores the n_states. \
    /// This is the correct fast approach when the number of states is known, but not in the machine data.
    pub fn new_with_n_states(transitions: TransitionTableBinaryArray1D, n_states: usize) -> Self {
        let mut machine = Self { transitions };
        machine.set_n_states(n_states);
        machine
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

    /// Calculates the id for forward rotating or backward rotating transitions. \
    /// This is an expensive operation and should only be used for display purposes.
    // TODO create normalized transition permutations array, so no calc is necessary, just cut for n_states.
    // Should be building up for the states, so it is not alphabetically sorted, e.g. all 0 first, but
    // ---, 0LA, 0RA, 1LA, 1RA, 0LB, 0RB, 1LB, 1RA, etc.
    // machine BB2 1RB---_1LA0LA would have the same id as BB3 1RB---_1LA0LA_------
    // Since the fields are rotated from field 0,1,2 etc. all machines before the second to last has a value
    // other than --- can be skipped. Is this better for enumeration?
    // Rotating backward all machines can be skipped if the first entry is --- or once the last entry is reached.
    // Need to think about it.
    pub fn normalized_id_calc(&self) -> IdNormalized {
        let n_states = self.n_states();
        let tr_permutations = TransitionBinary::create_all_transition_permutations(n_states);
        #[cfg(not(feature = "normalized_id_reversed"))]
        return Self::calc_normalized_id_forward(&self, &tr_permutations);
        #[cfg(feature = "normalized_id_reversed")]
        Self::calc_normalized_id_backward(&self, &tr_permutations)
    }

    /// Calculates the id from the given transitions
    pub fn calc_normalized_id_forward(
        table: &MachineBinary,
        tr_permutations: &[TransitionBinary],
    ) -> IdNormalized {
        let n_states = table.n_states();
        let mut id: IdNormalized = 0;
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

            id += (pos * tr_permutations.len().pow(n2 - (n2 - i as u32))) as IdNormalized;
        }

        id
    }

    pub fn calc_normalized_id_backward(
        table: &MachineBinary,
        tr_permutations: &[TransitionBinary],
    ) -> IdNormalized {
        let n_states = table.n_states();
        let mut id: IdNormalized = 0;
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

            id += (pos * tr_permutations.len().pow(n2 - i as u32 - 1)) as IdNormalized;
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

    fn try_from(mg: MachineGeneric) -> Result<Self, Self::Error> {
        let dim = mg.dimensions();
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
        for (i, t) in mg.transitions.iter().enumerate().skip(1).take(dim.n_states) {
            transitions[i * 2] = TransitionBinary::from(&t[0]);
            transitions[i * 2 + 1] = TransitionBinary::from(&t[1]);
            // transitions[i * 2] = (&t[0]).into();
        }
        transitions[0].transition |= dim.n_states as TransitionType;

        Ok(Self { transitions })
    }
}

impl From<&MachineInfo> for MachineBinary {
    fn from(mi: &MachineInfo) -> Self {
        Self {
            transitions: mi.machine().transitions,
        }
    }
}

// convert from Machine Binary to MachineGeneric, simple, but slow
impl From<MachineBinary> for MachineGeneric {
    fn from(mb: MachineBinary) -> Self {
        let tm = mb.to_standard_tm_text_format();
        MachineGeneric::try_from_standard_tm_text_format(&tm).unwrap()
    }
}

impl Display for MachineBinary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_standard_tm_text_format())
    }
}

/// This struct is used in DataProvider to allow an index id. \
/// To keep the size small, instead of Option<id> the u64::MAX is used to indicate not used.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MachineId {
    id: u64,
    machine: MachineBinary,
}

impl MachineId {
    pub fn new(id: u64, machine: MachineBinary) -> Self {
        Self { id, machine }
    }

    pub fn new_no_id(machine: MachineBinary) -> Self {
        Self {
            id: u64::MAX,
            machine,
        }
    }

    /// new from transitions as String tuple
    /// # Panics
    /// Panics if wrong format
    pub fn from_string_tuple(transitions_as_str: &[(&str, &str)]) -> Self {
        let m = MachineBinary::from_string_tuple(transitions_as_str);
        Self::from(&m)
    }

    /// Returns the id, instead of Option, the unused case is: u64::MAX
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the id, instead of Option, the unused case is: u64::MAX
    pub fn id_or_normalized_id(&self) -> IdNormalized {
        if self.has_id() {
            self.id as IdNormalized
        } else {
            self.machine.normalized_id_calc()
        }
    }

    pub fn id_as_option(&self) -> Option<u64> {
        if self.id == u64::MAX {
            None
        } else {
            Some(self.id)
        }
    }

    pub fn has_id(&self) -> bool {
        self.id != u64::MAX
    }

    pub fn machine(&self) -> &MachineBinary {
        &self.machine
    }

    pub fn machine_mut(&mut self) -> &mut MachineBinary {
        &mut self.machine
    }

    pub fn file_name(&self) -> String {
        if self.has_id() {
            format!(
                "BB{}_ID_{}_{}",
                self.machine.n_states(),
                self.id,
                self.to_standard_tm_text_format()
            )
        } else {
            format!(
                "BB{}_{}",
                self.machine.n_states(),
                self.to_standard_tm_text_format()
            )
        }
    }

    pub fn n_states(&self) -> usize {
        self.machine.n_states()
    }

    pub fn to_standard_tm_text_format(&self) -> String {
        self.machine.to_standard_tm_text_format()
    }
}

impl Default for MachineId {
    fn default() -> Self {
        Self {
            id: u64::MAX,
            machine: MachineBinary::default(),
        }
    }
}

impl From<&MachineBinary> for MachineId {
    fn from(mb: &MachineBinary) -> Self {
        Self {
            id: u64::MAX,
            machine: *mb,
        }
    }
}

impl From<&MachineInfo> for MachineId {
    fn from(mi: &MachineInfo) -> Self {
        Self {
            id: mi.id(),
            machine: mi.machine(),
        }
    }
}

// convert from Machine Binary to MachineGeneric, simple, but slow
impl From<MachineId> for MachineGeneric {
    fn from(m_id: MachineId) -> Self {
        let tm = m_id.to_standard_tm_text_format();
        let mut mg = MachineGeneric::try_from_standard_tm_text_format(&tm).unwrap();
        mg.id = m_id.id_as_option();

        mg
    }
}

impl TryFrom<MachineGeneric> for MachineId {
    type Error = &'static str;

    fn try_from(mg: MachineGeneric) -> Result<Self, Self::Error> {
        let m = MachineBinary::try_from(mg)?;
        if let Some(id) = mg.id {
            Ok(Self::new(id, m))
        } else {
            Ok(Self::new_no_id(m))
        }
    }
}

impl TryFrom<&str> for MachineId {
    type Error = &'static str;

    fn try_from(tm_text_format: &str) -> Result<Self, Self::Error> {
        let mg = MachineGeneric::try_from_standard_tm_text_format(tm_text_format)?;
        let m = MachineBinary::try_from(mg)?;

        Ok(MachineId::new_no_id(m))
    }
}

impl Display for MachineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = &crate::config::user_locale();
        write!(
            f,
            "ID: {} {}",
            self.id.to_formatted_string(locale),
            self.machine
        )
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

    pub fn machine_id(&self) -> MachineId {
        let m = self.machine();

        MachineId::new_no_id(m)
    }
}
