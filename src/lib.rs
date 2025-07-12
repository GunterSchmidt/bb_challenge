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
pub mod decider_bouncer;
pub mod decider_bouncer_v2;
pub mod decider_cycler_v4;
pub mod decider_cycler_v5;
pub mod decider_engine;
pub mod decider_hold_u128_long;
pub mod decider_hold_u128_long_v2;
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
pub mod tape_utils;
pub mod transition_generic;
pub mod transition_symbol2;
pub mod utils;

pub type ResultUnitEndReason = Result<(), decider_result::EndReason>;

// #[allow(clippy::enum_variant_names)]
/// This enum can be used to call the different variants.
pub enum Cores {
    SingleCore,
    SingleCoreGeneratorMultiCoreDecider,
    MultiCore,
}
