//! This crate contains the pre-decider functionality. This are all deciders which do not require a
//! step-by-step approach but be can solely decided on the given transition table. This is extremely quick
//! and rules out >90% of the machines. \
//! This is implemented in an even more efficient way in EnumeratorReduced (which should be used always).
//! EnumeratorFull generates all machines and then can be filtered by this pre-decider first. Just call
//! run_pre_decider(&machine) for this.

use crate::{
    config::MAX_STATES,
    machine_binary::MachineBinary,
    status::{MachineStatus, PreDeciderReason},
    transition_binary::{TransitionBinary, TransitionType, STATE_HALT_BINARY, TRANSITIONS_FOR_A0},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreDeciderRun {
    DoNotRun,
    RunNormalForward,
    RunStartBRightOnly,
}

/// This struct allows the predecider to be put in the decider chain. It is not required,
/// run_pre_decider(&machine) can be used separately.
pub struct PreDecider;

// TODO same checks, e.g. only right, when not all states are used
// TODO Hypothesis: Longest contains self referencing element, e.g. BB5 MAX B1, D1
// TODO pre decider states: state B: only 1 of the two can have a state higher than C. In case one points to state A or B, then max C is allowed.
// TODO For state C: only 1 of the two can have a state higher than D. In case one points to state A, B or C, then max D is allowed for the other.
// TODO https://turbotm.de/~heiner/BB/mabu90.html#Enumeration:
// If there are two states which are (syntactically) equivalent, these two can be identified (Sigma(N+1) > Sigma(N)). Example: (x,0)->(x,0,R), (x,1)->(z,0,L), (y,0)->(y,0,R) and (y,1)->(z,0,L) imply that states x and y are equivalent.
// If a sequence of three transitions is guaranteed to have the same effect as a single transition, only one of both constructions need be inspected. Example: Let x,y,z and s be states with x!=y, a, b, and c arbitrary symbols, and D from {L,R}, then (x,0)->(y,0,L), (x,1)->(y,1,L), and (y,b)->(z,c,D) implies that (s,a)->(x,b,R) and (s,a)->(z,c,D) have the same effect.

/// Runs quick deciders, which only check the transition table without a step-by-step execution. \
/// Example: Is there exactly one hold condition? If no hold condition exists, it runs endlessly. If more than one hold
/// condition exist, then this machine may hold sometime, but will not be the max machine for this many n_states.
/// Only the machines which have a hold condition in A0 will return status hold with 1 step, all others will return
/// an elimination description. \
/// The returned count on the description is not complete. That is because multiple deciders may apply
/// and only the first one gets counted. For instance, a transition table may write only zeros, go only to right
/// and have too many hold conditions. The deciders are ordered in a reasonable way depending on statistical relevance
/// (the most likely check first, as this will make the check on the other deciders obsolete) and complexity (execution time).
/// Returns MachineStatus::NoDecision if no special case could be identified.
#[inline(always)]
pub fn run_pre_decider_strict(machine: &MachineBinary) -> MachineStatus {
    // check if first element is hold
    if machine.transition_start().is_halt() {
        return MachineStatus::DecidedHalt(1);
    }

    // check like EnumeratorReduced: State Start can only be 0RB or 1RB, otherwise
    // - is recursive if state A is next state
    // - going to left is same as going to right -> no L direction
    // - states can be switched if going to C, D or E
    if machine.transition_start() != TRANSITIONS_FOR_A0[0]
        && machine.transition_start() != TRANSITIONS_FOR_A0[1]
    {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotStartStateBRight);
    }

    let n_states = machine.n_states();
    let tr_used = machine.transitions_used(n_states);
    if count_hold_transitions(tr_used) != 1 {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotExactlyOneHaltCondition);
    }

    if check_only_one_direction(tr_used) {
        // return MachineStatus::DecidedEndless(EndlessReason::OnlyOneDirection);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::OnlyOneDirection);
    }

    if check_simple_start_cycle(machine) {
        // return MachineStatus::DecidedEndless(EndlessReason::SimpleStartLoop);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::SimpleStartCycle);
    }

    if check_only_zero_writes(tr_used) {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::WritesOnlyZero);
    }

    if check_not_all_states_used(machine, n_states) {
        // return MachineStatus::DecidedNotMaxNotAllStatesUsed;
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotAllStatesUsed);
    }

    MachineStatus::NoDecision
}

