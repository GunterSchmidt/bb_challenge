pub mod config;
pub mod decider;
pub mod decider_bouncer;
// pub mod decider_engine;
pub mod decider_expanding_loop;
pub mod decider_loop_v4;
pub mod decider_u128;
pub mod decider_u128_long;
pub mod decider_u64;
pub mod error;
pub mod file;
pub mod generator;
pub mod generator_full;
pub mod generator_reduced;
pub mod machine;
pub mod machine_info;
pub mod pre_deciders;
pub mod reporter;
pub mod result;
pub mod status;
pub mod sub_decider;
pub mod sub_decider_loop_v4;
pub mod tape_utils;
pub mod test;
pub mod transition_generic;
pub mod transition_symbol2;
// pub mod transition_v3;
pub mod utils;

/// Number of states the program can handle. Used for array definitions.
pub const MAX_STATES: usize = 5;

/// Number type used for step counters.
// TODO unclear usage, smaller may be better for storage, larger if more than 2.4 billion steps are required
pub type StepType = u32;

/// Default step limit if not changed in working machine.
pub const STEP_LIMIT_DEFAULT: StepType = 50_000_000;
/// Default tape size limit if not changed in working machine.
pub const TAPE_SIZE_LIMIT_DEFAULT: usize = 20000;

/// Recommended batch size
pub const GENERATOR_BATCH_SIZE_RECOMMENDATION: usize = 500_000;
const TAPE_SIZE_INIT_CELL_BLOCKS: usize = 8;
/// Initial size tape long, min is 256. Must be a multiple of 32.
const TAPE_SIZE_INIT_CELLS: usize = TAPE_SIZE_INIT_CELL_BLOCKS * 32;
const MAX_TAPE_GROWTH: usize = 2 << 20; // 1 MB
