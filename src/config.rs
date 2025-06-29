use std::{fmt::Display, time::SystemTime};

use hashbrown::HashMap;
use num_format::ToFormattedString;

use crate::{
    generator_full::GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION,
    generator_reduced::GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED, utils::user_locale,
};

// File path, can always be passed as parameter.
pub(crate) const PATH_DATA: &str = "./data/";
pub(crate) const FILE_PATH_BB5_CHALLENGE_DATA_FILE: &str =
    "res/all_5_states_undecided_machines_with_global_header";

// Tape
pub(crate) const TAPE_SIZE_INIT_CELL_BLOCKS: usize = 8;
/// Initial size tape long, min is 256. Must be a multiple of 32.
pub(crate) const TAPE_SIZE_INIT_CELLS: usize = TAPE_SIZE_INIT_CELL_BLOCKS * 32;
pub(crate) const MAX_TAPE_GROWTH: usize = 2 << 20; // 1 MB

/// Only used in Default to initialize, use new_default() instead.
pub(crate) const N_STATES_DEFAULT: usize = 5;
/// Default tape size limit if not changed in working machine.
const BATCH_SIZE_FILE: usize = 200;
const TAPE_SIZE_LIMIT_DEFAULT: usize = 20000;
const RECORD_MACHINES_MAX_STEPS_DEFAULT: usize = 10;
const CPU_UTILIZATION_DEFAULT: usize = 100;

// --- Below are program defining definitions, where changes may have a serious impact. ---

// TODO use these StepTypes instead of StepType
/// Number type used for step counters which may exceed u64 and are not used as collection index.
pub type StepTypeBig = u32;
/// Number type used for step counters which are used as collection index.
pub type StepTypeCol = usize;
/// Number type used for step counters which never exceed u32 and may be used as collection index (casting is free on u64 machines).
/// Smaller may be better for memory usage and performance, larger if more than 2.4 billion steps (u32) are required.
pub type StepTypeSmall = u32;
/// Number type for the machine id and other values related to MAX_STATES.
/// The idea is to allow states 8, 9 and 10 by switching to u128.
/// However, most code was written before this was introduced, and needs to be evaluated and tested for u128.
pub type IdBig = u64;

/// Number of states the program can handle. Max working is 7, as this is the limit for u64.
/// This is used for array definitions. Higher numbers require more memory and slow down execution.
// TODO change u64 type to UBB to allow max 10.
pub const MAX_STATES: usize = 5;
/// Number of states the TransitionGeneral should be able to handle.
/// TODO test and describe limits
pub(crate) const MAX_STATES_GENERIC: usize = 10;
pub(crate) const MAX_SYMBOLS_GENERIC: usize = 10;

// TODO make config reference with lifetime
// TODO then include file path?
// Display for Config
/// This sets the configuration for the decider run. \
/// Use new_default(n_states) or the builder to create a Config.
#[derive(Debug, Clone)]
pub struct Config {
    n_states: usize,
    /// This is the hold step limit. If this many steps are walked, then exit undecided.
    step_limit_hold: StepTypeBig,
    /// Search step limit for cycles. The loop size can be close to the max step size,
    /// but requires twice as many steps as the loop can only be identified if a repeated loop is found.
    step_limit_cycler: StepTypeSmall,
    /// Search step limit for bouncer.
    step_limit_bouncer: StepTypeSmall,
    /// The init value determines if machines with less steps are recorded.
    /// This can be updated as previous batch runs max can be used as init value for next batches,
    /// reducing updates because a new machine with higher max steps was found.
    steps_max_init: StepTypeBig,
    /// Unclear usage, if any. Should be the size of the tape, but the tape grows in packages.
    tape_size_limit: usize,
    /// For data provider: Return max this many machines.
    machines_limit: IdBig,
    // Ids from bb_challenge file (start, end exclusive). If None then all.
    file_id_range: Option<std::ops::Range<IdBig>>,
    /// batch size for operation
    batch_size: usize,
    /// Specific to the GeneratorFull: desired batch_size
    generator_full_batch_size_request: usize,
    /// Specific to the GeneratorReduced: desired batch_size. One needs to test different sizes for max performance.
    generator_reduced_batch_size_request: usize,
    // TODO remove, there are not many machines, possibly by replace record = true, but does it make a performance difference?
    // #[deprecated]
    limit_machines_max_steps: usize,
    /// This many undecided machines are stored in the ResultDecider. If full, the decider exits.
    limit_machines_undecided: usize,
    /// CPU utilization in percent, e.g. 75 -> 6 of 8 cores used. 0-150 allowed.
    cpu_utilization_percent: usize,
    /// Additional config e.g. for deciders using this library.
    config_key_value: HashMap<String, String>,
    /// Creation time of this Config. Used for file names.
    creation_time: SystemTime,
    /// When set to false UTC is used instead, but this may be confusing to the user.
    use_local_time: bool,
}

