// // pub mod decider_engine;
// // pub mod decider_expanding_loop;
// // pub mod decider_u128;
// // pub mod decider_u128_long;
// // pub mod sub_decider;
// // pub mod sub_decider_loop_v4;
pub mod arg_handler;
pub mod config;
// pub mod data_provider;
// pub mod decider;
// pub mod error;
// pub mod examples;
// pub mod html;
// pub mod machine;
// pub mod machine_binary;
// pub mod machine_info;
// pub mod pre_decider;
// pub mod reporter;
// pub mod single_thread_worker;
// pub mod status;
// pub mod step_record;
// pub mod tape;
// pub mod transition_binary;
pub mod machine_generic;
pub mod transition_generic;
pub mod utils;

// /// This is used to define the CPU usage during generator and decider run.
// // TODO possibly move CPU percent into this enum, remove from Config
// pub enum CoreUsage {
//     SingleCore,
//     SingleCoreGeneratorMultiCoreDecider,
//     MultiCore,
// }
