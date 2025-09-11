#![allow(dead_code)]

use std::{
    collections::VecDeque,
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
};

use crate::{
    config::{Config, CoreUsage, StepBig, MAX_STATES, NUM_FIELDS},
    data_provider::{
        enumerator::{num_turing_machine_permutations, Enumerator},
        enumerator_binary::EnumeratorType,
        DataProvider, DataProviderBatch, ResultDataProvider,
    },
    decider::{
        decider_bouncer_128::DeciderBouncer128,
        decider_cycler_small::DeciderCyclerSmall,
        decider_engine,
        decider_result::{EndReason, PreDeciderCount},
        pre_decider::{check_no_halt_transition, moves_only_right_status, PreDeciderRun},
        Decider, DeciderConfig, DeciderStandard,
    },
    machine_binary::{MachineBinary, MachineId},
    machine_info::MachineInfo,
    status::MachineStatus,
    transition_binary::{TransitionBinary, TRANSITION_BINARY_HALT, TRANSITION_BINARY_UNDEFINED},
};

const NUM_TR_PERMUTATIONS: usize = MAX_STATES * 4 + 1;
const PRE_CHECK: &str = "Pre-Check";
const BOUNCER_MAX_STEPS: StepBig = 300;
const CYCLER_MAX_STEPS: StepBig = 100;

#[derive(Debug)]
pub struct MachineForStack {
    machine: MachineBinary,
    field_no: usize,
    // max_state: usize,
}

// TODO If max one halt condition, then use pre-decider before using cycler. quicker decision
// Could also be interesting for cases that result in Non-Halt with the given data, e.g. only 0 or only R.
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
    tr_permutations: [TransitionBinary; NUM_TR_PERMUTATIONS],
    /// The transition set for one Turning machine. Will be adjusted each round and copied to the machine permutation.
    machine: MachineBinary,
    machine_stack: Vec<MachineBinary>,
    machines_for_stack: VecDeque<MachineForStack>,
    /// Number of used fields in the transition table (including the 2 empty fields for dummy state 0).
    n_fields: usize,
    /// Stores the id of the current transition permutation for the corresponding transition field.
    fields: [usize; NUM_FIELDS],
    fields_max_state: [usize; NUM_FIELDS],
    field_no: usize,
    last_field_id: usize,

    /// Batch no, increased for every call, starting with 0.
    batch_no: usize,
    /// The reduced actual batch size (number of Turing machines enumerated in each call).
    batch_size: usize,
    /// Instead of limit, which is not checked anyhow, use id of last batch to check if limit is reached.
    batch_last: usize,

    /// Currently known max number of steps. All machines with less steps will be disregarded.
    /// This also means this cannot be used to find machines with maximum number of ones.
    max_steps: StepBig,
    /// list of machines having max_steps
    // TODO return value
    machines_max_steps: Vec<MachineInfo>,
    undecided_count: usize,
    undecided_multiple_halts_count: usize,
    undecided_machines_multiple_halts: Vec<MachineBinary>,
    tnf_machines_count: u64,

    decider_cycler: DeciderCyclerSmall,
    decider_bouncer: DeciderBouncer128,
    #[cfg(feature = "enable_csv_for_tnf")]
    csv: CsvWriter,
    #[cfg(feature = "debug_enumerator")]
    max_for_stack: usize,
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
        let mut machine_stack = Vec::new();
        // Fill machines for field 2
        for i in 6..9 {
            if tr_permutations[i].is_dir_right() {
                machine.transitions[2] = tr_permutations[i];
                machine_stack.push(machine);
            }
        }

        #[cfg(feature = "enable_csv_for_tnf")]
        let mut csv = CsvWriter::try_new(config).expect("File Error");
        #[cfg(feature = "enable_csv_for_tnf")]
        {
            use crate::transition_binary::{TRANSITION_0RA_BINARY_FIRST, TRANSITION_1RA};

            machine.transitions[2] = TRANSITION_BINARY_UNDEFINED;
            csv.write_machine(&machine, &MachineStatus::DecidedHaltField(1, 2), PRE_CHECK);
            machine.transitions[2] = TRANSITION_0RA_BINARY_FIRST;
            csv.write_machine(&machine, &MachineStatus::DecidedHaltField(1, 2), PRE_CHECK);
            machine.transitions[2] = TRANSITION_1RA;
            csv.write_machine(&machine, &MachineStatus::DecidedHaltField(1, 2), PRE_CHECK);
        }

        let config = Config::builder_from_config(config)
            .step_limit_decider_cycler(CYCLER_MAX_STEPS)
            .step_limit_decider_bouncer(BOUNCER_MAX_STEPS)
            .build();

        Self {
            n_states,
            n_machines,
            limit,
            tr_permutations,
            machine,
            machine_stack,
            machines_for_stack: VecDeque::new(),
            n_fields,
            fields: [0; NUM_FIELDS],
            fields_max_state: [0; NUM_FIELDS],
            field_no: 4,
            last_field_id: MachineBinary::last_used_field_id_in_transition_array_exclusive(
                n_states,
            ),
            batch_no: 0,
            batch_last: 0,
            // TODO batch size
            batch_size: 10_000,
            // avoid to much useless storing of machines in the beginning
            max_steps: if n_states < 4 { 6 } else { 100 },
            machines_max_steps: Vec::new(),
            undecided_count: 0,
            undecided_multiple_halts_count: 0,
            undecided_machines_multiple_halts: Vec::new(),
            tnf_machines_count: 3,
            decider_cycler: DeciderCyclerSmall::new(&config),
            decider_bouncer: DeciderBouncer128::new(&config),

            #[cfg(feature = "enable_csv_for_tnf")]
            csv,
            #[cfg(feature = "debug_enumerator")]
            max_for_stack: 0,
        }
    }

    /// This creates all transition permutations for one field, e.g. \
    /// BB1: 0LA, 1LA, 0RA, 1RA \
    /// BB2: 0LA, 1LA, 0RA, 1RA, 0LB, 1LB, 0RB, 1RB \
    /// The enumeration is
    /// * undefined
    /// * each state first: A, B, C, ... (allowing to skip all following if an not yet allowed state is reached and also \
    /// have the same order for all states. This can be made into a constant.)
    /// * each direction next: L, R
    /// * symbol last: 0, 1
    /// * 1RZ is not part of this list, it is set for last transition.
    ///
    /// The number can be calculated by (4 * n_states + 1), e.g. 21 for BB5. \
    /// Keep this order as it is required by TNF tree enumeration.
    pub fn create_all_transition_permutations() -> [TransitionBinary; NUM_TR_PERMUTATIONS] {
        let mut transitions = [TRANSITION_BINARY_UNDEFINED; NUM_TR_PERMUTATIONS];

        // all to left
        let mut i = 1;
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

    fn moves_only_right() {}
}

