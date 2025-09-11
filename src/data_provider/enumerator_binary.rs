//! Enumerator to produce all Turing machines (permutations) for the given number of states.  
//! While this works it is highly inefficient to check every single machine. This can be used
//! at most until BB4. \
//! It mostly exists to validate the logic and results of EnumeratorReduced during development.
//!
//! How it works: \
//! First the initial [transition_table] is created (stack), filled with 0RA as first element for each transition.
//! Also the list of transitions to cycle through is created [tr_permutations], holding the (4n+1) elements.
//! In each cycle one [Permutation] is created, holding the id (starting with 0) and the transition table
//! and stored in [permutations] (heap). \
//! Because only two bytes are used for each transition, this results in a small array which can be copied
//! extremely fast. \
//! During each cycle (usually) only one transition is changed in the transition table,
//! thus minimizing the changed data.
//! After all transitions have been created or the block size is reached, the permutations are returned (moved). \
//!
//! There are two generation option: front or back:
//! - front: Field A0 is rotated first (then A1, B0, B1, C0 etc.) or
//! - back: The last field is rotated first, e.g. for BB5: E1, then E0, D1, D0, C1 etc.
//!
//! Since this is a calculable regular pattern, it is possible to break the permutation generation
//! into several batches and addressing a batch directly (it is not required to create all the
//! permutations up to that batch). This is used to parallelize permutation generation and
//! subsequently the deciders to run in multiple threads.
// TODO enumerationReducedReverse is broken

use crate::{
    config::{Config, NUM_FIELDS},
    data_provider::{
        enumerator::{machines_for_n_states_1, num_turing_machine_permutations, Enumerator},
        DataProvider, DataProviderBatch, DataProviderThreaded, ResultDataProvider,
    },
    decider::{
        decider_result::{EndReason, PreDeciderCount},
        pre_decider::{
            check_not_all_states_used, check_only_right_direction, check_only_zero_writes,
            check_simple_start_cycle, count_hold_transitions, PreDeciderRun,
        },
    },
    machine_binary::{MachineBinary, MachineId},
    status::PreDeciderReason,
    transition_binary::{TransitionBinary, TRANSITIONS_FOR_A0},
};

// const BATCH_SIZE_REQUEST_SINGLE_THREAD_MAX: usize = 500_000;
const TR_PERMUTATIONS_FIELD_DEFAULT: [Vec<TransitionBinary>; NUM_FIELDS] =
    [const { Vec::new() }; NUM_FIELDS];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnumeratorType {
    EnumeratorFullForward,
    EnumeratorFullBackward,
    EnumeratorReducedForward,
    EnumeratorReducedBackwardNotWorking,
    EnumeratorTNF,
}

/// This enumerator creates all permutations of transition sets (Turing machine) possible for the given n_states,
/// where 'hold' is limited to this one transition: '---'. This results in (4n+1)^2n combinations. \
/// The transition table is enumerated by permuting all transition permutations for field A0, then A1, then B0 and so on.
pub struct EnumeratorBinary {
    /// Next id for the enumerated machine, starting with 0. \
    /// The id is correct for the Full Enumerators and the ReverseForward, but indicates just a counter for ReverseBackward.
    /// It can later be set like the full id with calc_id in the MachineInfo.
    // TODO id for larger BBx
    id_next: u64,
    ids_skip_start: u64,
    /// batch_no, increased for every call, batch 0 will show batch 1
    batch_no: usize,
    // batch_no_skip: usize,
    /// The number of states used for this enumerator.
    n_states: usize,
    /// The total number of machines to be enumerated. For Full this is the number of Turing Machines for that n.
    /// For Reduced this is initially the number of Turing Machines to be created (using only 0RB and 1RB for field A0) and
    /// may later be reduced if more (tree) elements can be eliminated.
    n_machines: u64,
    /// The number of machines to process if a limit is applied.
    // TODO This is a bit unclear and needs to be clarified.
    n_machines_to_process: u64,
    /// The reduced actual batch size (number of Turing machines enumerated in each call).
    batch_size: usize,
    /// The given limit of machines to enumerate or (if smaller) the maximum number of machines for the number of states.
    limit_id: u64,
    /// The total number of batches to create all permutations.
    num_batches: usize,
    /// The (4n+1) permutations for the transitions
    tr_permutations: Vec<TransitionBinary>,
    /// The (4n+1) permutations for the transitions per field. Can be reduced to eligible ones.
    tr_permutations_field: [Vec<TransitionBinary>; NUM_FIELDS],
    /// The transition set for one Turning machine. Will be adjusted each round and copied to the machine permutation.
    machine: MachineBinary,
    /// Number of used fields in the transition table (including the 2 empty fields for dummy state 0).
    n_fields: usize,
    /// Stores the id of the current transition permutation for the corresponding transition field.
    fields: [usize; NUM_FIELDS],
    field_no: usize,
    /// Sets if the first field A0 is rotated first (then A1, B0, B1, C0 etc.) or
    /// the last field (BB5: E1, then E0, D1, D0, C1 etc.)
    gen_type: EnumeratorType,

    // reduced only
    id_batch_last: u64,
    pre_decider_count_batch: Option<PreDeciderCount>,
    #[cfg(feature = "bb_enumerator_longest_skip_chain")]
    longest_skip_chain: Counter,
}

