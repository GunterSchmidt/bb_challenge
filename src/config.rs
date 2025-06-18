use crate::{StepType, GENERATOR_BATCH_SIZE_RECOMMENDATION, TAPE_SIZE_LIMIT_DEFAULT};

const RECORD_MACHINES_MAX_STEPS_DEFAULT: u16 = 10;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    n_states: usize,
    // not StepType as comparison is against len?
    pub step_limit: StepType,
    pub tape_size_limit: usize,
    pub generate_limit: u64,
    pub generator_batch_size_request: usize,
    pub record_machines_max_steps: u16,
    pub record_machines_undecided: u32,
}

impl Config {
    pub fn new(
        n_states: usize,
        step_limit: StepType,
        tape_size_limit: usize,
        generate_limit: u64,
        generator_batch_size_request: usize,
        record_machines_max_steps: u16,
        record_machines_undecided: u32,
    ) -> Self {
        Self {
            n_states,
            step_limit,
            tape_size_limit,
            generate_limit,
            generator_batch_size_request,
            record_machines_max_steps,
            record_machines_undecided,
        }
    }

    pub fn builder(n_states: usize) -> ConfigBuilder {
        ConfigBuilder::new(n_states)
    }

    pub fn new_default(n_states: usize) -> Config {
        let step_limit = Self::decider_step_limit_default(n_states);
        Self {
            n_states,
            step_limit,
            // TODO depending on n_states
            tape_size_limit: TAPE_SIZE_LIMIT_DEFAULT,
            generate_limit: Self::generate_limit_default(n_states),
            generator_batch_size_request: GENERATOR_BATCH_SIZE_RECOMMENDATION,
            record_machines_max_steps: RECORD_MACHINES_MAX_STEPS_DEFAULT,
            record_machines_undecided: 0,
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
            4 => 350_000_000,
            // TODO higher limit, to find 47.xxx.xxx
            5 => 350_000_000,
            _ => panic!("Not build for this."),
        }
    }

    pub fn n_states(&self) -> usize {
        self.n_states
    }
}

pub struct ConfigBuilder {
    n_states: usize,
    step_limit: Option<StepType>,
    tape_size_limit: Option<usize>,
    generate_limit: Option<u64>,
    generator_batch_size_request: Option<usize>,
    record_machines_max_steps: Option<u16>,
    record_machines_undecided: Option<u32>,
}

impl ConfigBuilder {
    fn new(n_states: usize) -> Self {
        Self {
            n_states,
            step_limit: None,
            tape_size_limit: None,
            generate_limit: None,
            generator_batch_size_request: None,
            record_machines_max_steps: None,
            record_machines_undecided: None,
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

    pub fn generator_batch_size_request(mut self, generator_batch_size_request: usize) -> Self {
        self.generator_batch_size_request = Some(generator_batch_size_request);
        self
    }

    pub fn record_machines_max_steps(mut self, record_machines_max_steps: u16) -> Self {
        self.record_machines_max_steps = Some(record_machines_max_steps);
        self
    }

    pub fn record_machines_undecided(mut self, record_machines_undecided: u32) -> Self {
        self.record_machines_undecided = Some(record_machines_undecided);
        self
    }

    pub fn build(self) -> Config {
        Config {
            n_states: self.n_states,
            step_limit: self
                .step_limit
                .unwrap_or_else(|| Config::decider_step_limit_default(self.n_states)),
            tape_size_limit: self.tape_size_limit.unwrap_or(TAPE_SIZE_LIMIT_DEFAULT),
            generate_limit: self
                .generate_limit
                .unwrap_or_else(|| Config::generate_limit_default(self.n_states)),
            generator_batch_size_request: self
                .generator_batch_size_request
                .unwrap_or(GENERATOR_BATCH_SIZE_RECOMMENDATION),
            record_machines_max_steps: self
                .record_machines_max_steps
                .unwrap_or(RECORD_MACHINES_MAX_STEPS_DEFAULT),
            record_machines_undecided: self.record_machines_undecided.unwrap_or(0),
        }
    }
}
