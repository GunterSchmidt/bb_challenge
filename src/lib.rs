// pub mod decider_engine;
// pub mod decider_expanding_loop;
// pub mod decider_u128;
// pub mod decider_u128_long;
// pub mod sub_decider;
// pub mod sub_decider_loop_v4;
pub mod arg_handler;
pub mod bb_file_reader;
pub mod config;
pub mod data_provider;
pub mod decider;
pub mod decider_bouncer_v1;
pub mod decider_bouncer_v2;
pub mod decider_cycler;
pub mod decider_data_128;
pub mod decider_engine;
pub mod decider_hold_u128_long_v3;
pub mod decider_result;
pub mod decider_result_worker;
pub mod decider_u64;
pub mod error;
pub mod generator;
pub mod generator_full;
pub mod generator_reduced;
pub mod html;
pub mod machine;
pub mod machine_info;
pub mod pre_decider;
pub mod reporter;
pub mod single_thread_worker;
pub mod status;
pub mod step_record;
pub mod tape_utils;
pub mod transition_generic;
pub mod transition_symbol2;
pub mod utils;

pub type ResultUnitEndReason = Result<(), decider_result::EndReason>;

/// This is used to define the CPU usage during generator and decider run.
// TODO possibly move CPU percent into this enum, remove from Config
pub enum CoreUsage {
    SingleCore,
    SingleCoreGeneratorMultiCoreDecider,
    MultiCore,
}