impl EnumeratorBinary {
    /// Creates a new enumerator \
    pub fn new(enumeration_type: EnumeratorType, config: &Config) -> Self {
        let n_states = config.n_states();
        assert!(n_states <= 7, "This enumerator can not create all permutations for {n_states} states as this would exceed u64:MAX permutations.");

        let n_fields = n_states * 2 + 2;
        let tr_permutations = TransitionBinary::create_all_transition_permutations(n_states);
        let mut transition_table = MachineBinary::new_default(n_states);
        // set all in set to the first variant
        transition_table.transitions[2..n_fields].fill(tr_permutations[0]);

        // special logic for reduced backward
        let tr_permutations_field;
        let ids_skip_start: u64;
        // let batch_no_skip;
        let n_machines = num_turing_machine_permutations(n_states) as u64;
        let mut n_machines_to_process = n_machines;
        let mut fields = [0; NUM_FIELDS];
        match enumeration_type {
            EnumeratorType::EnumeratorFullForward
            | EnumeratorType::EnumeratorFullBackward
            | EnumeratorType::EnumeratorReducedForward => {
                ids_skip_start = 0;
                // batch_no_skip = 0;
                tr_permutations_field = TR_PERMUTATIONS_FIELD_DEFAULT;
            }
            EnumeratorType::EnumeratorReducedBackwardNotWorking => {
                // id must jump 2 to 0RB and then skips the whole tree
                // permutations in a normal field: (4 * n_states + 1), e.g. 17 for BB4
                // first field of 2n_states fields, so 2n-1 not created.
                ids_skip_start = 2 * (4 * n_states as u64 + 1).pow(2 * n_states as u32 - 1);
                tr_permutations_field =
                    Self::create_all_transition_permutations_for_fields(n_states, &tr_permutations);
                transition_table.transitions[2] = tr_permutations_field[2][2];
                fields[2] = 2;
                // number eliminated in A0: (4 * n_states as u64 + 1 - 2)
                // This formula is correct but it will result in ids_skip_start because 2 are skipped and 2 are processed.
                // n_machines_to_process -= (4 * n_states as u64 - 1)
                //     * (4 * n_states as u64 + 1).pow(2 * n_states as u32 - 1);
                n_machines_to_process = ids_skip_start;
                // batch_no_skip = 0;
            }
            EnumeratorType::EnumeratorTNF => panic!("wrong call"),
        }
        // if gen_type == EnumeratorType::EnumeratorReducedBackward {
        //     // id must jump 2 to 0RB and then skips the whole tree
        //     // permutations in a normal field: (4 * n_states + 1), e.g. 17 for BB4
        //     // first field of 2n_states fields, so 2n-1 not created.
        //     // id_next = 2 * (4 * n_states as u64 + 1).pow(2 * n_states as u32 - 1);
        //     tr_permutations_field =
        //         Self::create_all_transition_permutations_for_fields(n_states, &tr_permutations);
        //     transition_table.transitions[2] = tr_permutations_field[2][0];
        //     // number eliminated in A0: (4 * n_states as u64 + 1 - 2)
        //     n_machines -=
        //         (4 * n_states as u64 - 1) * (4 * n_states as u64 + 1).pow(2 * n_states as u32 - 1);
        //     batch_no_skip = 0;
        // } else {
        //     // id_next = 0;
        //     batch_no_skip = 0;
        //     tr_permutations_field = TR_PERMUTATIONS_FIELD_DEFAULT;
        // };

        // limit
        let limit_id = if config.machines_limit() > 0 {
            let limit = config.machines_limit().min(n_machines);
            if limit < n_machines {
                if enumeration_type != EnumeratorType::EnumeratorReducedBackwardNotWorking {
                    n_machines_to_process = limit;
                    limit
                } else {
                    limit + ids_skip_start
                }
            } else {
                limit
            }
        } else {
            n_machines
        };

        // batch size
        let batch_size = Self::calc_batch_size(
            config.enumerator_full_batch_size_request(),
            n_states,
            n_machines as u128,
        );
        let num_batches = n_machines_to_process.div_ceil(batch_size as u64) as usize;

        Self {
            id_next: ids_skip_start,
            ids_skip_start,
            batch_no: 0,
            // batch_no_skip,
            n_machines,
            n_machines_to_process,
            batch_size,
            num_batches,
            limit_id,
            tr_permutations,
            tr_permutations_field,
            machine: transition_table,
            n_fields,
            fields,
            field_no: match enumeration_type {
                EnumeratorType::EnumeratorFullForward
                | EnumeratorType::EnumeratorReducedForward => 4,
                EnumeratorType::EnumeratorFullBackward
                | EnumeratorType::EnumeratorReducedBackwardNotWorking => n_fields - 3,
                EnumeratorType::EnumeratorTNF => panic!("wrong call"),
            },
            n_states,
            gen_type: enumeration_type,

            id_batch_last: 0,
            pre_decider_count_batch: Default::default(),
            #[cfg(feature = "bb_enumerator_longest_skip_chain")]
            longest_skip_chain: Default::default(),
        }
    }

    fn calc_batch_init(&mut self, batch_no: usize) {
        self.batch_no = batch_no;
        self.id_next = batch_no as u64 * self.batch_size as u64 + self.ids_skip_start;
        // fields 0 and 1 are unused
        // fields 2 and 3 will be 0 as this is guaranteed by the batch size (status A always fully permuted)
        // calculating the remaining fields
        let permutations = (4 * self.n_states + 1) as u64;
        let mut remain = self.id_next / (permutations * permutations);
        // reset transitions, only the used ones will be filled in the following loop, possibly redundant
        self.machine.transitions[2..self.n_states * 2 + 2].fill(self.tr_permutations[0]);
        match self.gen_type {
            EnumeratorType::EnumeratorFullForward | EnumeratorType::EnumeratorReducedForward => {
                // field_no is always 4 or n-3
                let mut i = 4;
                loop {
                    if remain > 0 {
                        let m = remain % permutations;
                        self.fields[i] = m as usize;
                        self.machine.transitions[i] = self.tr_permutations[self.fields[i]];
                        i += 1;
                        remain = (remain - m) / permutations;
                    } else {
                        break;
                    }
                }
            }
            EnumeratorType::EnumeratorFullBackward
            | EnumeratorType::EnumeratorReducedBackwardNotWorking => {
                let mut i = self.n_fields - 3;
                loop {
                    if remain > 0 {
                        let m = remain % permutations;
                        self.fields[i] = m as usize;
                        self.machine.transitions[i] = self.tr_permutations[self.fields[i]];
                        i -= 1;
                        remain = (remain - m) / permutations;
                    } else {
                        break;
                    }
                }
            }
            EnumeratorType::EnumeratorTNF => panic!("wrong call"),
        }
    }

