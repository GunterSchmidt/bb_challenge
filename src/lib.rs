pub mod config;
pub mod decider;
// pub mod decider_bouncer;
// pub mod decider_engine;
pub mod data_provider;
pub mod data_provider_threaded;
// pub mod decider_expanding_loop;
pub mod decider_hold_u128_long;
pub mod decider_loop_v4;
// pub mod decider_u128;
// pub mod decider_u128_long;
// pub mod decider_u64;
pub mod error;
pub mod file;
pub mod generator;
pub mod generator_full;
pub mod generator_reduced;
pub mod machine;
pub mod machine_info;
pub mod pre_decider;
pub mod reporter;
pub mod result;
pub mod status;
pub mod sub_decider;
pub mod sub_decider_loop_v4;
pub mod tape_utils;
pub mod test;
pub mod transition_generic;
pub mod transition_symbol2;
pub mod utils;

/// Number of states the program can handle. Used for array definitions. Max is 7, as this is the limit for u64.
// TODO change u64 type to UBB to allow max 10.
pub const MAX_STATES: usize = 5;
/// Number of states the TransitionGeneral should be able to handle.
/// TODO test and describe limits
pub const MAX_STATES_GENERIC: usize = 10;
pub const MAX_SYMBOLS_GENERIC: usize = 10;

/// Number type used for step counters.
/// Smaller may be better for storage, larger if more than 2.4 billion steps are required.
pub type StepType = u32;
/// Number type for the machine id and other values related to MAX_STATES.
pub type UBB = u64;

const TAPE_SIZE_INIT_CELL_BLOCKS: usize = 8;
/// Initial size tape long, min is 256. Must be a multiple of 32.
const TAPE_SIZE_INIT_CELLS: usize = TAPE_SIZE_INIT_CELL_BLOCKS * 32;
const MAX_TAPE_GROWTH: usize = 2 << 20; // 1 MB