impl Config {
    /// Builder to initialize required values.
    pub fn builder(n_states: usize) -> ConfigBuilder {
        ConfigBuilder::new(n_states)
    }

    /// Default values for testing purposes. Better use builder.
    pub fn new_default(n_states: usize) -> Config {
        let step_limit = Self::step_limit_hold_default(n_states);
        Self {
            n_states,
            batch_size: BATCH_SIZE_FILE,
            step_limit_hold: step_limit,
            steps_max_init: if n_states == 1 { 0 } else { 2 },
            // TODO depending on n_states
            tape_size_limit: TAPE_SIZE_LIMIT_DEFAULT,
            machines_limit: Self::generate_limit_default(n_states),
            generator_full_batch_size_request: GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION,
            generator_reduced_batch_size_request: GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED,
            file_id_range: None,
            limit_machines_max_steps: RECORD_MACHINES_MAX_STEPS_DEFAULT,
            limit_machines_undecided: 0,
            cpu_utilization_percent: CPU_UTILIZATION_DEFAULT,
            config_key_value: HashMap::new(),
            creation_time: SystemTime::now(),
            use_local_time: true,
            step_limit_bouncer: Self::step_limit_bouncer_default(n_states),
            step_limit_cycler: Self::step_limit_cycler_default(n_states),
        }
    }

    /// Step limit defaults for actual runs.
    pub fn step_limit_hold_default(n_states: usize) -> StepTypeBig {
        match n_states {
            1 => 10,
            2 => 10,
            3 => 25,
            4 => 110,
            5 => 50_000_000,
            _ => panic!("Cannot handle this step limit!"),
        }
    }

    /// Step limit defaults for actual runs of deciders of type bouncer.
    pub fn step_limit_bouncer_default(n_states: usize) -> StepTypeSmall {
        // TODO fine tune
        match n_states {
            1 => 1_000,
            2 => 1_000,
            3 => 5_000,
            4 => 20_000,
            5 => 150_000,
            _ => panic!("Cannot handle this step limit!"),
        }
    }

    /// Step limit defaults for actual runs of deciders of type cycler.
    pub fn step_limit_cycler_default(n_states: usize) -> StepTypeSmall {
        // TODO fine tune
        match n_states {
            1 => 100,
            2 => 100,
            3 => 250,
            4 => 500,
            5 => 5_100,
            _ => panic!("Cannot handle this step limit!"),
        }
    }