    /// Returns the next batch of permutations and an info if this is the last batch.
    /// This is the core logic of the enumerator.
    fn enumerate_full_permutation_batch_next_forward(&mut self) -> (Vec<MachineId>, bool) {
        // if self.id_next >= self.n_machines {
        //     return (Vec::new(), true);
        // }

        // let mut is_last_batch = false;
        let mut permutations = Vec::with_capacity(self.batch_size);
        let num_tr_permutations = self.tr_permutations.len();
        // 'creation: loop {
        loop {
            // permutations state A
            // loop all transitions for first state and its two symbols
            let mut id = self.id_next;
            for v1 in self.tr_permutations.iter() {
                self.machine.transitions[3] = *v1;
                for v0 in self.tr_permutations.iter() {
                    self.machine.transitions[2] = *v0;
                    let permutation = MachineId::new_no_id(self.machine);
                    permutations.push(permutation);
                    id += 1;
                    if id == self.limit_id {
                        // total maximum reached
                        self.id_next = id;
                        return (permutations, true);
                    }
                }
            }
            self.id_next = id;

            // update line two, permutations state B (still separate for performance)
            self.fields[4] += 1;
            if self.fields[4] < num_tr_permutations {
                self.machine.transitions[4] = self.tr_permutations[self.fields[4]];
            } else {
                // set field back to first option
                self.fields[4] = 0;
                self.machine.transitions[4] = self.tr_permutations[0];
                'outer: loop {
                    self.field_no += 1;
                    self.fields[self.field_no] += 1;
                    if self.fields[self.field_no] < num_tr_permutations {
                        // set next field with next permutation
                        self.machine.transitions[self.field_no] =
                            self.tr_permutations[self.fields[self.field_no]];
                        loop {
                            self.field_no -= 1;
                            if self.field_no == 4 {
                                break 'outer;
                            } else {
                                // set previous field back to first option
                                self.fields[self.field_no] = 0;
                                self.machine.transitions[self.field_no] = self.tr_permutations[0];
                            }
                        }
                    }
                }
            }
            if permutations.len() == self.batch_size {
                break;
            }
        }

