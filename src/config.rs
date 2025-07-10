use std::{fmt::Display, time::SystemTime};

use hashbrown::HashMap;
use num_format::ToFormattedString;

use crate::utils::file_exists;

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
const CPU_UTILIZATION_DEFAULT: usize = 100;

const GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION: usize = 500_000;
const GENERATOR_REDUCED_BATCH_SIZE_RECOMMENDATION: usize = 5_000_000;
const WRITE_HTML_STEP_LIMIT: u32 = 100_000;

// --- Below are program defining definitions, where changes may have a serious impact. ---

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

// TODO make config reference with lifetime,
// TODO include file path?
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
    steps_min: StepTypeBig,
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
    /// This many decided machines are stored in the ResultDecider. If full, the decider exits.
    /// This is mainly for individual ResultDeciders to further process machines with certain characteristics.
    limit_machines_decided: usize,
    /// This many undecided machines are stored in the ResultDecider. If full, the decider exits.
    /// This is mainly to find machines to further analyze.
    limit_machines_undecided: usize,
    /// CPU utilization in percent, e.g. 75 -> 6 of 8 cores used. 0-150 allowed.
    cpu_utilization_percent: usize,
    /// Additional config e.g. for deciders using this library.
    config_key_value: HashMap<String, String>,
    /// Creation time of this Config. Used for file names.
    creation_time: SystemTime,
    /// When set to false UTC is used instead, but this may be confusing to the user.
    use_local_time: bool,
    /// Outputs decider steps into an html file
    write_html_file: bool,
    write_html_step_limit: u32,
}

impl Config {
    /// Builder to initialize required values.
    pub fn builder(n_states: usize) -> ConfigBuilder {
        ConfigBuilder::new(n_states)
    }

    /// Builder to initialize required values.
    pub fn builder_from_config(config: &Config) -> ConfigBuilder {
        ConfigBuilder::new_config(config)
    }