impl Enumerator for EnumeratorTNF {
    fn enumerate_permutation_batch_no(&mut self, _batch_no: usize) -> (Vec<MachineId>, bool) {
        panic!("The TNF Enumerator does not support direct batch no access.")
    }

    // TODO Pre-Check: No transition going to higher levels (to a stop field), e.g.  0RB0LA_1LA1RB_------_------
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
            self.machines_max_steps.push(MachineInfo::from_machine(
                self.machine,
                MachineStatus::DecidedHalt(1),
            ));
            permutations.push(m);
            return (permutations, true);
        }

        // let check = MachineBinary::try_from("1RB0LC_1LC0RC_1LA0LC_------").unwrap();

        loop {
            // After this if self.machine must be filled with the correct machine
            if self.machine_stack.is_empty() {
                match self.machines_for_stack.pop_front() {
                    Some(mfs) => {
                        // put first machine in self.machine
                        self.machine = mfs.machine;
                        let max_state = (self.machine.max_state_used(self.last_field_id) + 1)
                            .min(self.n_states);
                        let last_perm = max_state * 4 + 1;
                        for i in 2..last_perm {
                            self.machine.transitions[mfs.field_no] = self.tr_permutations[i];
                            self.machine_stack.push(self.machine);
                        }
                        self.machine.transitions[mfs.field_no] = self.tr_permutations[1];
                    }
                    // all machines created
                    None => break,
                }
            } else {
                self.machine = self.machine_stack.remove(0);
            }

            // check machine
            self.tnf_machines_count += 1;
            let tr_used = &self.machine.transitions[2..self.last_field_id];
            let mut status = moves_only_right_status(tr_used);
            #[cfg(not(feature = "enumerator_1RB_only"))]
            if status == MachineStatus::NoDecision && self.machine.transitions[2].is_symbol_zero() {
                status = crate::decider::pre_decider::has_only_zero_writes_status(tr_used);
            }
            // check has halt in case not all states are used
            // if self.machine == check {
            //     println!();
            // }
            let max_state = self.machine.max_state_used(self.last_field_id);
            if max_state < self.n_states {
                let tr_used = &self.machine.transitions[2..max_state * 2 + 2];
                if check_no_halt_transition(tr_used) {
                    status = MachineStatus::DecidedNonHalt(
                        crate::status::NonHaltReason::NoHaltTransition,
                    );
                }
            }
            if status == MachineStatus::NoDecision {
                // use Check_Cycler_Small
                let has_two_undefined = self.machine.has_at_least_two_undefined(self.last_field_id);
                if !has_two_undefined {
                    // only one undefined (halt), use pre-decider checks for Non-Halt identification
                }

                // TODO faster if working with MachineBinary for small cycler
                let m_id = MachineId::new_no_id(self.machine);
                status = self.decider_cycler.decide_machine(&m_id);

                // debug output
                #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
                println!(
                    "{:>3} {}: {status}, Cycler ",
                    self.tnf_machines_count, self.machine
                );

                if let MachineStatus::Undecided(_, _, _) = status {
                    status = self.decider_bouncer.decide_machine(&m_id);
                    #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
                    println!(
                        "{:>3} {}: {status}, Bouncer ",
                        self.tnf_machines_count, self.machine
                    );
                    #[cfg(feature = "enable_csv_for_tnf")]
                    self.csv.write_machine(
                        &self.machine,
                        &status,
                        DeciderBouncer128::decider_id().name,
                    );
                } else {
                    #[cfg(feature = "enable_csv_for_tnf")]
                    self.csv.write_machine(
                        &self.machine,
                        &status,
                        DeciderCyclerSmall::decider_id().name,
                    );
                }
            } else {
                // debug output
                #[cfg(feature = "enable_csv_for_tnf")]
                {
                    self.csv.write_machine(&self.machine, &status, PRE_CHECK);
                }
                #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
                println!(
                    "{:>3} {}: {}, {PRE_CHECK}",
                    self.tnf_machines_count, self.machine, status
                );
            }

            #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
            if self.tnf_machines_count % 512 == 0 {
                println!();
            }

            match status {
                // Halts, machine is put on the stack and later expanded on the stop field with all permutations of that field.
                MachineStatus::DecidedHaltField(steps, field_no) => {
                    // println!("Result Cycler {count_m}: {} {status}", self.machine);
                    if steps >= self.max_steps {
                        if steps > self.max_steps {
                            self.max_steps = steps;
                            self.machines_max_steps.clear();
                        }
                        let mut m = self.machine;
                        m.transitions[field_no] = TRANSITION_BINARY_HALT;
                        self.machines_max_steps.push(MachineInfo::new(m, status));
                    }
                    // If only one undefined is left, then that one must be the halt condition.
                    // Iterating would result in machines without halt condition.
                    if self.machine.has_at_least_two_undefined(self.last_field_id) {
                        #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
                        {
                            let mut m = self.machine;
                            m.transitions[field_no] = TRANSITION_BINARY_HALT;
                            println!("to stack {}: {m}", self.machines_for_stack.len());
                        }
                        self.machines_for_stack.push_back(MachineForStack {
                            machine: self.machine,
                            field_no,
                        });
                        #[cfg(all(feature = "debug_enumerator", feature = "enable_csv_for_tnf"))]
                        {
                            if self.max_for_stack < self.machines_for_stack.len() {
                                self.max_for_stack = self.machines_for_stack.len();
                                self.csv
                                    .writeln(&format!("Max stack = {}", self.max_for_stack));
                            }
                        }
                    }
                }

                // Non-Halt: In this case the machine is just irrelevant and the tree is cut here as it is not further pursued.
                MachineStatus::DecidedNonHalt(_) => {}

                MachineStatus::Undecided(_, _, _) => {
                    self.undecided_count += 1;
                    if self.machine.has_at_least_two_undefined(
                        MachineBinary::last_used_field_id_in_transition_array_exclusive(max_state),
                    ) {
                        self.undecided_multiple_halts_count += 1;
                        if self.undecided_machines_multiple_halts.len() < 10 {
                            self.undecided_machines_multiple_halts.push(self.machine);
                        }
                        #[cfg(all(debug_assertions, feature = "debug_enumerator"))]
                        {
                            println!("Machine Undecided: {}", self.machine);
                            println!("This is an erroneous situation, as the halt is not reached but could potentially, requiring more machines to create.");
                            // todo!("Undecided with more than one halt condition.");
                        }
                    }
                    // println!("Result Cycler {count_m}: {} {status}", self.machine);
                    permutations.push(MachineId::new_no_id(self.machine))
                }
                _ => todo!(),
            }
        }

        println!("\nResult: \n{}", &self);
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
        writeln!(f, "machines total: {}", self.n_machines)?;
        writeln!(f, "machines in TNF tree: {}", self.tnf_machines_count)?;
        writeln!(f, "limit: {}", self.limit)?;
        let mut t = Vec::new();
        for i in 0..self.n_states * 4 {
            t.push(self.tr_permutations[i].to_string());
        }
        writeln!(f, "Transitions: {}", t.join(", "))?;
        writeln!(f, "Max Steps: {}", self.max_steps)?;
        for m in self.machines_max_steps.iter() {
            writeln!(f, "Max Machine: {m}")?;
        }
        writeln!(f, "Undecided machines: {}", self.undecided_count)?;
        writeln!(
            f,
            "Undecided machines which could stop and need more sub-machines: {}",
            self.undecided_multiple_halts_count
        )?;
        for m in self.undecided_machines_multiple_halts.iter() {
            writeln!(f, "check: {m}")?;
        }

        Ok(())
    }
}