        (permutations, false)
    }

    /// Returns the next batch of permutations and an info if this is the last batch.
    /// This is the core logic of the enumerator.
    fn enumerate_full_permutation_batch_next_backward(&mut self) -> (Vec<MachineId>, bool) {
        // if self.id_next >= self.n_machines {
        //     return (Vec::new(), true);
        // }
        // println!("Id: ", self.id_next);
        // println!("Tr: {}", self.transition_table);

        // let mut is_last_batch = false;
        let mut permutations = Vec::with_capacity(self.batch_size);
        let num_tr_permutations = self.tr_permutations.len();
        let first = self.n_fields - 1;
        let third = self.n_fields - 3;
        loop {
            // permutations state E
            // loop all transitions for last state and its two symbols
            let mut id = self.id_next;
            for v1 in self.tr_permutations.iter() {
                self.machine.transitions[first - 1] = *v1;
                for v0 in self.tr_permutations.iter() {
                    self.machine.transitions[first] = *v0;
                    let permutation = MachineId::new_no_id(self.machine);
                    permutations.push(permutation);
                    id += 1;
                    if id == self.limit_id {
                        // total maximum reached
                        self.id_next = id;
                        return (permutations, true);
                    }
                }
            }
            self.id_next = id;

            // update line two, permutations state D for BB5 (still separate for performance)
            self.fields[third] += 1;
            if self.fields[third] < num_tr_permutations {
                self.machine.transitions[third] = self.tr_permutations[self.fields[third]];
            } else {
                // set field back to first option
                self.fields[third] = 0;
                self.machine.transitions[third] = self.tr_permutations[0];
                'outer: loop {
                    self.field_no -= 1;
                    self.fields[self.field_no] += 1;
                    if self.fields[self.field_no] < num_tr_permutations {
                        // set next field with next permutation
                        self.machine.transitions[self.field_no] =
                            self.tr_permutations[self.fields[self.field_no]];
                        loop {
                            self.field_no += 1;
                            if self.field_no == third {
                                break 'outer;
                            } else {
                                // set previous field back to first option
                                self.fields[self.field_no] = 0;
                                self.machine.transitions[self.field_no] = self.tr_permutations[0];
                            }
                        }
                    }
                }
            }
            if permutations.len() == self.batch_size {
                break;
            }
        }

        (permutations, false)
    }

    /// Returns the next batch of permutations and an info if this is the last batch.
    fn enumerate_reduced_permutation_batch_next_forward(&mut self) -> (Vec<MachineId>, bool) {
        if self.n_states == 1 {
            return (machines_for_n_states_1(), true);
        }
        // if self.id_next >= self.n_machines {
        //     return (Vec::new(), true);
        // }
        // println!("Enumerator reduced: is_next = {}", self.id_next);
        self.id_batch_last = (self.id_next + self.batch_size as u64 - 1).min(self.limit_id - 1);
        let mut pre_decider_count_batch = PreDeciderCount::default();

        let mut permutations = Vec::with_capacity(self.batch_size);
        let num_tr_permutations = self.tr_permutations.len();
        let ids_left_out = self.tr_permutations.len() as u64 - 2;
        let mut num_hold_a1;
        let range_3_to_n_states = 4..self.n_states * 2 + 2;
        let mut num_hold_other_lines =
            count_hold_transitions(&self.machine.transitions[range_3_to_n_states.clone()]);
        loop {
            // permutations state A
            // loop all transitions for first state and its two symbols
            // id must jump 2 to 0RB
            let mut id = self.id_next + 2;
            for v1 in self.tr_permutations.iter() {
                self.machine.transitions[3] = *v1;
                num_hold_a1 = v1.is_halt() as usize;
                for v0 in TRANSITIONS_FOR_A0.iter() {
                    self.machine.transitions[2] = *v0;
                    // let mut permutation = Machine::new(id, self.transition_table);
                    // There is no hold in A0
                    if num_hold_a1 + num_hold_other_lines != 1 {
                        pre_decider_count_batch.num_not_exactly_one_halt_condition += 1;
                        #[cfg(feature = "bb_enumerator_longest_skip_chain")]
                        self.longest_skip_chain.add_counter(
                            &permutation,
                            PreDeciderReason::NotExactlyOneHaltCondition,
                        );
                    } else {
                        // run pre-decider check
                        let check_pre = self.check_pre_decider();
                        #[cfg(feature = "bb_enumerator_longest_skip_chain")]
                        match check_pre {
                            PreDeciderReason::None => {
                                self.longest_skip_chain.update_max(id - 1);
                                if self.longest_skip_chain.counter > 1000 {
                                    println!(
                                        "Found chain: {}, {}",
                                        self.longest_skip_chain.counter,
                                        self.longest_skip_chain.machines_max_to_string(4)
                                    );
                                }
                                self.longest_skip_chain.reset_counter();
                            }
                            _ => {
                                self.longest_skip_chain.add_counter(&permutation, check_pre);
                                println!("Pre: {id}: {}", permutation.to_standard_tm_text_format())
                            }
                        }
                        match check_pre {
                            // store machine only in this case
                            PreDeciderReason::None => {
                                let mut permutation = self.machine;
                                permutation.has_self_referencing_transition_store_result();
                                permutations.push(MachineId::new_no_id(permutation));
                                #[cfg(feature = "bb_print_non_pre_perm")]
                                println!(
                                    "Perm: {id}: {}",
                                    permutation.to_standard_tm_text_format()
                                );
                            }
                            PreDeciderReason::NotAllStatesUsed => {
                                pre_decider_count_batch.num_not_all_states_used += 1;
                            }
                            PreDeciderReason::NotExactlyOneHaltCondition => {
                                pre_decider_count_batch.num_not_exactly_one_halt_condition += 1;
                            }
                            PreDeciderReason::OnlyOneDirection => {
                                pre_decider_count_batch.num_only_one_direction += 1;
                            }
                            PreDeciderReason::SimpleStartCycle => {
                                pre_decider_count_batch.num_simple_start_cycle += 1;
                            }
                            PreDeciderReason::StartRecursive => {
                                // This one does not happen here, it is included in "not enumerated".
                                pre_decider_count_batch.num_start_recursive += 1;
                            }
                            PreDeciderReason::NotStartStateBRight => {
                                // This one does not happen here, it is included in "not enumerated".
                                pre_decider_count_batch.num_not_start_state_b_right += 1;
                            }
                            PreDeciderReason::WritesOnlyZero => {
                                pre_decider_count_batch.num_writes_only_zero += 1;
                            }
                        }
                    }
                    id += 1;
                    if id == self.limit_id {
                        // total maximum reached
                        self.id_next = id;
                        // not enumerated = size of batch - enumerated permutations - eliminated permutations
                        pre_decider_count_batch.num_not_enumerated = self.limit_id
                            - self.id_batch_start()
                            - permutations.len() as u64
                            - pre_decider_count_batch.num_total();
                        self.pre_decider_count_batch = Some(pre_decider_count_batch);
                        return (permutations, true);
                    }
                }
                // adjust id back to normal count
                id += ids_left_out;
                if id >= self.limit_id {
                    // total maximum reached
                    self.id_next = id;
                    // not enumerated = size of batch - enumerated permutations - eliminated permutations
                    pre_decider_count_batch.num_not_enumerated = self.limit_id
                        - self.id_batch_start()
                        - permutations.len() as u64
                        - pre_decider_count_batch.num_total();
                    self.pre_decider_count_batch = Some(pre_decider_count_batch);
                    return (permutations, true);
                }
            }
            // subtract jump to 0RB
            self.id_next = id - 2;

            // update line two, permutations state B (still separate for performance)
            self.fields[4] += 1;
            if self.fields[4] < num_tr_permutations {
                self.machine.transitions[4] = self.tr_permutations[self.fields[4]];
            } else {
                // set field back to first option
                self.fields[4] = 0;
                self.machine.transitions[4] = self.tr_permutations[0];
                'outer: loop {
                    self.field_no += 1;
                    self.fields[self.field_no] += 1;
                    if self.fields[self.field_no] < num_tr_permutations {
                        // set next field with next permutation
                        self.machine.transitions[self.field_no] =
                            self.tr_permutations[self.fields[self.field_no]];
                        loop {
                            self.field_no -= 1;
                            if self.field_no == 4 {
                                break 'outer;
                            } else {
                                // set previous field back to first option
                                self.fields[self.field_no] = 0;
                                self.machine.transitions[self.field_no] = self.tr_permutations[0];
                            }
                        }
                    }
                }
            }
            if id >= self.id_batch_last {
                break;
            }
            let tr3_used = &self.machine.transitions[range_3_to_n_states.clone()];
            num_hold_other_lines = count_hold_transitions(tr3_used);
        }

        pre_decider_count_batch.num_not_enumerated =
            (self.batch_size - permutations.len()) as u64 - pre_decider_count_batch.num_total();
        self.pre_decider_count_batch = Some(pre_decider_count_batch);

        (permutations, false)
    }

    fn enumerate_reduced_permutation_batch_next_backward(&mut self) -> (Vec<MachineId>, bool) {
        if self.n_states == 1 {
            return (machines_for_n_states_1(), true);
        }
        // if self.id_next >= self.n_machines {
        //     return (Vec::new(), true);
        // }
        self.id_batch_last = (self.id_next + self.batch_size as u64 - 1).min(self.limit_id - 1);
        let mut pre_decider_count_batch = PreDeciderCount::default();

        let mut permutations = Vec::with_capacity(self.batch_size);
        let first = self.n_fields - 1;
        let third = self.n_fields - 3;
        let mut num_hold_e0;
        let range_count_hold = 3..self.n_states * 2;
        let mut num_hold_other_lines =
            count_hold_transitions(&self.machine.transitions[range_count_hold.clone()]);
        loop {
            // Last state is assumed to be E for the comments, start remains at A0.
            // Also the reduced number of stated for A0 remains 0RB and 1RB
            // permutations state E
            // loop all transitions for last state
            // id must jump 2 to 0RB
            // TODO
            let mut id = self.id_next;
            for v1 in self.tr_permutations_field[first - 1].iter() {
                self.machine.transitions[first - 1] = *v1;
                num_hold_e0 = v1.is_halt() as usize;
                for v0 in self.tr_permutations_field[first].iter() {
                    self.machine.transitions[first] = *v0;
                    // let mut permutation = Machine::new(id, self.transition_table);
                    // if id == 2154 {
                    //     println!()
                    // }
                    // Check exactly one hold condition
                    if v0.is_halt() as usize + num_hold_e0 + num_hold_other_lines != 1 {
                        pre_decider_count_batch.num_not_exactly_one_halt_condition += 1;
                        #[cfg(feature = "bb_enumerator_longest_skip_chain")]
                        self.longest_skip_chain.add_counter(
                            &permutation,
                            PreDeciderReason::NotExactlyOneHaltCondition,
                        );
                    } else {
                        // run pre-decider check
                        let check_pre = self.check_pre_decider();
                        #[cfg(feature = "bb_enumerator_longest_skip_chain")]
                        match check_pre {
                            PreDeciderReason::None => {
                                self.longest_skip_chain.update_max(id - 1);
                                if self.longest_skip_chain.counter > 1000 {
                                    println!(
                                        "Found chain: {}, {}",
                                        self.longest_skip_chain.counter,
                                        self.longest_skip_chain.machines_max_to_string(4)
                                    );
                                }
                                self.longest_skip_chain.reset_counter();
                            }
                            _ => {
                                self.longest_skip_chain.add_counter(&permutation, check_pre);
                                println!("Pre: {id}: {}", permutation.to_standard_tm_text_format())
                            }
                        }
                        match check_pre {
                            // store machine only in this case
                            PreDeciderReason::None => {
                                let mut permutation = self.machine;
                                permutation.has_self_referencing_transition_store_result();
                                permutations.push(MachineId::new_no_id(permutation));
                                #[cfg(feature = "bb_print_non_pre_perm")]
                                println!(
                                    "Perm: {id}: {}",
                                    permutation.to_standard_tm_text_format()
                                );
                            }
                            PreDeciderReason::NotAllStatesUsed => {
                                pre_decider_count_batch.num_not_all_states_used += 1;
                            }
                            PreDeciderReason::NotExactlyOneHaltCondition => {
                                pre_decider_count_batch.num_not_exactly_one_halt_condition += 1;
                            }
                            PreDeciderReason::OnlyOneDirection => {
                                pre_decider_count_batch.num_only_one_direction += 1;
                            }
                            PreDeciderReason::SimpleStartCycle => {
                                pre_decider_count_batch.num_simple_start_cycle += 1;
                            }
                            PreDeciderReason::StartRecursive => {
                                // This one does not happen here, it is included in "not enumerated".
                                pre_decider_count_batch.num_start_recursive += 1;
                            }
                            PreDeciderReason::NotStartStateBRight => {
                                // This one does not happen here, it is included in "not enumerated".
                                pre_decider_count_batch.num_not_start_state_b_right += 1;
                            }
                            PreDeciderReason::WritesOnlyZero => {
                                pre_decider_count_batch.num_writes_only_zero += 1;
                            }
                        }
                    }
                    id += 1;
                    if id == self.limit_id {
                        // total maximum reached
                        self.id_next = id;
                        pre_decider_count_batch.num_not_enumerated = self.limit_id
                            - self.id_batch_start()
                            - permutations.len() as u64
                            - pre_decider_count_batch.num_total();
                        self.pre_decider_count_batch = Some(pre_decider_count_batch);
                        return (permutations, true);
                    }
                }
                // id += ids_left_out;
                // if id >= self.limit {
                //     // total maximum reached
                //     self.id_next = id;
                //     self.pre_decider_count_batch.num_not_enumerated = self.limit
                //         - self.id_batch_start()
                //         - permutations.len() as u64
                //         - self.pre_decider_count_batch.num_total();
                //     return (permutations, true);
                // }
            }
            // TODO subtract jump to 0RB
            self.id_next = id;

            // update line two, permutations state D (still separate for performance)
            self.fields[third] += 1;
            if self.fields[third] < self.tr_permutations_field[third].len() {
                self.machine.transitions[third] =
                    self.tr_permutations_field[third][self.fields[third]];
            } else {
                // set field back to first option
                self.fields[third] = 0;
                self.machine.transitions[third] = self.tr_permutations_field[third][0];
                'outer: loop {
                    self.field_no -= 1;
                    if self.field_no < 2 {
                        // Ends here
                        // add remaining ids for this batch
                        self.id_next = self.id_batch_last + 1;
                        pre_decider_count_batch.num_not_enumerated = self.limit_id
                            - self.id_batch_start()
                            - permutations.len() as u64
                            - pre_decider_count_batch.num_total();
                        self.pre_decider_count_batch = Some(pre_decider_count_batch);
                        return (permutations, true);
                    }
                    self.fields[self.field_no] += 1;
                    if self.fields[self.field_no] < self.tr_permutations_field[self.field_no].len()
                    {
                        // set next field with next permutation
                        self.machine.transitions[self.field_no] =
                            self.tr_permutations_field[self.field_no][self.fields[self.field_no]];
                        loop {
                            self.field_no += 1;
                            if self.field_no == third {
                                break 'outer;
                            } else {
                                // set previous field back to first option
                                self.fields[self.field_no] = 0;
                                self.machine.transitions[self.field_no] =
                                    self.tr_permutations_field[self.field_no][0];
                            }
                        }
                    }
                }
            }
            if id >= self.id_batch_last {
                break;
            }
            let tr3_used = &self.machine.transitions[range_count_hold.clone()];
            num_hold_other_lines = count_hold_transitions(tr3_used);
        }

        pre_decider_count_batch.num_not_enumerated =
            (self.batch_size - permutations.len()) as u64 - pre_decider_count_batch.num_total();
        self.pre_decider_count_batch = Some(pre_decider_count_batch);

        (permutations, false)
    }

    #[inline]
    pub fn check_pre_decider(&self) -> PreDeciderReason {
        let tr_used = self.machine.transitions_used(self.n_states);
        if check_only_right_direction(tr_used) {
            return PreDeciderReason::OnlyOneDirection;
        }
        if check_only_zero_writes(tr_used) {
            return PreDeciderReason::WritesOnlyZero;
        }
        if check_not_all_states_used(&self.machine, self.n_states) {
            return PreDeciderReason::NotAllStatesUsed;
        }
        if check_simple_start_cycle(&self.machine) {
            return PreDeciderReason::SimpleStartCycle;
        }

        PreDeciderReason::None
    }

    fn create_all_transition_permutations_for_fields(
        n_states: usize,
        tr_permutations: &[TransitionBinary],
    ) -> [Vec<TransitionBinary>; NUM_FIELDS] {
        let mut tr_permutations_field = TR_PERMUTATIONS_FIELD_DEFAULT;
        // tr_permutations_field[2] = TRANSITIONS_FOR_A0.to_vec();
        for i in 2..n_states * 2 + 2 {
            tr_permutations_field[i] = tr_permutations.to_vec();
        }
        tr_permutations_field[2].truncate(4);

        tr_permutations_field
    }

    fn id_batch_start(&self) -> u64 {
        self.id_batch_last / self.batch_size as u64 * self.batch_size as u64
    }
}