    /// Default values for testing purposes. Better use builder.
    pub fn new_default(n_states: usize) -> Config {
        let step_limit = Self::step_limit_hold_default(n_states);
        Self {
            n_states,
            batch_size: BATCH_SIZE_FILE,
            step_limit_hold: step_limit,
            steps_min: if n_states == 1 { 0 } else { 2 },
            // TODO depending on n_states
            tape_size_limit: TAPE_SIZE_LIMIT_DEFAULT,
            machines_limit: Self::generate_limit_default(n_states),
            generator_full_batch_size_request: GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION,
            generator_reduced_batch_size_request: GENERATOR_REDUCED_BATCH_SIZE_RECOMMENDATION,
            file_id_range: None,
            limit_machines_decided: 0,
            limit_machines_undecided: 0,
            cpu_utilization_percent: CPU_UTILIZATION_DEFAULT,
            config_key_value: HashMap::new(),
            creation_time: SystemTime::now(),
            use_local_time: true,
            step_limit_bouncer: Self::step_limit_bouncer_default(n_states),
            step_limit_cycler: Self::step_limit_cycler_default(n_states),
            write_html_file: false,
            write_html_step_limit: WRITE_HTML_STEP_LIMIT,
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

    pub fn limit_machines_decided(&self) -> usize {
        self.limit_machines_decided
    }

    pub fn limit_machines_undecided(&self) -> usize {
        self.limit_machines_undecided
    }

    // pub fn set_limit_machines_undecided(&mut self, limit: usize) {
    //     self.limit_machines_undecided = limit;
    // }

    pub fn machines_limit(&self) -> u64 {
        self.machines_limit
    }

    pub fn n_states(&self) -> usize {
        self.n_states
    }

    pub fn steps_min(&self) -> StepTypeBig {
        self.steps_min
    }

    // // increases the value if new_max is larger
    // pub fn increase_steps_min(&mut self, new_max: StepTypeBig) {
    //     if new_max > self.steps_min {
    //         self.steps_min = new_max;
    //     }
    // }

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

    pub fn write_html_file(&self) -> bool {
        self.write_html_file
    }

    pub fn write_html_step_limit(&self) -> u32 {
        self.write_html_step_limit
    }

    // TODO TOML config file
    /// Directory for all file outputs
    pub fn get_result_path() -> String {
        let path = "./result";
        if !file_exists(path) {
            // create dir
            std::fs::create_dir(path).expect("Path could not be created.");
        }
        path.to_string()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new_default(N_STATES_DEFAULT)
    }
}

// TODO init_steps_max: StepType
#[derive(Default)]
pub struct ConfigBuilder {
    n_states: usize,
    batch_size: Option<usize>,
    file_id_range: Option<std::ops::Range<IdBig>>,
    generator_batch_size_request_full: Option<usize>,
    generator_batch_size_request_reduced: Option<usize>,
    step_limit_hold: Option<StepTypeBig>,
    step_limit_bouncer: Option<StepTypeSmall>,
    step_limit_cycler: Option<StepTypeSmall>,
    tape_size_limit: Option<usize>,
    machines_limit: Option<u64>,
    limit_machines_decided: Option<usize>,
    limit_machines_undecided: Option<usize>,
    cpu_utilization_percent: Option<usize>,
    config_key_value: Option<HashMap<String, String>>,
    use_local_time: Option<bool>,
    write_html_file: Option<bool>,
    write_html_step_limit: Option<u32>,
}

impl ConfigBuilder {
    fn new(n_states: usize) -> Self {
        Self {
            n_states,
            ..Default::default() // All: None,
        }
    }

    fn new_config(config: &Config) -> ConfigBuilder {
        Self {
            n_states: config.n_states,
            batch_size: Some(config.batch_size),
            step_limit_hold: Some(config.step_limit_hold),
            step_limit_bouncer: Some(config.step_limit_bouncer),
            step_limit_cycler: Some(config.step_limit_cycler),
            tape_size_limit: Some(config.tape_size_limit),
            machines_limit: Some(config.machines_limit),
            file_id_range: config.file_id_range.clone(),
            generator_batch_size_request_full: Some(config.generator_full_batch_size_request),
            generator_batch_size_request_reduced: Some(config.generator_reduced_batch_size_request),
            limit_machines_decided: Some(config.limit_machines_decided),
            limit_machines_undecided: Some(config.limit_machines_undecided),
            cpu_utilization_percent: Some(config.cpu_utilization_percent),
            config_key_value: Some(config.config_key_value.clone()),
            use_local_time: Some(config.use_local_time),
            write_html_file: Some(config.write_html_file),
            write_html_step_limit: Some(config.write_html_step_limit),
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

    pub fn limit_machines_decided(mut self, value: usize) -> Self {
        self.limit_machines_decided = Some(value);
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

    pub fn write_html_file(mut self, value: bool) -> Self {
        self.write_html_file = Some(value);
        self
    }

    pub fn write_html_step_limit(mut self, value: u32) -> Self {
        self.write_html_step_limit = Some(value);
        self
    }

    pub fn build(self) -> Config {
        let mut config = Config {
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
            steps_min: if self.n_states == 1 { 0 } else { 2 },
            tape_size_limit: self.tape_size_limit.unwrap_or(TAPE_SIZE_LIMIT_DEFAULT),
            machines_limit: self
                .machines_limit
                .unwrap_or_else(|| Config::generate_limit_default(self.n_states)),
            generator_full_batch_size_request: self
                .generator_batch_size_request_full
                .unwrap_or(GENERATOR_FULL_BATCH_SIZE_RECOMMENDATION),
            generator_reduced_batch_size_request: self
                .generator_batch_size_request_reduced
                .unwrap_or(GENERATOR_REDUCED_BATCH_SIZE_RECOMMENDATION),
            file_id_range: self.file_id_range,
            limit_machines_decided: self.limit_machines_decided.unwrap_or(0),
            limit_machines_undecided: self.limit_machines_undecided.unwrap_or(0),
            cpu_utilization_percent: self
                .cpu_utilization_percent
                .unwrap_or(CPU_UTILIZATION_DEFAULT),
            config_key_value: self.config_key_value.unwrap_or_default(),
            creation_time: SystemTime::now(),
            use_local_time: self.use_local_time.unwrap_or(true),
            write_html_file: self.write_html_file.unwrap_or(false),
            write_html_step_limit: self.write_html_step_limit.unwrap_or(WRITE_HTML_STEP_LIMIT),
        };

        #[cfg(not(feature = "bb_enable_html_reports"))]
        if config.write_html_file {
            println!("WARNING: feature 'bb_enable_html_reports' is not enabled, cannot write HTML files.");
            config.write_html_file = false;
        }

        config
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

pub fn user_locale() -> num_format::Locale {
    // TODO get user locale
    // let locale = SystemLocale::default().unwrap(); // does not work on windows

    num_format::Locale::en
}
