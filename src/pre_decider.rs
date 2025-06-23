//! This crate contains the pre-decider functionality. This are all deciders which do not require a
//! step-by-step approach but can solely decide on the given transition table. This is extremly quick
//! and rules out >90% of the machines. \
//! This is implemented in an even more efficient way in GeneratorReduced (which is should be used always).
//! GeneratorFull generates all machines and then can be filtered by this pre-decider first. Just call
//! run_pre_decider(&machine) for this.

use crate::{
    status::{MachineStatus, PreDeciderReason},
    transition_symbol2::{TransitionSymbol2, TransitionTableSymbol2},
    MAX_STATES,
};

/// This struct allows the predecider to be put in the decider chain. It is not required,
/// run_pre_decider(&machine) can be used separately.
pub struct PreDecider;

// impl Decider for PreDecider {
//     fn new_decider(&self) -> Self {
//         PreDecider
//     }
//
//     fn decide_machine(&mut self, machine: &crate::machine::Machine) -> MachineStatus {
//         let r = run_pre_decider(machine.transition_table());
//         if r == MachineStatus::NoDecision {
//             return MachineStatus::Undecided(crate::status::UndecidedReason::Undefined, 0, 0);
//         }
//         r
//     }
//
//     fn name(&self) -> String {
//         "PreDecider".to_string()
//     }
// }

// TODO same checks, e.g. only right, when not all states are used
// TODO Hypthesis: Longest contains self referencing element, e.g. BB5 MAX B1, D1
// TODO pre decider states: state B: only 1 of the two can have a state higher than C. In case one points to state A or B, then max C is allowed.
// TODO For state C: only 1 of the two can have a state higher than D. In case one points to state A, B or C, then max D is allowed for the other.
/// Runs quick deciders, which only check the transition table without a step-by-step execution. \
/// Example: Is there exactly one hold condition? If no hold condition exists, it runs endlessly. If more than one hold
/// condition exist, then this machine may hold sometime, but will not be the max machine for this many n_states.
/// Only the machines which have a hold condition in A0 will return status hold with 1 step, all others will return
/// an elimination decription. \
/// The returned count on the description is not complete. That is because multiple deciders may apply
/// and only the first one gets counted. For instance, a transition table may write only zeros, go only to right
/// and have too many hold conditions. The deciders are ordered in a reasonable way depending on statistical relevance
/// (the most likely check first, as this will make the check on the other deciders obsolete) and complexity (execution time).
/// Returns MachineStatus::NoDecision if no special case could be identified.
pub fn run_pre_decider(table: &TransitionTableSymbol2) -> MachineStatus {
    // check loop quick bit

    // check if first element is hold
    if table.transition_start().is_hold() {
        // let ones_on_tape = if transition_start().is_symbol_one() { 1 } else { 0 };
        // // self.info.status = MachineStatus::DecidedHolds(1, ones_on_tape);
        // // return 1 // step;
        // return MachineStatus::DecidedHolds(1, ones_on_tape);
        return MachineStatus::DecidedHolds(1);
    }

    if check_first_status_recursive(table) {
        // return MachineStatus::DecidedEndless(EndlessReason::StartRecursive);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::StartRecursive);
    }

    let n_states = table.n_states();
    let tr_used = table.transitions_used(n_states);
    let c = count_hold_transitions(tr_used);
    // if c == 0 {
    //     return MachineStatus::DecidedEndless(EndlessReason::NoHoldTransition);
    // } else if c > 1 {
    //     // return MachineStatus::NoDecision;
    //     return MachineStatus::DecidedNotMaxTooManyHoldTransitions;
    // }
    if c != 1 {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotExactlyOneHoldCondition);
    }

    if check_only_one_direction(tr_used) {
        // return MachineStatus::DecidedEndless(EndlessReason::OnlyOneDirection);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::OnlyOneDirection);
    }

    if check_simple_start_loop(table) {
        // return MachineStatus::DecidedEndless(EndlessReason::SimpleStartLoop);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::SimpleStartLoop);
    }

    if check_only_zero_writes(tr_used) {
        // let machine = Machine::new(0, *table);
        // let status = DeciderU128Long::<SubDeciderDummy>::run_decider(&machine);
        // match status {
        //     MachineStatus::DecidedHolds(steps) => {
        //         println!("{table}, Steps {steps}");
        //         return status;
        //     }
        //     _ => {}
        // }
        // return MachineStatus::DecidedEndless(EndlessReason::WritesOnlyZero);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::WritesOnlyZero);
    }

    if check_not_all_states_used(table, n_states) {
        // return MachineStatus::DecidedNotMaxNotAllStatesUsed;
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotAllStatesUsed);
    }

    MachineStatus::NoDecision
}