impl Enumerator for EnumeratorBinary {
    /// Returns the next batch of permutations and an info if this is the last batch.
    fn enumerate_permutation_batch_no(&mut self, batch_no: usize) -> (Vec<MachineId>, bool) {
        self.calc_batch_init(batch_no);
        self.enumerate_permutation_batch_next()
    }

    /// Returns the next batch of permutations and an info if this is the last batch.
    /// This is the core logic of the enumerator.
    fn enumerate_permutation_batch_next(&mut self) -> (Vec<MachineId>, bool) {
        let r = match self.gen_type {
            EnumeratorType::EnumeratorFullForward => {
                self.enumerate_full_permutation_batch_next_forward()
            }
            EnumeratorType::EnumeratorFullBackward => {
                self.enumerate_full_permutation_batch_next_backward()
            }
            EnumeratorType::EnumeratorReducedForward => {
                self.enumerate_reduced_permutation_batch_next_forward()
            }
            EnumeratorType::EnumeratorReducedBackwardNotWorking => {
                self.enumerate_reduced_permutation_batch_next_backward()
            }
            EnumeratorType::EnumeratorTNF => panic!("wrong call"),
        };
        self.batch_no += 1;

        r
    }

    /// The given limit of machines to enumerate or (if smaller) the maximum number of machines for the number of states.
    fn limit(&self) -> u64 {
        self.limit_id
    }

