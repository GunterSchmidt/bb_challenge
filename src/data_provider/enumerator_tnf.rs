use std::fmt::Display;

use crate::{
    config::{Config, CoreUsage, MAX_STATES},
    data_provider::{
        enumerator::{num_turing_machine_permutations, Enumerator, NUM_FIELDS},
        enumerator_binary::EnumeratorType,
        DataProvider, DataProviderBatch, ResultDataProvider,
    },
    decider::{
        decider_cycler_small::DeciderCyclerSmall,
        decider_engine,
        decider_result::{EndReason, PreDeciderCount},
        pre_decider::PreDeciderRun,
        Decider, DeciderStandard,
    },
    machine_binary::{MachineBinary, MachineId},
    transition_binary::{
        TransitionBinary, TRANSITION_BINARY_HALT, TRANSITION_BINARY_UNDEFINED,
        TRANSITION_BINARY_UNUSED,
    },
};

#[derive(Debug)]
pub struct EnumeratorTNF {
    /// The number of states used for this enumerator.
    n_states: usize,
    /// The total number of machines to be enumerated. This is the number of Turing Machines for that n.
    n_machines: u64,
    /// The limit of machines to enumerate or (if smaller) the maximum number of machines for the number of states. \
    /// For performance reasons the limit is checked only after each batch.
    limit: u64,

    /// The (4n) permutations for the transitions, kept as array on the stack, so it is in level 1 cache.
    tr_permutations: [TransitionBinary; MAX_STATES * 4],
    /// The transition set for one Turning machine. Will be adjusted each round and copied to the machine permutation.
    machine: MachineBinary,
    /// Number of used fields in the transition table (including the 2 empty fields for dummy state 0).
    n_fields: usize,
    /// Stores the id of the current transition permutation for the corresponding transition field.
    fields: [usize; NUM_FIELDS],
    field_no: usize,

    /// Batch no, increased for every call, starting with 0.
    batch_no: usize,
    /// The reduced actual batch size (number of Turing machines enumerated in each call).
    batch_size: usize,
    /// Instead of limit, which is not checked anyhow, use id of last batch to check if limit is reached.
    batch_last: usize,

    decider_cycler: DeciderCyclerSmall,
}

impl EnumeratorTNF {
    pub fn new(config: &Config) -> Self {
        let n_states = config.n_states();
        assert!(n_states <= 7, "This enumerator can not create all permutations for {n_states} states as this would exceed u64:MAX permutations.");

        let n_machines = num_turing_machine_permutations(n_states) as u64;
        let limit = config.machines_limit().min(n_machines);

        let n_fields = n_states * 2 + 2;
        let tr_permutations = Self::create_all_transition_permutations();
        let mut machine = MachineBinary::new_default(n_states);
        // set all in set to the first variant
        machine.transitions[2..n_fields].fill(TRANSITION_BINARY_UNDEFINED);

        Self {
            n_states,
            n_machines,
            limit,
            tr_permutations,
            machine,
            n_fields,
            fields: [0; NUM_FIELDS],
            field_no: 4,
            batch_no: 0,
            batch_last: 0,
            // TODO batch size
            batch_size: 10_000,
            decider_cycler: DeciderCyclerSmall::new(config),
        }
    }

    /// This creates all transition permutations for one field, e.g. \
    /// BB1: 0LA, 1LA, 0RA, 1RA \
    /// BB2: 0LA, 1LA, 0RA, 1RA, 0LB, 1LB, 0RB, 1RB \
    /// The enumeration is
    /// * each state first: A, B, C, ... (allowing to skip all following if an not yet allowed state is reached and also \
    /// have the same order for all states. This can be made into a constant.)
    /// * each direction next: L, R
    /// * symbol last: 0, 1
    /// * neither undefined or 1RZ are part of this list, as undefined is default and 1RZ is set for last transition.
    ///
    /// The number can be calculated by (4 * n_states), e.g. 20 for BB5. \
    /// Keep this order as it is required by TNF tree enumeration.
    pub fn create_all_transition_permutations() -> [TransitionBinary; MAX_STATES * 4] {
        let mut transitions = [TRANSITION_BINARY_UNUSED; MAX_STATES * 4];

        // all to left
        let mut i = 0;
        for state in 1..=MAX_STATES as u8 {
            for direction in [1u8, 0u8] {
                for symbol in [0u8, 1u8] {
                    // tr as symbol, direction, next state
                    let tr = [symbol, direction, state as u8];
                    transitions[i] = TransitionBinary::try_new(tr).unwrap();
                    i += 1;
                }
            }
        }

        transitions
    }
}