// All checks return true if the check condition is met, in other words an error is returned.

/// Checks if the first transition A0 changes the state. If not, it will  
/// run endless as the same entry is used all the time. \
/// This eliminates 0LA, 1LA, 0RA and 1RA as first entry.
#[inline]
pub fn check_first_status_recursive(table: &TransitionTableSymbol2) -> bool {
    table.transition_start().has_next_state_a()
}

/// Elimination Rule 7: Only zero written
/// Check if any entry in the first column writes a 1 or holds. Otherwise  
/// it will run endless.  
/// TODO check: Even if the start transition A0 writes a 1, it will run endless.
#[inline]
pub fn check_only_zero_writes(tr_used: &[TransitionSymbol2]) -> bool {
    !tr_used.iter().step_by(2).any(|t| t.is_symbol_one())
}

/// Elimination Rule 8: Only one direction
/// Check if all transistions go into the same direction, they will encounter 0 only.  
/// Also required: No hold for 0 column.  
/// Since only 0 is encountered, column 1 is irrelevant
#[inline]
pub fn check_only_one_direction(tr_used: &[TransitionSymbol2]) -> bool {
    // all going right
    (tr_used
        .iter()
        .step_by(2)
        .all(|t| t.is_dir_right())
        // or all going left
        || tr_used
            .iter()
             .step_by(2)
            .all(|t| t.is_dir_left()))
            // or state hold encountered. This is usually not necessary if direction == 0 in case of hold state.
        && tr_used
            .iter()
             .step_by(2)
            .all(|t| !t.is_hold() )
}

#[inline]
pub fn check_no_hold_transition(tr_used: &[TransitionSymbol2]) -> bool {
    !tr_used.iter().any(|t| t.is_hold())
}

/// Elimination Rule 5: Not exactly one hold condition
#[inline]
pub fn count_hold_transitions(tr_used: &[TransitionSymbol2]) -> usize {
    tr_used.iter().filter(|t| t.is_hold()).count()
}

/// Elimination Rule 6: Simple start loop
/// A simple start loop consists of two elements, the start transition and the following  
/// transition to go back to start, e.g. A0:0RC and C0:0LA.  
/// If in both cases the 0 is written (direction irrelevant), then this is endless.
#[inline]
pub fn check_simple_start_loop(table: &TransitionTableSymbol2) -> bool {
    if table.transition_start().is_symbol_one() {
        return false;
    }

    let start_state = table.transition_start().state_x2();
    // now compare if it goes back to state 1
    table.transition(start_state).has_next_state_a() && table.transition(start_state).symbol() == 0
}