    /// Generator limit, designed for testing purposes.
    pub fn generate_limit_default(n_states: usize) -> u64 {
        match n_states {
            1 | 2 => 10_000,
            // covers all machines
            3 => 5_000_000,
            // will find highest machine
            4 => 200_000_000,
            // TODO higher limit, to find 47.xxx.xxx
            5 => 350_000_000,
            _ => panic!("Not build for this."),
        }
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn config_key_value(&self) -> &HashMap<String, String> {
        &self.config_key_value
    }

    /// Returns the value for the given key (get() from HashMap).
    pub fn config_value(&self, key: &str) -> Option<&String> {
        self.config_key_value.get(key)
    }

    pub fn cpu_utilization_percent(&self) -> usize {
        self.cpu_utilization_percent
    }

    pub fn creation_time(&self) -> SystemTime {
        self.creation_time
    }

    pub fn file_id_range(&self) -> Option<std::ops::Range<IdBig>> {
        self.file_id_range.clone()
    }

    pub fn file_id_range_to_string(&self) -> String {
        let locale = user_locale();
        if let Some(range) = &self.file_id_range {
            format!(
                "{}..{}",
                range.start.to_formatted_string(&locale),
                range.end.to_formatted_string(&locale)
            )
        } else {
            "unlimited".to_string()
        }
    }

    pub fn generator_batch_size_request_full(&self) -> usize {
        self.generator_full_batch_size_request
    }

    pub fn generator_batch_size_request_reduced(&self) -> usize {
        self.generator_reduced_batch_size_request
    }

    pub fn limit_machines_max_steps(&self) -> usize {
        self.limit_machines_max_steps
    }

    pub fn limit_machines_undecided(&self) -> usize {
        self.limit_machines_undecided
    }

    pub fn set_limit_machines_undecided(&mut self, limit: usize) {
        self.limit_machines_undecided = limit;
    }

    pub fn machines_limit(&self) -> u64 {
        self.machines_limit
    }

    pub fn n_states(&self) -> usize {
        self.n_states
    }

    pub fn steps_max_init(&self) -> StepTypeBig {
        self.steps_max_init
    }

    // increases the value if new_max is larger
    pub fn increase_steps_max_init(&mut self, new_max: StepTypeBig) {
        if new_max > self.steps_max_init {
            self.steps_max_init = new_max;
        }
    }

    pub fn step_limit_hold(&self) -> StepTypeBig {
        self.step_limit_hold
    }

    pub fn step_limit_bouncer(&self) -> StepTypeSmall {
        self.step_limit_bouncer
    }

    pub fn step_limit_cycler(&self) -> StepTypeSmall {
        self.step_limit_cycler
    }

    pub fn tape_size_limit(&self) -> usize {
        self.tape_size_limit
    }

    pub fn use_local_time(&self) -> bool {
        self.use_local_time
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new_default(N_STATES_DEFAULT)
    }
}

// pub struct ConfigKeyValue {
//     key: String,
//     value: String,
// }

// TODO init_steps_max: StepType
#[derive(Default)]
pub struct ConfigBuilder {
    n_states: usize,
    batch_size: Option<usize>,
    step_limit_hold: Option<StepTypeBig>,
    step_limit_bouncer: Option<StepTypeSmall>,
    step_limit_cycler: Option<StepTypeSmall>,
    tape_size_limit: Option<usize>,
    machines_limit: Option<u64>,
    file_id_range: Option<std::ops::Range<IdBig>>,
    generator_batch_size_request_full: Option<usize>,
    generator_batch_size_request_reduced: Option<usize>,
    limit_machines_max_steps: Option<usize>,
    limit_machines_undecided: Option<usize>,
    cpu_utilization_percent: Option<usize>,
    config_key_value: Option<HashMap<String, String>>,
    use_local_time: Option<bool>,
}

impl ConfigBuilder {
    fn new(n_states: usize) -> Self {
        Self {
            n_states,
            ..Default::default() // All: None,
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    pub fn cpu_utilization(mut self, percent: usize) -> Self {
        self.cpu_utilization_percent = Some(percent);
        self
    }

    pub fn file_id_range(mut self, file_id_range: std::ops::Range<IdBig>) -> Self {
        self.file_id_range = Some(file_id_range);
        self
    }

    pub fn generator_full_batch_size_request(mut self, batch_size_request: usize) -> Self {
        self.generator_batch_size_request_full = Some(batch_size_request);
        self
    }

    pub fn generator_reduced_batch_size_request(mut self, batch_size_request: usize) -> Self {
        self.generator_batch_size_request_reduced = Some(batch_size_request);
        self
    }

    pub fn limit_machines_max_steps(mut self, value: usize) -> Self {
        self.limit_machines_max_steps = Some(value);
        self
    }

    pub fn limit_machines_undecided(mut self, value: usize) -> Self {
        self.limit_machines_undecided = Some(value);
        self
    }

    pub fn machine_limit(mut self, generate_limit: u64) -> Self {
        self.machines_limit = Some(generate_limit);
        self
    }

    pub fn step_limit_hold(mut self, step_limit: StepTypeBig) -> Self {
        self.step_limit_hold = Some(step_limit);
        self
    }

    pub fn step_limit_bouncer(mut self, step_limit: StepTypeSmall) -> Self {
        self.step_limit_bouncer = Some(step_limit);
        self
    }

    pub fn step_limit_cycler(mut self, step_limit: StepTypeSmall) -> Self {
        self.step_limit_cycler = Some(step_limit);
        self
    }

    pub fn tape_size_limit(mut self, tape_size_limit: usize) -> Self {
        self.tape_size_limit = Some(tape_size_limit);
        self
    }

    pub fn use_local_time(mut self, value_false_is_utc: bool) -> Self {
        self.use_local_time = Some(value_false_is_utc);
        self
    }

    pub fn build(self) -> Config {
        Config {
            n_states: self.n_states,
            batch_size: self.batch_size.unwrap_or(BATCH_SIZE_FILE),
            step_limit_hold: self
                .step_limit_hold
                .unwrap_or_else(|| Config::step_limit_hold_default(self.n_states)),
            step_limit_bouncer: self
                .step_limit_bouncer
                .unwrap_or_else(|| Config::step_limit_bouncer_default(self.n_states)),
            step_limit_cycler: self
                .step_limit_cycler
                .unwrap_or_else(|| Config::step_limit_cycler_default(self.n_states)),
            steps_max_init: if self.n_states == 1 { 0 } else { 2 },
            tape_size_limit: self.tape_size_limit.unwrap_or(TAPE_SIZE_LIMIT_DEFAULT),
            machines_limit: self
                .machines_limit
                .unwrap_or_else(|| Config::generate_limit_default(self.n_states)),
            generator_full_batch_size_request: self
                .generator_batch_size_request_full
                .unwrap_or(GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION),
            generator_reduced_batch_size_request: self
                .generator_batch_size_request_reduced
                .unwrap_or(GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED),
            file_id_range: self.file_id_range,
            limit_machines_max_steps: self
                .limit_machines_max_steps
                .unwrap_or(RECORD_MACHINES_MAX_STEPS_DEFAULT),
            limit_machines_undecided: self.limit_machines_undecided.unwrap_or(0),
            cpu_utilization_percent: self
                .cpu_utilization_percent
                .unwrap_or(CPU_UTILIZATION_DEFAULT),
            config_key_value: self.config_key_value.unwrap_or_default(),
            creation_time: SystemTime::now(),
            use_local_time: true,
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = user_locale();
        write!(
            f,
            "Limit Steps Hold: {}, Cycler: {}, Bouncer: {}, ",
            self.step_limit_hold.to_formatted_string(&locale),
            self.step_limit_cycler.to_formatted_string(&locale),
            self.step_limit_bouncer.to_formatted_string(&locale)
        )?;
        writeln!(
            f,
            "Limit Machines: {}, File Id Range: {}",
            self.machines_limit.to_formatted_string(&locale),
            self.file_id_range_to_string()
        )?;
        write!(
            f,
            "Batch Size Data Provider: {}, Gen Full: {}, Gen Reduced: {}, ",
            self.batch_size.to_formatted_string(&locale),
            self.generator_batch_size_request_full()
                .to_formatted_string(&locale),
            self.generator_batch_size_request_reduced()
                .to_formatted_string(&locale)
        )
    }
}