const CSV_HEADER: &str = "machine,status,decider";
#[derive(Debug)]

pub struct CsvWriter {
    n_states: usize,
    /// Main path without sub directory
    html_out_path: String,
    // / Sub-dir. This is mandatory, the option is only to check if it is set.
    // sub_dir: Option<String>,
    /// full path, set_sub_dir to set this path, mandatory.
    file_name: String,
    buf_writer: BufWriter<File>,
    count_line: u64,
}

impl CsvWriter {
    pub fn try_new(config: &Config) -> Result<Self, std::io::Error> {
        let n_states = config.n_states();
        let file_name = format!("BB{n_states} enumeration.csv");
        let html_out_path = config.config_toml().html_out_path().to_string();
        let p = std::path::Path::new(&html_out_path).join(&file_name);
        let file = File::create(&p)?;
        let mut buf_writer = BufWriter::new(file);
        writeln!(buf_writer, "{CSV_HEADER}")?;

        Ok(Self {
            n_states,
            html_out_path,
            file_name,
            buf_writer,
            count_line: 0,
        })
    }

    pub fn writeln(&mut self, text: &str) {
        self.count_line += 1;
        writeln!(self.buf_writer, "{text}").expect("File write error");
    }

    pub fn write_machine(
        &mut self,
        machine: &MachineBinary,
        status: &MachineStatus,
        decider_name: &str,
    ) {
        self.count_line += 1;
        if self.count_line % 1024 * 128 == 0 {
            println!("CSV file writing: line {}", self.count_line);
        }
        let txt = format!("{machine},{status},{decider_name}");
        writeln!(self.buf_writer, "{txt}").expect("File write error");
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
    let n_states = 3;
    let config_1 = Config::builder(n_states)
        // 10_000_000_000 for BB4
        .machine_limit(1000_000_000_000)
        .limit_machines_undecided(200)
        .write_html_file_undecided(true)
        .build();

    // let dc_cycler_1 = DeciderStandard::Cycler.decider_config(&config_1);
    // let decider_config = vec![
    //     dc_cycler_1,
    //     // dc_bouncer_1,
    //     // dc_cycler_2,
    //     // dc_hold,
    // ];

    let (config, config_cycler_2) = DeciderConfig::standard_config(&config_1);
    let decider_config = DeciderStandard::standard_decider_for_config(&config, &config_cycler_2);

    let decider_last = 4;
    let result = decider_engine::run_decider_chain_gen(
        &decider_config[0..decider_last],
        EnumeratorType::EnumeratorTNF,
        CoreUsage::SingleCore,
    );

    println!("\n{}", result.to_string_with_duration());
    // if let Some(m_undecided) = result.machines_undecided_sorted() {
    //     for m in m_undecided.iter().take(10) {
    //         println!("Undecided {}", m);
    //     }
    // }
}
