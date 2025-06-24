use std::time::SystemTime;

use crate::StepType;
use hashbrown::HashMap;

/// Default step limit if not changed in working machine.
pub const STEP_LIMIT_DEFAULT: StepType = 50_000_000;
pub const GENERATOR_BATCH_SIZE_RECOMMENDATION_FULL: usize = 500_000;
pub const GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED: usize = 5_000_000;
/// Default tape size limit if not changed in working machine.
const TAPE_SIZE_LIMIT_DEFAULT: usize = 20000;
const RECORD_MACHINES_MAX_STEPS_DEFAULT: usize = 10;
const CPU_UTILIZATION_DEFAULT: usize = 100;
// TODO make config reference with lifetime
// TODO include data path
pub const PATH_DATA: &str = "./data/";

/// This sets the configuration for the decider run. \
/// Use new_default(n_states) or the builder to create a Config.
#[derive(Debug, Clone)]
pub struct Config {
    n_states: usize,
    // not StepType as comparison is against len?
    step_limit: StepType,
    // This can be updated as previous batch runs max can be used as init value.
    init_steps_max: StepType,
    tape_size_limit: usize,
    generate_limit: u64,
    generator_batch_size_request_full: usize,
    generator_batch_size_request_reduced: usize,
    // TODO remove, there are not many machines, possibly by replace record = true, but does it make a performance difference?
    limit_machines_max_steps: usize,
    limit_machines_undecided: usize,
    /// CPU utilization in percent, e.g. 75 -> 6 of 8 cores used. 0-150 allowed.
    cpu_utilization_percent: usize,
    /// Additional config e.g. for specific deciders.
    config_key_value: HashMap<String, String>,
    creation_time: SystemTime,
    /// when set to false UTC is used instead, e.g. for file name
    // TODO builder
    use_local_time: bool,
}

impl Config {
    // pub fn new(
    //     n_states: usize,
    //     step_limit: StepType,
    //     init_steps_max: StepType,
    //     tape_size_limit: usize,
    //     generate_limit: u64,
    //     generator_batch_size_request_full: usize,
    //     generator_batch_size_request_reduced: usize,
    //     record_machines_max_steps: usize,
    //     record_machines_undecided: usize,
    //     cpu_utilization: usize,
    // ) -> Self {
    //     Self {
    //         n_states,
    //         step_limit,
    //         init_steps_max,
    //         tape_size_limit,
    //         generate_limit,
    //         generator_batch_size_request_full,
    //         generator_batch_size_request_reduced,
    //         record_machines_max_steps,
    //         record_machines_undecided,
    //         cpu_utilization,
    //     }
    // }

    pub fn builder(n_states: usize) -> ConfigBuilder {
        ConfigBuilder::new(n_states)
    }

    pub fn new_default(n_states: usize) -> Config {
        let step_limit = Self::decider_step_limit_default(n_states);
        Self {
            n_states,
            step_limit,
            init_steps_max: if n_states == 1 { 0 } else { 2 },
            // TODO depending on n_states
            tape_size_limit: TAPE_SIZE_LIMIT_DEFAULT,
            generate_limit: Self::generate_limit_default(n_states),
            generator_batch_size_request_full: GENERATOR_BATCH_SIZE_RECOMMENDATION_FULL,
            generator_batch_size_request_reduced: GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED,
            limit_machines_max_steps: RECORD_MACHINES_MAX_STEPS_DEFAULT,
            limit_machines_undecided: 0,
            cpu_utilization_percent: CPU_UTILIZATION_DEFAULT,
            config_key_value: HashMap::new(),
            creation_time: SystemTime::now(),
            use_local_time: true,
        }
    }

    pub fn decider_step_limit_default(n_states: usize) -> StepType {
        match n_states {
            1 => 10,
            2 => 10,
            3 => 25,
            4 => 110,
            5 => 50_000_000,
            _ => panic!("Cannot handle this step limit!"),
        }
    }

