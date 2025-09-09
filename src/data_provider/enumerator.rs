use crate::{
    config::MAX_STATES, data_provider::DataProvider, decider::decider_result::PreDeciderCount,
    machine_binary::MachineId, transition_binary::TRANSITION_BINARY_HALT,
};

/// Number of fields used in the transition table (Turing machine).
pub const NUM_FIELDS: usize = MAX_STATES * 2 + 2;

#[non_exhaustive]
#[derive(Debug)]
pub enum EnumeratorStandard {
    EnumeratorFull,
    EnumeratorReducedForward,
    EnumeratorReducedBackward,
}

// TODO remove what is in DataProvider. Why?
pub trait Enumerator: DataProvider {
    /// Returns the specific batch of permutations and an info if this is the last batch.
    fn enumerate_permutation_batch_no(&mut self, batch_no: usize) -> (Vec<MachineId>, bool);

    /// Returns the next batch of permutations and an info if this is the last batch.
    fn enumerate_permutation_batch_next(&mut self) -> (Vec<MachineId>, bool);

    /// The given limit of machines to enumerate or (if smaller) the maximum number of machines for the number of states.
    fn limit(&self) -> u64;

    fn pre_decider_count(&self) -> PreDeciderCount;

    fn num_eliminated(&self) -> u64;

    // fn check_Enumerator_batch_size_request_single_thread(&mut self);

    /// The batch size for the packages of Turing machines enumerated in each call. \
    /// The size is reduced to the nearest multiple of permutations for one state, which
    /// is the number of transition variants squared.
    fn calc_batch_size(max_batch_size: usize, n_states: usize, n_machines: u128) -> usize {
        assert!(n_states <= MAX_STATES);
        // let n_machines = num_turing_machine_permutations(n_states);
        if n_machines <= max_batch_size as u128 {
            return n_machines as usize;
        }

        let permutations = 4 * n_states + 1;
        let divider = permutations * permutations;
        // batch_size
        (max_batch_size / divider) * divider
    }
}

// impl Display for Enumerator {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "limit: {}, batch size: {}",
//             self.limit(),
//             self.batch_size()
//         )
//     }
// }

// /// Elimination Rule 2: Left and Right: The directions L and R can be interchanged in all steps.
// /// Elimination Rule x: State interchangeable: Any state can be interchanged with any other state other than A.
// /// Therefore the start transition must go to B.
// /// Elimination Rule 3: No Hold in start field A0.
// /// Elimination Rule 4: No reference to state A in A0.
// /// In the end only 0RB and 1RB remain, so this is disables and replaced with a const.
// pub fn filter_all_transition_permutations_for_a0(
//     all_transition_permutations: &[TransitionSymbol2],
// ) -> Vec<TransitionSymbol2> {
//     let filtered: Vec<TransitionSymbol2> = all_transition_permutations
//         .iter()
//         .filter(|t| t.state() == 2 && t.goes_right() && !t.has_next_state_a() && !t.is_hold())
//         .copied()
//         .collect::<Vec<TransitionSymbol2>>();
//
//     filtered
// }

/// Number of Turing machines for Alphabet 2 and n states (limit n = 7) \
/// Formula (4n+1)^2n \
/// Source: <https://bbchallenge.org/story#definition-of-bb>
pub fn num_turing_machine_permutations_u64(n_states: usize) -> u64 {
    // 4 * n_states + 1: Each state has 2 directions and 2 symbols, giving 4 permutations. Additional there is one hold permutation.
    // pow(2 * n_states): now a table is created for each state with two read symbols and each field can hold all permutations.
    assert!(n_states <= 7, "Limit for u64 is a maximum of 7 states.");
    ((4 * n_states + 1) as u64).pow(2 * n_states as u32)
}

/// Number of Turing machines for Alphabet 2 and n states (limit n = 10) \
/// Formula (4n+1)^2n \
/// Source: <https://bbchallenge.org/story#definition-of-bb>
pub fn num_turing_machine_permutations(n_states: usize) -> u128 {
    assert!(n_states <= 10, "Limit for u128 is a maximum of 10 states.");
    ((4 * n_states + 1) as u128).pow(2 * n_states as u32)
}

/// In some enumerators, no machines are created as field A0 usually starts with 0RB or 1RB. Therefore fake the result.
pub fn machines_for_n_states_1() -> Vec<MachineId> {
    let mut tr_permutations =
        crate::transition_binary::TransitionBinary::create_all_transition_permutations(1);
    tr_permutations[4] = TRANSITION_BINARY_HALT;
    let mut transition_table = crate::machine_binary::MachineBinary::new_default(1);
    transition_table.transitions[2] = crate::transition_binary::TRANSITION_BINARY_HALT;
    let mut machines = Vec::new();
    for tr in tr_permutations {
        transition_table.transitions[3] = tr;
        machines.push(MachineId::new_no_id(transition_table));
    }

    machines
}