/// This check will validate the actually used states by following the used states starting from A0.  
/// It requires that A0 is not hold.
#[inline]
pub fn check_not_all_states_used(table: &TransitionTableSymbol2, n_states: usize) -> bool {
    // array for check result per state
    let mut state_check = [false; (MAX_STATES + 1)];
    // check states for A0 and followinf x0
    let sa0 = table.transition_start().state() as usize;
    state_check[sa0] = true;
    let mut states_used = 1; // counter avoids double loop
    let s = table.transition(sa0 * 2).state() as usize;
    if s != 0 && s != sa0 {
        state_check[s] = true;
        states_used += 1;
    }

    // check all not used states
    // not rusty for performance
    let n_states_plus_1 = n_states + 1;
    loop {
        let mut found = false;
        if states_used == n_states {
            return false;
        } else {
            // get target states for all used states
            for s in 1..n_states_plus_1 {
                if state_check[s] {
                    let t = table.transition(s * 2).state() as usize;
                    if t != 0 && !state_check[t] {
                        state_check[t] = true;
                        states_used += 1;
                        found = true;
                    }
                    let t = table.transition(s * 2 + 1).state() as usize;
                    if t != 0 && !state_check[t] {
                        state_check[t] = true;
                        states_used += 1;
                        found = true;
                    }
                }
            }
        }
        if !found {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_check_pre_decider_no_decision() {
        // check does not apply
        // TODO BB4 max TODO jkfsdl
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = run_pre_decider(&tc);
        assert_eq!(check_result, MachineStatus::NoDecision);

        // BB5 max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = run_pre_decider(&tc);
        assert_eq!(check_result, MachineStatus::NoDecision);
    }

    #[test]
    fn test_check_pre_decider_not_all_states_used() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        // TODO is this BB5 MAX, then maybe the rhythm is clear and BB6 can be created
        // transitions.push(("0RA", "0RA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, false);

        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LD")); // goes to A or D
        transitions.push(("0LB", "1LD")); // goes to B or D

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LA")); // always to A, so B is unused

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, true);
    }

    #[test]
    fn test_check_pre_decider_only_zero_writes() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "---"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_only_zero_writes(&tc.transitions_used_self());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RB", "1RB"));
        transitions.push(("0LA", "1LA"));
        transitions.push(("---", "1LA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_only_zero_writes(&tc.transitions_used_self());
        assert_eq!(check_result, true);
    }

    //     // #[test]
    //     // fn test_check_loop() {
    //     //     // Holds after 14 steps
    //     //     let mut transitions: Vec<(&str, &str)> = Vec::new();
    //     //     transitions.push(("0LB", "1LB"));
    //     //     transitions.push(("1RC", "---"));
    //     //     transitions.push(("1LA", "0RA"));
    //     //     transitions.push(("0RA", "0RA"));
    //     //     // steps
    //     //     // 01 0LB  +00
    //     //     // 02 1RC  1+0
    //     //     // 03 1LA  +11
    //     //     // 04 1LB  +011
    //     //     // 05 1RC  1+11
    //     //     // 06 0RA  10+1
    //     //     // 07 1LB  1+01
    //     //     // 08 1RC  11+1
    //     //     // 09 0RA  110+0
    //     //     // 10 0LB  11+00
    //     //     // 11 1RC  111+0
    //     //     // 12 1LA  11+11
    //     //     // 13 1LB  1+111
    //     //     // 14 ---
    //
    //     //     let machine = MachineV2::new(transitions);
    //     //     let check_result = decider_first_v1(&machine.info);
    //     //     let r = match check_result {
    //     //         MachineStatus::NotRun => todo!(),
    //     //         MachineStatus::Running => todo!(),
    //     //         MachineStatus::DecidedEndless(_) => todo!(),
    //     //         MachineStatus::DecidedHolds(_, _) => true,
    //     //         MachineStatus::Undecided(_, _) => false,
    //     //     };
    //     //     assert_eq!(r, true);
    //
    //     //     // Hold found
    //     //     let mut transitions: Vec<(&str, &str)> = Vec::new();
    //     //     transitions.push(("1RB", "1RB"));
    //     //     transitions.push(("---", "---"));
    //
    //     //     let machine = MachineV2::new(transitions);
    //     //     let check_result = decider_first_v1(&machine.info);
    //     //     let r = match check_result {
    //     //         MachineStatus::NotRun => todo!(),
    //     //         MachineStatus::Running => todo!(),
    //     //         MachineStatus::DecidedEndless(_) => todo!(),
    //     //         MachineStatus::DecidedHolds(_, _) => true,
    //     //         MachineStatus::Undecided(_, _) => false,
    //     //     };
    //     //     assert_eq!(r, true);
    //     // }

    #[test]
    fn test_check_pre_decider_only_one_direction() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "---"));
        // transitions.push(("---", "---"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_only_one_direction(&tc.transitions_used_self());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1RA", "0RA"));
        // transitions.push(("---", "---"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_only_one_direction(&tc.transitions_used_self());
        assert_eq!(check_result, true);
    }

    #[test]
    fn test_check_pre_decider_no_hold_condition() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "---"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_no_hold_transition(&tc.transitions_used_self());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "1LA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_no_hold_transition(&tc.transitions_used_self());
        assert_eq!(check_result, true);
    }

    #[test]
    fn test_check_pre_decider_simple_start_loop() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "---"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_simple_start_loop(&tc);
        assert_eq!(check_result, false);

        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RB"));
        transitions.push(("1LA", "1LA"));
        transitions.push(("0LB", "1LA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_simple_start_loop(&tc);
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RB"));
        transitions.push(("1LA", "1LA"));
        transitions.push(("0LA", "1LA"));

        let tc = TransitionTableSymbol2::from_string_tuple(&transitions);
        let check_result = check_simple_start_loop(&tc);
        assert_eq!(check_result, true);
    }

    // fn run_test_pre_decider(transitions: &[(&str, &str)]) -> MachineStatus {
    //     let tc = TransitionTableCompact::from_string_tuple(&transitions);
    //     run_pre_deciders(&tc, tc.n_states())
    // }
}