    fn pre_decider_count(&self) -> PreDeciderCount {
        match self.pre_decider_count_batch {
            Some(p) => p,
            None => Default::default(),
        }
    }

    // fn check_enumerator_batch_size_request_single_thread(&mut self) {
    //     if self.batch_size > BATCH_SIZE_REQUEST_SINGLE_THREAD_MAX {
    //         // self.config.enumerator_batch_size_request_full = BATCH_SIZE_REQUEST_SINGLE_THREAD_MAX;
    //         let batch_size =
    //             Self::calc_batch_size(BATCH_SIZE_REQUEST_SINGLE_THREAD_MAX, self.n_states);
    //         // self.num_batches = ((self.limit + batch_size as u64 - 1) / batch_size as u64) as usize;
    //         self.num_batches = self.limit.div_ceil(batch_size as u64) as usize;
    //         self.batch_size = batch_size;
    //     }
    // }

    fn num_eliminated(&self) -> u64 {
        self.pre_decider_count().num_total()
    }
}

impl DataProvider for EnumeratorBinary {
    fn name(&self) -> &str {
        // TODO name for each variant
        "Enumerator Full"
    }

    fn machine_batch_next(&mut self) -> ResultDataProvider {
        let (machines, is_last_batch) = self.enumerate_permutation_batch_next();
        let end_reason = if is_last_batch {
            EndReason::IsLastBatch
        } else {
            EndReason::None
        };
        Ok(DataProviderBatch {
            // batch no is already set to next batch
            batch_no: self.batch_no - 1,
            machines,
            pre_decider_count: self.pre_decider_count_batch,
            end_reason,
        })
    }

    fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// The total number of batches to create all permutations.
    fn num_batches(&self) -> usize {
        self.num_batches
    }

    fn num_machines_to_process(&self) -> u64 {
        self.limit_id
        // self.n_machines_to_process
    }

    fn requires_pre_decider_check(&self) -> PreDeciderRun {
        match self.gen_type {
            EnumeratorType::EnumeratorFullForward | EnumeratorType::EnumeratorFullBackward => {
                PreDeciderRun::RunStartBRightOnly
            }
            EnumeratorType::EnumeratorReducedForward
            | EnumeratorType::EnumeratorReducedBackwardNotWorking => PreDeciderRun::DoNotRun,
            EnumeratorType::EnumeratorTNF => panic!("wrong call"),
        }
    }

    // fn returns_pre_decider_count(&self) -> bool {
    //     false
    // }

    // fn set_batch_size_for_num_threads(&mut self, num_threads: usize) {
    //     // TODO fine tune
    //     if num_threads == 1 {
    //         self.check_enumerator_batch_size_request_single_thread();
    //     }
    // }
}

impl DataProviderThreaded for EnumeratorBinary {
    fn new_from_data_provider(&self) -> Self {
        let mut transition_table = MachineBinary::new_default(self.n_states);
        // set all in set to the first variant
        transition_table.transitions[2..2 + self.n_states * 2].fill(self.tr_permutations[0]);

        Self {
            id_next: 0,
            ids_skip_start: self.ids_skip_start,
            batch_no: 0,
            // batch_no_skip: self.batch_no_skip,
            n_machines: self.n_machines,
            n_machines_to_process: self.n_machines_to_process,
            batch_size: self.batch_size,
            limit_id: self.limit_id,
            num_batches: self.num_batches,
            tr_permutations: self.tr_permutations.clone(),
            tr_permutations_field: self.tr_permutations_field.clone(),
            machine: transition_table,
            n_fields: self.n_fields,
            fields: [0; NUM_FIELDS],
            field_no: match self.gen_type {
                EnumeratorType::EnumeratorFullForward
                | EnumeratorType::EnumeratorReducedForward => 4,
                EnumeratorType::EnumeratorFullBackward
                | EnumeratorType::EnumeratorReducedBackwardNotWorking => self.n_fields - 3,
                EnumeratorType::EnumeratorTNF => panic!("wrong call"),
            },
            n_states: self.n_states,
            gen_type: self.gen_type,

            id_batch_last: 0,
            pre_decider_count_batch: Default::default(),
            #[cfg(feature = "bb_enumerator_longest_skip_chain")]
            longest_skip_chain: Default::default(),
        }
    }

    fn batch_no(&mut self, batch_no: usize) -> DataProviderBatch {
        let (machines, is_last_batch) = self.enumerate_permutation_batch_no(batch_no);
        let end_reason = if is_last_batch {
            EndReason::IsLastBatch
        } else {
            EndReason::None
        };
        DataProviderBatch {
            batch_no: self.batch_no,
            machines,
            pre_decider_count: self.pre_decider_count_batch,
            end_reason,
        }
    }
}

/// Compares sequential batches with batch_no generation
pub fn validate_next_with_batch_no() {
    let n_states = 3;
    let config = Config::builder(n_states)
        .enumerator_full_batch_size_request(100000)
        // .enumerator_first_rotate_field_front(true)
        .build();
    let mut enumerator_next =
        EnumeratorBinary::new(EnumeratorType::EnumeratorReducedBackwardNotWorking, &config);
    let mut enumerator_batch_no =
        EnumeratorBinary::new(EnumeratorType::EnumeratorReducedBackwardNotWorking, &config);
    // let (_m_no, _is_finished) = enumerator_batch_no.enumerate_permutation_batch_no(484);

    println!("Machines: {}", enumerator_next.n_machines);

    let mut batch_no = 0;
    let mut counter = 0;
    loop {
        let (m_next, is_finished) = enumerator_next.enumerate_permutation_batch_next();
        // delete known transition data to force update
        for i in 2..8 {
            enumerator_batch_no.machine.transitions[i].transition = 0;
        }
        let (m_no, _is_finished) = enumerator_batch_no.enumerate_permutation_batch_no(batch_no);
        println!(
            "batch_no {}, size next{}, size batch_no {}",
            batch_no + 1,
            m_next.len(),
            m_no.len()
        );
        assert_eq!(m_next.len(), m_no.len());

        for (i, m) in m_next.iter().enumerate() {
            let mv = &m_no[i];
            counter += 1;
            assert_eq!(m, mv);
        }

        if is_finished {
            println!(
                "counted: {counter} of {} machines",
                enumerator_next.n_machines
            );
            // assert_eq!(counter, enumerator_next.n_machines);
            break;
        }

        batch_no += 1;
    }
    // let result = batch_run_decider_chain_data_provider_single_thread(&vec![dc], enumerator);
    // println!("{}", result);
    // println!("{}", result.machines_max_steps_to_string(10));
    // assert_eq!(result_max_steps_known(n_states), result.steps_max());
    println!();
}