    pub fn generate_limit_default(n_states: usize) -> u64 {
        match n_states {
            1 | 2 => 10_000,
            3 => 5_000_000,
            4 => 200_000_000,
            // TODO higher limit, to find 47.xxx.xxx
            5 => 350_000_000,
            _ => panic!("Not build for this."),
        }
    }

    pub fn cpu_utilization_percent(&self) -> usize {
        self.cpu_utilization_percent
    }

    pub fn generate_limit(&self) -> u64 {
        self.generate_limit
    }

    pub fn generator_batch_size_request_full(&self) -> usize {
        self.generator_batch_size_request_full
    }

    pub fn generator_batch_size_request_reduced(&self) -> usize {
        self.generator_batch_size_request_reduced
    }

    pub fn init_steps_max(&self) -> u32 {
        self.init_steps_max
    }

    // increases the value if new_max is larger
    pub fn increase_init_step_max(&mut self, new_max: StepType) {
        if new_max > self.init_steps_max {
            self.init_steps_max = new_max;
        }
    }

    pub fn n_states(&self) -> usize {
        self.n_states
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

    pub fn step_limit(&self) -> u32 {
        self.step_limit
    }

    pub fn tape_size_limit(&self) -> usize {
        self.tape_size_limit
    }

    pub fn config_key_value(&self) -> &HashMap<String, String> {
        &self.config_key_value
    }

    /// Returns the value for the given key (get() from HashMap).
    pub fn config_value(&self, key: &str) -> Option<&String> {
        self.config_key_value.get(key)
    }

    pub fn creation_time(&self) -> SystemTime {
        self.creation_time
    }

    pub fn use_local_time(&self) -> bool {
        self.use_local_time
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
    step_limit: Option<StepType>,
    tape_size_limit: Option<usize>,
    generate_limit: Option<u64>,
    generator_batch_size_request_full: Option<usize>,
    generator_batch_size_request_reduced: Option<usize>,
    limit_machines_max_steps: Option<usize>,
    limit_machines_undecided: Option<usize>,
    cpu_utilization_percent: Option<usize>,
    config_key_value: Option<HashMap<String, String>>,
}

impl ConfigBuilder {
    fn new(n_states: usize) -> Self {
        Self {
            n_states,
            ..Default::default() // All: None,
        }
    }

    pub fn step_limit(mut self, step_limit: StepType) -> Self {
        self.step_limit = Some(step_limit);
        self
    }

    pub fn tape_size_limit(mut self, tape_size_limit: usize) -> Self {
        self.tape_size_limit = Some(tape_size_limit);
        self
    }

    pub fn generate_limit(mut self, generate_limit: u64) -> Self {
        self.generate_limit = Some(generate_limit);
        self
    }

    pub fn generator_batch_size_request_full(
        mut self,
        generator_batch_size_request_full: usize,
    ) -> Self {
        self.generator_batch_size_request_full = Some(generator_batch_size_request_full);
        self
    }

    pub fn generator_batch_size_request_reduced(
        mut self,
        generator_batch_size_request_reduced: usize,
    ) -> Self {
        self.generator_batch_size_request_reduced = Some(generator_batch_size_request_reduced);
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

    pub fn cpu_utilization(mut self, percent: usize) -> Self {
        self.cpu_utilization_percent = Some(percent);
        self
    }

    pub fn build(self) -> Config {
        Config {
            n_states: self.n_states,
            step_limit: self
                .step_limit
                .unwrap_or_else(|| Config::decider_step_limit_default(self.n_states)),
            init_steps_max: if self.n_states == 1 { 0 } else { 2 },
            tape_size_limit: self.tape_size_limit.unwrap_or(TAPE_SIZE_LIMIT_DEFAULT),
            generate_limit: self
                .generate_limit
                .unwrap_or_else(|| Config::generate_limit_default(self.n_states)),
            generator_batch_size_request_full: self
                .generator_batch_size_request_full
                .unwrap_or(GENERATOR_BATCH_SIZE_RECOMMENDATION_FULL),
            generator_batch_size_request_reduced: self
                .generator_batch_size_request_reduced
                .unwrap_or(GENERATOR_BATCH_SIZE_RECOMMENDATION_REDUCED),
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
