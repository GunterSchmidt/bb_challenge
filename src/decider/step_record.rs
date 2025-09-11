use crate::transition_binary::DirectionType;

/// Record of every step to identify cycles.
#[derive(Debug)]
pub struct StepRecordU128 {
    /// Table field which holds the current transition, basically the state and current symbol which lead to this transition. \
    /// Allows quick compare of symbol & state in one step.
    pub for_field_id: usize,
    /// Direction of the current step; can be used to calculate -1 Left, 1 Right.
    pub direction: DirectionType,
    /// tape before the current transition was executed
    pub tape_before: u128,
    #[cfg(all(debug_assertions, feature = "debug_cycler"))]
    #[allow(dead_code)]
    text: [char; 3],
}

impl StepRecordU128 {
    #[inline]
    pub fn new(for_field_id: usize, direction: DirectionType, tape_before: u128) -> Self {
        Self {
            for_field_id,
            direction,
            tape_before,
            #[cfg(all(debug_assertions, feature = "debug_cycler"))]
            text: Self::to_chars(for_field_id, direction),
        }
    }

    //     #[cfg(all(debug_assertions, feature = "debug_cycler"))]
    //     pub fn for_state(&self) -> i16 {
    //         (self.for_state_symbol & Self::FILTER_STATE) >> 1
    //     }
    //
    //     #[cfg(all(debug_assertions, feature = "debug_cycler"))]
    //     pub fn for_symbol(&self) -> i16 {
    //         self.for_state_symbol & Self::FILTER_SYMBOL_PURE
    //     }

    #[cfg(all(debug_assertions, feature = "debug_cycler"))]
    pub fn field_id_to_string(&self) -> String {
        TransitionSymbol2::field_id_to_string(self.for_field_id)
    }

    #[cfg(all(debug_assertions, feature = "debug_cycler"))]
    fn to_chars(for_field_id: usize, direction: i16) -> [char; 3] {
        let dir = match direction {
            -1 => 'L',
            1 => 'R',
            _ => '-',
        };
        let s = TransitionSymbol2::field_id_to_string(for_field_id);
        // let state = if from_state & crate::transition_symbol2::FILTER_STATE == 0 {
        //     'Z'
        // } else {
        //     (((from_state & crate::transition_symbol2::FILTER_STATE) >> 1) as u8 + b'A' - 1) as char
        // };

        [s.as_bytes()[0] as char, s.as_bytes()[1] as char, dir]
    }
}