/// run this only in release mode from command line: \
/// cargo test --release enumerator_full
#[cfg(test)]
mod tests {
    use crate::{
        decider::decider_engine::{
            batch_run_decider_chain_data_provider_single_thread,
            batch_run_decider_chain_threaded_data_provider_multi_thread,
        },
        decider::decider_result::result_max_steps_known,
        decider::DeciderStandard,
    };

    use super::*;

    const GEN_TYPE: EnumeratorType = EnumeratorType::EnumeratorFullBackward;

    #[test]
    fn decider_enumerator_full_bb2() {
        run_test_decider_enumerator_full(2);
    }

    #[test]
    fn decider_enumerator_full_bb3() {
        run_test_decider_enumerator_full(3);
    }

    /// run this only in release mode from command line: \
    /// cargo test --release test_decider_enumerator_full_bb4
    #[test]
    fn decider_enumerator_full_bb4() {
        run_test_decider_enumerator_full(4);
    }

    /// run this only in release mode from command line: \
    /// cargo test --release test_decider_enumerator_full_threaded_bb4
    #[test]
    fn decider_enumerator_full_bb4_threaded() {
        run_test_decider_enumerator_full_threaded(4);
    }

    #[test]
    fn enumerator_full_direct_access_batch_no() {
        let config = Config::builder(4)
            .enumerator_full_batch_size_request(10_000)
            .machine_limit(10_000_000)
            .build();
        let m1;
        let mut batch_no = 0;
        let mut g = EnumeratorBinary::new(GEN_TYPE, &config);
        loop {
            let (vm, is_finished) = g.enumerate_permutation_batch_next();
            if is_finished {
                m1 = vm.first().unwrap().clone();
                break;
            }
            batch_no += 1;
        }

        // check direct batch access
        let mut g = EnumeratorBinary::new(GEN_TYPE, &config);
        let (vm, _) = g.enumerate_permutation_batch_no(batch_no);
        let m2 = vm.first().unwrap().clone();
        assert_eq!(m1, m2);
        // println!("m1: {}", m1);

        // check exact permutation
        if GEN_TYPE == EnumeratorType::EnumeratorFullForward {
            // let id = 9_993_042;
            let mut transitions: Vec<(&str, &str)> = Vec::new();
            transitions.push(("0RA", "0RA"));
            transitions.push(("0RA", "1LB"));
            transitions.push(("0RA", "1RD"));
            transitions.push(("0RA", "0RA"));
            let m = MachineBinary::from_string_tuple(&transitions);
            assert_eq!(*m1.machine(), m);
        }
    }

    #[test]
    fn enumerator_full_direct_access_batch_no_all() {
        let config = Config::builder(3)
            .enumerator_full_batch_size_request(10_000)
            // .machine_limit(10_000_000)
            .machine_limit(0)
            .build();
        let mut m1;
        let mut batch_no = 0;
        let mut g1 = EnumeratorBinary::new(GEN_TYPE, &config);
        let mut g2 = EnumeratorBinary::new(GEN_TYPE, &config);
        loop {
            let (vm1, is_finished) = g1.enumerate_permutation_batch_next();
            m1 = vm1.first().unwrap();
            // check direct batch access
            let (vm2, _) = g2.enumerate_permutation_batch_no(batch_no);
            let m2 = vm2.first().unwrap();
            // print!("{batch_no}, ");
            assert_eq!(m1, m2);
            assert_eq!(vm1.len(), vm2.len());
            if is_finished {
                break;
            }
            batch_no += 1;
        }

        // println!("m1: {}", m1);
    }

    fn run_test_decider_enumerator_full(n_states: usize) {
        let config = config_bench(n_states);
        let dc = DeciderStandard::Cycler.decider_config(&config);
        let enumerator = EnumeratorBinary::new(GEN_TYPE, &config);
        let result = batch_run_decider_chain_data_provider_single_thread(&vec![dc], enumerator);
        println!("{}", result);
        println!("{}", result.machines_max_steps_to_string(10));
        assert_eq!(result_max_steps_known(n_states), result.steps_max());
    }

    fn run_test_decider_enumerator_full_threaded(n_states: usize) {
        let config = config_bench(n_states);
        let dc = DeciderStandard::Cycler.decider_config(&config);
        let enumerator = EnumeratorBinary::new(GEN_TYPE, &config);
        let result =
            batch_run_decider_chain_threaded_data_provider_multi_thread(&vec![dc], enumerator);
        // println!("{}", result);
        assert_eq!(result_max_steps_known(n_states), result.steps_max());
    }

    fn config_bench(n_states: usize) -> Config {
        let limit = if GEN_TYPE == EnumeratorType::EnumeratorFullForward {
            200_000_000
        } else {
            1_550_000_000
        };
        Config::builder(n_states)
            // .enumerator_batch_size_request_full(GENERATOR_BATCH_SIZE_REQUEST_FULL)
            // .enumerator_batch_size_request_reduced(GENERATOR_BATCH_SIZE_REQUEST_REDUCED)
            .machine_limit(limit)
            // .step_limit_cycler(110)
            // .cpu_utilization(100)
            .build()
    }
}