#[inline(always)]
pub fn run_pre_decider_simple(machine: &MachineBinary) -> MachineStatus {
    // check if first element is hold
    if machine.transition_start().is_halt() {
        return MachineStatus::DecidedHalt(1);
    }

    if check_start_transition_is_recursive(machine) {
        // return MachineStatus::DecidedEndless(EndlessReason::StartRecursive);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::StartRecursive);
    }

    let n_states = machine.n_states();
    let tr_used = machine.transitions_used(n_states);
    if count_hold_transitions(tr_used) != 1 {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::NotExactlyOneHaltCondition);
    }

    if check_only_one_direction(tr_used) {
        // return MachineStatus::DecidedEndless(EndlessReason::OnlyOneDirection);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::OnlyOneDirection);
    }

    if check_simple_start_cycle(machine) {
        // return MachineStatus::DecidedEndless(EndlessReason::SimpleStartLoop);
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::SimpleStartCycle);
    }

    if check_only_zero_writes(tr_used) {
        return MachineStatus::EliminatedPreDecider(PreDeciderReason::WritesOnlyZero);
    }

    if check_not_all_states_used(machine, n_states) {
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
pub fn check_start_transition_is_recursive(table: &MachineBinary) -> bool {
    table.transition_start().has_next_state_a()
}

/// Elimination Rule 7: Only zero written
/// Check if any entry in the first column writes a 1 or holds. Otherwise
/// it will run endless.
/// TODO check: Even if the start transition A0 writes a 1, it will run endless.
#[inline]
pub fn check_only_zero_writes(tr_used: &[TransitionBinary]) -> bool {
    !tr_used.iter().step_by(2).any(|t| t.is_symbol_one())
}

/// Elimination Rule 8: Only one direction
/// Check if all transitions go into the same direction, they will encounter 0 only.  
/// Also required: No hold for 0 column.  
/// Since only 0 is encountered on the tape if moved in same direction, 2nd column is irrelevant
/// The machine can be endless (if no hold is in column 0) or may hold very quickly (not max).
#[inline]
pub fn check_only_one_direction(tr_used: &[TransitionBinary]) -> bool {
    // all going right
    tr_used
        .iter()
        .step_by(2)
        .all(|t| t.is_dir_right() || t.is_halt())
        // or all going left
        || tr_used
            .iter()
             .step_by(2)
            .all(|t| t.is_dir_left() || t.is_halt())
    // add parenthesis if and is re-enabled
    // or state hold encountered. This is usually not necessary if direction == 0 in case of hold state.
    // && tr_used
    //     .iter()
    //      .step_by(2)
    //     .all(|t| !t.is_hold() )
}

/// Elimination Rule 8: Only one direction
/// Check if all transitions go into the same direction, they will encounter 0 only.  
/// Also required: No hold for 0 column.  
/// Since only 0 is encountered on the tape if moved in same direction, column 1 is irrelevant
/// The machine can be endless (if no hold is in column 0) or may hold very quickly (not max).
#[inline]
pub fn check_only_right_direction(tr_used: &[TransitionBinary]) -> bool {
    // all going right
    // skip 0 as A0 is always going right
    tr_used[2..]
        .iter()
        .step_by(2)
        .all(|t| t.is_dir_right() || t.is_halt())
}

/// Checks if no hold transition exists at all.
#[inline]
pub fn check_no_hold_transition(tr_used: &[TransitionBinary]) -> bool {
    !tr_used.iter().any(|t| t.is_halt())
}

/// Elimination Rule 5: Not exactly one hold condition.
#[inline]
pub fn count_hold_transitions(tr_used: &[TransitionBinary]) -> usize {
    tr_used.iter().filter(|t| t.is_halt()).count()
}

/// Elimination Rule 6: Simple start loop
/// A simple start loop consists of two elements, the start transition and the following  
/// transition to go back to start, e.g. A0:0RC and C0:0LA.  \
/// Case 1: 0RBxxx_0RAxxx: Writes only 0 repeatedly and goes right endless. \
/// Case 2: 0RBxxx_1RAxxx: Writes only 01 repeatedly and goes right endless. \
/// Case 3: 1RBxxx_0RAxxx: Writes only 10 repeatedly and goes right endless. \
/// Case 4: 1RBxxx_1RAxxx: Writes only 1 repeatedly and goes right endless. \
/// Case 5: 0RBxxx_0LAxxx: Writes only 00 and cycles endless. \
/// Case 6: 0RBxxx_1LAxxx: Writes 01 and step 4 requires A1, not a simple start loop. \
/// Case 7: 1RBxxx_0LAxxx: Writes 10 and step 2 requires A1, not a simple start loop. \
/// Case 8: 1RBxxx_1LAxxx: Writes 11 and step 2 requires A1, not a simple start loop. \
/// If in both cases the 0 is written (direction irrelevant), then this is endless.
///
/// 2nd recursion
/// 0RB---_1LB0RA -> 0RB, 1RB, 1RB endless
// TODO extend to step 4?
#[inline]
pub fn check_simple_start_cycle(table: &MachineBinary) -> bool {
    let t_start = table.transition_start();
    let start_state = t_start.state_x2();
    let tr_2nd = table.transition(start_state);
    // 2nd needs to point back to A0 (0 is always the case)
    if tr_2nd.has_next_state_a() {
        if t_start.is_symbol_one() && tr_2nd.direction() == t_start.direction() {
            // case 3 and 4, also to left: true, else case 7, 8
            return true;
        } else if tr_2nd.direction() == t_start.direction() || tr_2nd.is_symbol_zero() {
            // case 1, 2, 5: true, 6: false
            return true;
        }
    }

    // 2nd is a recursion
    if t_start.is_symbol_zero() && tr_2nd.state_x2() == start_state {
        return true;
    }

    false
}

/// This check will validate the actually used states by following the used states starting from A0.  
/// It requires that A0 is not hold and A0 is not recursive (previous checks will ensure this).
/// The pre-decider [check_only_one_direction] needs to be run before this.
#[inline]
pub fn check_not_all_states_used(table: &MachineBinary, n_states: usize) -> bool {
    // array for check result per state
    let mut states_used = [(false, false); (MAX_STATES + 1)];
    // check states for A0 and following x0
    let a0_state_next = table.transition_start().state() as usize;
    // set state pointed to from a0 to used (true)
    // example: A0: 1RC will have next transition C0 as the tape is empty. It is possible that A is never visited again. Than A1 is not used.
    states_used[a0_state_next].0 = true;
    let mut state_fields_used = 1;

    // use array instead of vec for performance
    let mut state_stack = [0; 10];
    let mut state_stack_size = 0;
    // follow state from A0 and look where it is going
    let second_state_next_symbol_0 = table.transition(a0_state_next * 2).state() as usize;
    if second_state_next_symbol_0 == STATE_HALT_BINARY as usize {
        return true;
    }
    // in this example mark C0 as used, but it is possible C is never visited again
    // example goes back to A, but from now on it is unclear if symbol on tape is 0 or 1
    // TODO (unless both have been writing 0)
    let s0 = table.transition(second_state_next_symbol_0 * 2).state() as usize;
    if s0 == STATE_HALT_BINARY as usize {
        return true;
    }
    // mark both fields as used
    states_used[second_state_next_symbol_0] = (true, true);
    state_fields_used += if second_state_next_symbol_0 == a0_state_next {
        1
    } else {
        2
    };

    state_stack[state_stack_size] = s0;
    state_stack_size += 1;
    let s1 = table.transition(second_state_next_symbol_0 * 2 + 1).state() as usize;
    if s0 != s1 && s1 != STATE_HALT_BINARY as usize {
        state_stack[state_stack_size] =
            table.transition(second_state_next_symbol_0 * 2 + 1).state() as usize;
        state_stack_size += 1;
    }
    // now follow until all states have been evaluated
    while state_stack_size > 0 {
        let state = state_stack[state_stack_size - 1];
        state_stack_size -= 1;
        if !states_used[state].0 {
            let s = table.transition(state * 2).state() as usize;
            if s != STATE_HALT_BINARY as usize && s != state {
                state_stack[state_stack_size] = s;
                state_stack_size += 1;
            }
            states_used[state].0 = true;
            state_fields_used += 1;
        }
        if !states_used[state].1 {
            let s = table.transition(state * 2 + 1).state() as usize;
            if s != STATE_HALT_BINARY as usize && s != state {
                state_stack[state_stack_size] = s;
                state_stack_size += 1;
            }
            states_used[state].1 = true;
            state_fields_used += 1;
        }
    }

    if state_fields_used < n_states * 2 {
        if state_fields_used == n_states * 2 - 1 {
            println!("Transitions: {}", table.to_standard_tm_text_format());
            println!("{}", table.to_table_string(false));
            todo!("states");
        }
        return true;
    }

    false
}

/// This pre-decider eliminates machines which use a direct path through the
/// transitions and either hold or run endless because of recursion.
/// Example 1RB0RA_0RB1LC_1LC---: A0 goes to B0, B0 referenced on itself -> endless.
/// Example 1RB0RB_1RC0RC_---0LC:
#[inline]
pub fn check_straight_to_end(table: &MachineBinary, n_states: usize) -> bool {
    // check states for A0 and following x0
    // let a0_state_next = table.transition_start().state() as usize;
    // // follow state from A0 and look where it is going
    // let second_state_next_symbol_0 = table.transition(a0_state_next * 2).state() as usize;
    // if second_state_next_symbol_0 == STATE_HOLD_SYM2 as usize {
    //     return true;
    // }
    // let s0 = table.transition(second_state_next_symbol_0 * 2).state() as usize;
    // if s0 == STATE_HOLD_SYM2 as usize {
    //     return true;
    // }
    // println!("Test Straight: {}", table.to_standard_tm_text_format());
    let mut tr = table.transition_start();
    let mut state = tr.state();
    let dir = tr.direction();
    let mut steps = 1;

    while steps < n_states + 2 {
        steps += 1;
        // read field for symbol 0
        tr = table.transitions[state as usize * 2];
        if tr.is_halt() {
            return true;
        }
        // as long direction does not change tape will be 0
        if tr.direction() == dir {
            // self referencing?
            if tr.state() == state {
                return true;
            }
        } else {
            return false;
        }
        state = tr.state();
    }

    false
}

/// This pre-decider eliminates valid, but not required machines, because they are essentially identical,
/// only the state order has been changed. \
/// Example for BB4: \
/// Machine No.   191,658,921: 1RB1LB_1LA0LC_---1LD_1RD0RA \
/// Machine No. 5,721,093,031: 1RB1LB_1LA0LD_1RC0RA_---1LC \
/// These machines are identical, only the states C and D are flipped. \
/// The following logic applies: \
/// States must appear in ascending order, no state can be skipped.
/// TODO this is not quite right
/// State A: A0 is 0RB or 1RB anyway. A1 can be anything.
/// State B: B0 or B1: At least one must be A, B or C.
/// State C: C0 or C1: At least one must be A, B or C or D.
/// Pre-Conditions
/// * A0 must to be 0RB or 1RB (strict test)
/// * Only one direction is eliminated
#[inline]
#[allow(unused)]
pub fn check_states_can_be_switched(
    // tr_used: &[TransitionSymbol2],
    table: &MachineBinary,
    n_states: usize,
) -> bool {
    // for (i, tr) in tr_used.iter().enumerate().skip(2){
    // This check requires A0 to be 0RB or 1RB, so A0 is always state B (=2).
    let mut max_state = 2; // table.transitions[2].state();
    let mut jump_to_a = 0;
    // first step always goes to B0
    // match table.transitions[4].state()

    for i in 2..n_states {
        // starts with state B
        let max_state_allowed = (i + 1) as TransitionType;
        // let mut has_one_correct = false;
        if table.transitions[i * 2].state() > max_state_allowed
            && table.transitions[i * 2 + 1].state() > max_state_allowed
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {

    use crate::machine_binary::{MachineBinary, NotableMachineBinary};

    use super::*;

    #[test]
    fn check_pre_decider_states_can_be_switched() {
        // BB4 Max Steps:             107 (Number of machines: 2)
        // Machine No.   191,658,921: 1RB1LB_1LA0LC_---1LD_1RD0RA
        // Machine No. 5,721,093,031: 1RB1LB_1LA0LD_1RC0RA_---1LC
        // Second is identical and does not need to be checked.
        let tm = "1RB1LB_1LA0LD_1RC0RA_---1LC";
        let table = MachineBinary::try_from(tm).unwrap();
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_states_can_be_switched(&table, table.n_states());
        println!("check result: {}", check_result);
        // assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_straight_to_end() {
        // let tm = "1RB0RA_0RB1LC_1LC---";
        // let table = TransitionTableSymbol2::from_standard_tm_text_format(&tm).unwrap();
        // // println!("{}", tc.to_standard_tm_text_format());
        // let check_result = check_straight_to_end(&table, table.n_states());
        // assert_eq!(check_result, true);

        let tm = "1RB0RB_1RC0RC_---0LC";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_straight_to_end(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case is interesting:
        // A 0RB 1RA
        // B 1LC 0RA
        // C 0RC ---
        // Step 1: A0 goes to B: But A is not used yet, unless it comes back to A. Otherwise it is just one more step to BB2.
        // Step 2: B0 goes to C: Since now symbols have been written, both C are possible, but only C0 goes further, to C.
        // Step 3: Either hold or C. So neither A nor B are visited again; thus not max.
        let tm = "0RB1RA_1LC0RA_0RC---";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case just caused an error because of an programming error.
        let tm = "1RB---_0LB0RA_0RA0RA";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case uses all states, but does not come back to A. So A is only used for the start transition,
        // which is regarded as "cannot reach max steps".
        let tm = "1RB1LC_0LC0LC_0LC---";
        let machine = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_not_all_states_used(&machine, machine.n_states());
        assert_eq!(check_result, true);

        let machine = NotableMachineBinary::BB3Max.machine();
        let check_result = check_not_all_states_used(&machine, machine.n_states());
        assert_eq!(check_result, false);

        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        // TODO is this BB5 MAX, then maybe the rhythm is clear and BB6 can be created
        // transitions.push(("0RA", "0RA"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        // println!("{}", tc.to_standard_tm_text_format());
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, false);

        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LD")); // goes to A or D
        transitions.push(("0LB", "1LD")); // goes to B or D

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LA")); // always to A, so B is unused

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_not_all_states_used() {
        let tm = "1RB0RB_1RC0RC_---0LC";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case is interesting:
        // A 0RB 1RA
        // B 1LC 0RA
        // C 0RC ---
        // Step 1: A0 goes to B: But A is not used yet, unless it comes back to A. Otherwise it is just one more step to BB2.
        // Step 2: B0 goes to C: Since now symbols have been written, both C are possible, but only C0 goes further, to C.
        // Step 3: Either hold or C. So neither A nor B are visited again; thus not max.
        let tm = "0RB1RA_1LC0RA_0RC---";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case just caused an error because of a programming error.
        let tm = "1RB---_0LB0RA_0RA0RA";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        // This case uses all states, but does not come back to A. So A is only used for the start transition,
        // which is regarded as "cannot reach max steps".
        let tm = "1RB1LC_0LC0LC_0LC---";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, true);

        let machine = NotableMachineBinary::BB3Max.machine();
        let check_result = check_not_all_states_used(&machine, machine.n_states());
        assert_eq!(check_result, false);

        // check does not apply
        // let mut transitions: Vec<(&str, &str)> = Vec::new();
        // transitions.push(("1RC", "1LC"));
        // transitions.push(("---", "1LD"));
        // transitions.push(("1LA", "0LB"));
        // transitions.push(("1RD", "0RA"));
        // TODO is this BB5 MAX, then maybe the rhythm is clear and BB6 can be created
        // transitions.push(("0RA", "0RA"));

        let tm = "1RC1LC_---1LD_1LA0LB_1RD0RA";
        let table = MachineBinary::try_from_standard_tm_text_format(&tm).unwrap();
        let check_result = check_not_all_states_used(&table, table.n_states());
        assert_eq!(check_result, false);

        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LD")); // goes to A or D
        transitions.push(("0LB", "1LD")); // goes to B or D

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RC")); // always goes to state C
        transitions.push(("---", "1LB"));
        transitions.push(("0LA", "1LA")); // always to A, so B is unused

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_not_all_states_used(&tc, tc.n_states());
        assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_only_zero_writes() {
        // check does not apply
        let tm = "1RB1RB_1LA---";
        let table = MachineBinary::try_from_standard_tm_text_format(tm).unwrap();
        let check_result = check_only_zero_writes(table.transitions_used_eval());
        assert_eq!(check_result, false);

        // check applies
        let tm = "0RB1RB_0LA1LA_---1LA";
        let table = MachineBinary::try_from_standard_tm_text_format(tm).unwrap();
        let check_result = check_only_zero_writes(table.transitions_used_eval());
        assert_eq!(check_result, true);
    }

    //     // #[test]
    //     // fn check_loop() {
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
    fn check_pre_decider_only_one_direction() {
        // only right or hold in column 0
        let tm = "1RB0RB_1RC0RC_---0LC";
        let table = MachineBinary::try_from(tm).unwrap();
        let check_result = check_only_one_direction(&table.transitions_used_eval());
        assert_eq!(check_result, true);

        // check does not apply
        let tm = "1RB1RB_1LA---";
        let table = MachineBinary::try_from(tm).unwrap();
        let check_result = check_only_one_direction(&table.transitions_used_eval());
        assert_eq!(check_result, false);

        // check applies
        let tm = "1RB1RB_1RA0RA";
        let table = MachineBinary::try_from(tm).unwrap();
        let check_result = check_only_one_direction(&table.transitions_used_eval());
        assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_no_hold_condition() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "---"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_no_hold_transition(&tc.transitions_used_eval());
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1RB"));
        transitions.push(("1LA", "1LA"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_no_hold_transition(&tc.transitions_used_eval());
        assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_simple_start_cycle() {
        // check does not apply
        let tm = "1RB1RB_1LA---";
        let tc = MachineBinary::try_from_standard_tm_text_format(tm).unwrap();
        let check_result = check_simple_start_cycle(&tc);
        assert_eq!(check_result, false);

        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RB"));
        transitions.push(("1LA", "1LA"));
        transitions.push(("0LB", "1LA"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_simple_start_cycle(&tc);
        assert_eq!(check_result, false);

        // check applies
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1RB"));
        transitions.push(("1LA", "1LA"));
        transitions.push(("0LA", "1LA"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = check_simple_start_cycle(&tc);
        assert_eq!(check_result, true);
    }

    #[test]
    fn check_pre_decider_no_decision() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1LB"));
        transitions.push(("1LD", "---"));
        transitions.push(("1LA", "0LC"));
        transitions.push(("1RC", "0RB"));

        let tc = MachineBinary::from_string_tuple(&transitions);
        let check_result = run_pre_decider_strict(&tc);
        assert_eq!(check_result, MachineStatus::NoDecision);

        // BB5 max
        let machine = NotableMachineBinary::BB5Max.machine();
        let check_result = run_pre_decider_strict(&machine);
        assert_eq!(check_result, MachineStatus::NoDecision);
    }
}