impl Enumerator for EnumeratorTNF {
    fn enumerate_permutation_batch_no(&mut self, _batch_no: usize) -> (Vec<MachineId>, bool) {
        panic!("The TNF Enumerator does not support direct batch no access.")
    }

    fn enumerate_permutation_batch_next(&mut self) -> (Vec<MachineId>, bool) {
        // if self.n_states == 1 {
        //     self.batch_no = 1;
        //     return (super::enumerator::machines_for_n_states_1(), true);
        // }
        let mut permutations = Vec::with_capacity(self.batch_size);
        // Alternative to return machines for n=1
        if self.n_states == 1 {
            self.batch_no = 1;
            self.machine.transitions[2] = TRANSITION_BINARY_HALT;
            let m = MachineId::new_no_id(self.machine);
            permutations.push(m);
            return (permutations, true);
        }

        let num_tr_permutations = self.n_states * 4;

        loop {
            // TODO faster if working with MachineBinary for small cycler
            let r = self
                .decider_cycler
                .decide_machine(&MachineId::new_no_id(self.machine));
            println!("Result Small Cycler: {r}");

            break;
        }

        self.batch_no += 1;
        (permutations, true)
    }

    fn limit(&self) -> u64 {
        self.limit
    }

    fn pre_decider_count(&self) -> PreDeciderCount {
        // PreDeciderCount::default();
        todo!()
    }

    fn num_eliminated(&self) -> u64 {
        todo!()
    }
}

impl DataProvider for EnumeratorTNF {
    fn name(&self) -> &str {
        "Enumerator TNF"
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
            // pre_decider_count: self.pre_decider_count_batch,
            // TODO count
            pre_decider_count: None,
            end_reason,
        })
    }

    fn batch_size(&self) -> usize {
        todo!()
    }

    fn num_batches(&self) -> usize {
        // TODO unclear what this controls
        self.batch_no
    }

    fn num_machines_to_process(&self) -> u64 {
        self.limit
    }

    fn requires_pre_decider_check(&self) -> PreDeciderRun {
        PreDeciderRun::DoNotRun
    }
}

impl Display for EnumeratorTNF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "n_states: {}", self.n_states)?;
        writeln!(f, "limit: {}", self.limit)?;
        let mut t = Vec::new();
        for i in 0..self.n_states * 4 {
            t.push(self.tr_permutations[i].to_string());
        }
        write!(f, "Transitions: {}", t.join(", "))
    }
}

pub fn test_enumerator_tnf_simple() {
    let n_states = 4;
    let config_1 = Config::builder(n_states)
        // 10_000_000_000 for BB4
        .machine_limit(1000_000_000_000)
        // .limit_machines_undecided(200)
        .write_html_file(true)
        .build();

    let tnf = EnumeratorTNF::new(&config_1);
    println!("Enumerator {tnf}");
}

pub fn test_enumerator_tnf() {
    let n_states = 2;
    let config_1 = Config::builder(n_states)
        // 10_000_000_000 for BB4
        .machine_limit(1000_000_000_000)
        // .limit_machines_undecided(200)
        .write_html_file(true)
        .build();

    let decider_last = 1;
    let dc_cycler_1 = DeciderStandard::Cycler.decider_config(&config_1);
    let decider_config = vec![
        dc_cycler_1,
        // dc_bouncer_1,
        // dc_cycler_2,
        // dc_hold,
    ];

    let result = decider_engine::run_decider_chain_gen(
        &decider_config[0..decider_last],
        EnumeratorType::EnumeratorTNF,
        CoreUsage::SingleCore,
    );

    println!("\n{}", result.to_string_with_duration());
}
