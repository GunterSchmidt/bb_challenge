use crate::{
    config::MAX_STATES,
    data_provider::DataProvider,
    decider_result::PreDeciderCount,
    machine::Machine,
    transition_symbol2::{TransitionSymbol2, TRANSITION_SYM2_HOLD},
};

#[non_exhaustive]
pub enum GeneratorStandard {
    GeneratorFull,
    GeneratorReduced,
}

// TODO remove what is in DataProvider
pub trait Generator: DataProvider {
    /// Create new generator for random batch no. \
    /// Avoids some recalculations for e.g. batch_size, but gives normal initialized struct.
    // TODO remove when decider_deprecated is removed
    fn new_from_generator_deprecated(&self) -> Self;

    /// Returns the specific batch of permutations and an info if this is the last batch.
    fn generate_permutation_batch_no(&mut self, batch_no: usize) -> (Vec<Machine>, bool);

    /// Returns the next batch of permutations and an info if this is the last batch.
    fn generate_permutation_batch_next(&mut self) -> (Vec<Machine>, bool);

    /// The given limit of machines to generate or (if smaller) the maximum number of machines for the number of states.
    fn limit(&self) -> u64;

    fn pre_decider_count(&self) -> PreDeciderCount;

    fn num_eliminated(&self) -> u64;

    fn check_generator_batch_size_request_single_thread(&mut self);

    /// The batch size for the packages of Turing machines generated in each call. \
    /// The size is reduced to the nearest multiple of permutations for one state, which
    /// is the number of transition variants squared.
    fn calc_batch_size(max_batch_size: usize, n_states: usize) -> usize {
        assert!(n_states <= MAX_STATES);
        let n_machines = num_turing_machine_permutations(n_states);
        if n_machines <= max_batch_size as u128 {
            return n_machines as usize;
        }

        let permutations = 4 * n_states + 1;
        let divider = permutations * permutations;
        // batch_size
        (max_batch_size / divider) * divider
    }
}

// impl Display for Generator {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "limit: {}, batch size: {}",
//             self.limit(),
//             self.batch_size()
//         )
//     }
// }

/// This creates all transition permutations for one field, e.g. \
/// 0RA, 1RA, 0LA, 1LA, --- for BB1 \
/// The number can be calculated by (4 * n_states + 1), e.g. 21 for BB5
pub fn create_all_transition_permutations(n_states: usize) -> Vec<TransitionSymbol2> {
    let mut transitions = Vec::new();
    let mut tr: [u8; 3];

    // all to right
    for i in 1..=n_states {
        // tr as symbol, direction, next state
        tr = [0, 0, i as u8];
        transitions.push(TransitionSymbol2::new(tr).unwrap());
        // write symbol
        tr[0] = 1;
        transitions.push(TransitionSymbol2::new(tr).unwrap());
    }
    // all to left
    for i in 1..=n_states {
        // tr as symbol, direction, next state
        tr = [0, 1, i as u8];
        transitions.push(TransitionSymbol2::new(tr).unwrap());
        // write symbol
        tr[0] = 1;
        transitions.push(TransitionSymbol2::new(tr).unwrap());
    }
    // hold as last transition
    transitions.push(TRANSITION_SYM2_HOLD);

    transitions
}

// /// Elimination Rule 2: Left and Right: The directions L and R can be interchanged in all steps.
// /// Elimination Rule x: State interchangable: Any state can be interchanged with any other state other than A.
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
