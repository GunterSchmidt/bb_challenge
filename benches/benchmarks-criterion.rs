#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use bb_challenge::{
    config::{Config, CoreUsage, StepBig},
    data_provider::{
        enumerator::Enumerator,
        enumerator_binary::{EnumeratorBinary, EnumeratorType},
    },
    decider::{
        decider_engine, decider_hold_long::DeciderHoldLong, decider_hold_macro::DeciderHoldMacro,
        decider_result::result_max_steps_known, Decider, DeciderConfig, DeciderStandard,
    },
    machine_binary::{MachineId, NotableMachineBinary},
    status::MachineStatus,
};

const WARM_UP_TIME_MS: u64 = 500;
const MEASUREMENT_TIME_MS: u64 = 2000;
const BENCH_GENERATOR_BATCH_SIZE_REQUEST_FULL: usize = 500_000;
const BENCH_GENERATOR_BATCH_SIZE_REQUEST_REDUCED: usize = 1_000_000;
const GENERATOR_LIMIT: u64 = 50_000_000;

criterion_group!(
    benches,
    benchmark_tape_type,
    // benchmark_enumerator,
    // benchmark_decider_gen_bb3,
    // benchmark_decider_gen_bb4,
);
criterion_main!(benches);

fn benchmark_enumerator(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Enumerator Create Only");

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    group.bench_function("Enumerator full", |b| b.iter(|| bench_generate_full()));
    group.bench_function("Enumerator reduced forward", |b| {
        b.iter(|| bench_generate_reduced_forward())
    });
    // group.bench_function("Enumerator reduced backward", |b| {
    //     b.iter(|| bench_generate_reduced_backward())
    // });

    group.finish();
}

fn benchmark_decider_gen_bb3(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Decider Loop BB3");
    let config = config_bench(3);
    let dc_cycler: DeciderConfig<'_> = DeciderStandard::Cycler.decider_config(&config);

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    // full single
    group.bench_function(
        "Decider Cycler (Data Provider Enumerator Full) Single BB3",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorFullBackward,
                    CoreUsage::SingleCore,
                )
            })
        },
    );

    // reduced single
    group.bench_function(
        "Decider Cycler (Data Provider Enumerator Reduced Forward) Single BB3",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorReducedForward,
                    CoreUsage::SingleCore,
                )
            })
        },
    );

    // // reduced single
    // group.bench_function(
    //     "Decider Cycler (Data Provider Enumerator Reduced Backward) Single BB3",
    //     |b| {
    //         b.iter(|| {
    //             bench_decider_data_provider_gen(
    //                 &dc_cycler,
    //                 &config,
    //                 EnumeratorType::EnumeratorReducedBackward,
    //                 CoreUsage::SingleCore,
    //             )
    //         })
    //     },
    // );

    // full threaded
    group.bench_function("Decider Cycler (Enumerator Full) Threaded BB3", |b| {
        b.iter(|| {
            bench_decider_data_provider_gen(
                &dc_cycler,
                &config,
                EnumeratorType::EnumeratorFullBackward,
                CoreUsage::MultiCore,
            )
        })
    });

    // full reduced
    group.bench_function(
        "Decider Cycler (Enumerator Reduced Forward) Threaded BB3",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorReducedForward,
                    CoreUsage::MultiCore,
                )
            })
        },
    );

    // // full reduced
    // group.bench_function(
    //     "Decider Cycler (Enumerator Reduced Backward) Threaded BB3",
    //     |b| {
    //         b.iter(|| {
    //             bench_decider_data_provider_gen(
    //                 &dc_cycler,
    //                 &config,
    //                 EnumeratorType::EnumeratorReducedBackward,
    //                 CoreUsage::MultiCore,
    //             )
    //         })
    //     },
    // );

    group.finish();
}

fn benchmark_decider_gen_bb4(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Decider Loop BB4");
    let config = config_bench(4);
    let dc_cycler: DeciderConfig<'_> = DeciderStandard::Cycler.decider_config(&config);

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    // full single
    group.bench_function(
        "Decider Cycler (Data Provider Enumerator Full) Single BB4",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorFullBackward,
                    CoreUsage::SingleCore,
                )
            })
        },
    );

    // reduced single
    group.bench_function(
        "Decider Cycler (Data Provider Enumerator Reduced Forward) Single BB4",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorReducedForward,
                    CoreUsage::SingleCore,
                )
            })
        },
    );

    // group.bench_function(
    //     "Decider Cycler (Data Provider Enumerator Reduced Backward) Single BB4",
    //     |b| {
    //         b.iter(|| {
    //             bench_decider_data_provider_gen(
    //                 &dc_cycler,
    //                 &config,
    //                 EnumeratorType::EnumeratorReducedBackward,
    //                 CoreUsage::SingleCore,
    //             )
    //         })
    //     },
    // );

    // full threaded
    group.bench_function("Decider (Enumerator Full) Threaded BB4", |b| {
        b.iter(|| {
            bench_decider_data_provider_gen(
                &dc_cycler,
                &config,
                EnumeratorType::EnumeratorFullBackward,
                CoreUsage::MultiCore,
            )
        })
    });

    // reduced threaded
    group.bench_function(
        "Decider (Data Provider Enumerator Reduced) Threaded BB4",
        |b| {
            b.iter(|| {
                bench_decider_data_provider_gen(
                    &dc_cycler,
                    &config,
                    EnumeratorType::EnumeratorReducedForward,
                    CoreUsage::MultiCore,
                )
            })
        },
    );

    group.finish();
}

fn benchmark_tape_type(c: &mut Criterion) {
    // let input = aoc_file_reader::read_file(FILENAME_PART_1);
    // machine_bb5_max.step_limit = 50_000_000;
    let machine_bb4_max = NotableMachineBinary::BB4Max.machine_id();
    let machine_bb5_max = NotableMachineBinary::BB5Max.machine_id();
    let config_4 = Config::new_default(4);
    let config_5 = Config::new_default(5);
    let mut decider_hold_long_4 = DeciderHoldLong::new(&config_4);
    let mut decider_hold_macro_4 = DeciderHoldMacro::new(&config_4);
    let mut decider_hold_long_5 = DeciderHoldLong::new(&config_5);
    let mut decider_hold_macro_5 = DeciderHoldMacro::new(&config_5);

    // c.bench_function("first deciders", |b| b.iter(|| run_decider_first()));
    // // c.bench_function("first deciders", |b| b.iter(|| test()));

    let mut group = c.benchmark_group("Bench Tape Type");

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    // group.sample_size(50);

    // group.bench_function("u64 hold decider BB4 max function", |b| {
    //     b.iter(|| bench_decider_hold_u64_bb4_max_function(&machine_p_bb4_max))
    // });
    // Removing 'u64 hold decider BB4 max object' from the test
    // results in a 50% higher run-time of 'u128 hold decider BB4 max object'?!
    // Only if sample_size is 50,
    // group.bench_function("u64 hold decider BB4 max object", |b| {
    //     b.iter(|| bench_decider_hold_u64_bb4_max_object(&machine_p_bb4_max))
    // });
    // group.bench_function("u128 hold decider BB4 max function", |b| {
    //     b.iter(|| bench_decider_hold_u128_function(&machine_p_bb4_max, 107))
    // });
    // group.bench_function("u128 hold decider BB4 max object", |b| {
    //     b.iter(|| bench_decider_hold_u128_object(&machine_p_bb4_max, 107))
    // });

    // group.bench_function("Create Config", |b| b.iter(|| Config::new_default(4)));

    group.bench_function("u128 long hold decider Bb4Max", |b| {
        b.iter(|| bench_decider_hold_u128_long(&machine_bb4_max, 4, 107))
    });
    group.bench_function("u128 long hold decider Bb5Max", |b| {
        b.iter(|| bench_decider_hold_u128_long(&machine_bb5_max, 5, 47176870))
    });

    group.bench_function("decider hold long Bb4Max single", |b| {
        b.iter(|| decider_hold_long_4.decide_machine(&machine_bb4_max))
    });
    group.bench_function("decider hold macro Bb4Max single", |b| {
        b.iter(|| decider_hold_macro_4.decide_machine(&machine_bb4_max))
    });

    // group.bench_function("decider hold long Bb5Max single", |b| {
    //     b.iter(|| decider_hold_long_5.decide_machine(&machine_bb5_max))
    // });
    // group.bench_function("decider hold macro Bb5Max single", |b| {
    //     b.iter(|| decider_hold_macro_5.decide_machine(&machine_bb5_max))
    // });

    //     // fair comparison, u128 would run longer as it can handle more steps
    //     machine_bb5_max.step_limit = 300;
    //     group.bench_function("u64 hold old BB5 max 300 steps", |b| {
    //         b.iter(|| bench_decider_hold_u64_applies_not_bb5_max(&machine_bb5_max))
    //     });
    //     group.bench_function("u128 hold old BB5 max 300 steps", |b| {
    //         b.iter(|| bench_decider_hold_u128_old_applies_not_bb5_max(&machine_bb5_max))
    //     });
    //
    //     machine_bb5_max.step_limit = 50_000_000;
    //     group.bench_function("u128 hold old BB5 max", |b| {
    //         b.iter(|| bench_decider_hold_u128_old_applies_not_bb5_max(&machine_bb5_max))
    //     });
    //     group.bench_function("u128 hold BB5 max", |b| {
    //         b.iter(|| bench_decider_hold_u128_applies_not_bb5_max(&machine_bb5_max))
    //     });
    //     group.bench_function("u64 then u128 hold BB5 max", |b| {
    //         b.iter(|| bench_decider_hold_u64_u128_applies_not_bb5_max(&machine_bb5_max))
    //     });

    group.finish();
}

// fn bench_decider_hold_u64_bb4_max_function(machine: &Machine) {
//     let config = Config::new_default(machine.n_states());
//     let check_result = DeciderU64::check_hold(&machine, &config);
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(107));
// }
//
// fn bench_decider_hold_u64_bb4_max_object(machine: &Machine) {
//     let config = Config::new_default(machine.n_states());
//     let mut d = DeciderU64::new(&machine, &config);
//     let check_result = d.run_check_hold();
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(107));
// }
//
// fn bench_decider_hold_u128_function(machine: &Machine, steps: StepType) {
//     let config = Config::new_default(machine.n_states());
//     let check_result = DeciderU128::check_hold(&machine, &config);
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
// }
//
// fn bench_decider_hold_u128_object(machine: &Machine, steps: StepType) {
//     let config = Config::new_default(machine.n_states());
//     let mut d = DeciderU128::new(&machine, &config);
//     let check_result = d.run_check_hold();
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
// }

fn bench_decider_hold_u128_long(machine: &MachineId, n_states: usize, steps_result: StepBig) {
    let config = Config::new_default(n_states);
    let check_result = DeciderHoldLong::decide_single_machine(&machine, &config);
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHalts(steps_result));
}

// fn bench_decider_hold_u64_applies_not_bb5_max(machine: &Machine) {
//     // BB5 Max
//     let config = Config::new_default(machine.n_states());
//     let mut d = DeciderU64::new(&machine, &config);
//     let check_result = d.run_check_hold();
//     // println!("{}", check_result);
//     let okay = match check_result {
//         MachineStatus::Undecided(_, _, _) => true,
//         _ => false,
//     };
//
//     assert!(okay);
// }

// fn bench_decider_hold_u128_applies_not_bb5_max(machine: &Machine) {
//     // BB5 Max
//     let config = Config::new_default(machine.n_states());
//     let mut decider = DeciderU128::new(&machine, &config);
//     let check_result = decider.run_check_hold();
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::UndecidedFastTapeBoundReached);
// }

// pub fn bench_decider_hold_u64_u128_applies_not_bb5_max(machine: &MachineCompact) {
//     // BB5 Max
//     let mut decider_u64 = bb_challenge::decider_u64::DeciderU64::new(&machine);
//     let mut check_result = decider_u64.run_check_hold();
//     if check_result == MachineStatus::UndecidedFastTapeBoundReached {
//         // let mut decider = DeciderU128::new(&machine_bb5_max);
//         let mut decider = bb_challenge::decider_u128::DeciderU128::new_handover_u64(&decider_u64);
//         check_result = decider.run_check_hold();
//     }
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::UndecidedFastTapeBoundReached);
// }

fn bench_generate_full() {
    let mut enumerator =
        EnumeratorBinary::new(EnumeratorType::EnumeratorFullBackward, &config_bench(5));
    loop {
        let (_permutations, is_last_batch) = enumerator.enumerate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

fn bench_generate_reduced_forward() {
    let mut enumerator =
        EnumeratorBinary::new(EnumeratorType::EnumeratorReducedForward, &config_bench(5));
    loop {
        let (_permutations, is_last_batch) = enumerator.enumerate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

// fn bench_generate_reduced_backward() {
//     let mut enumerator =
//         EnumeratorBinary::new(EnumeratorType::EnumeratorReducedBackward, &config_bench(5));
//     loop {
//         let (_permutations, is_last_batch) = enumerator.enumerate_permutation_batch_next();
//         if is_last_batch {
//             break;
//         }
//     }
// }

fn bench_decider_data_provider_gen(
    dc_cycler: &DeciderConfig<'_>,
    config: &Config,
    enumeration_type: EnumeratorType,
    cores: CoreUsage,
) {
    // let dc_cycler: DeciderConfig<'_> = DeciderStandard::Cycler.decider_config(config);
    let dc_cycler = dc_cycler.clone();
    let enumerator = EnumeratorBinary::new(enumeration_type, config);
    let result = match cores {
        CoreUsage::SingleCore => {
            decider_engine::batch_run_decider_chain_data_provider_single_thread_reporting(
                &vec![dc_cycler],
                enumerator,
                None,
            )
        }
        CoreUsage::SingleCoreEnumeratorMultiCoreDecider => {
            decider_engine::batch_run_decider_chain_threaded_data_provider_single_thread_reporting(
                &vec![dc_cycler],
                enumerator,
                None,
            )
        }
        CoreUsage::MultiCore => {
            decider_engine::batch_run_decider_chain_threaded_data_provider_multi_thread_reporting(
                &vec![dc_cycler],
                enumerator,
                None,
            )
        }
    };
    // println!("{}", result);
    let n_states = config.n_states();
    if n_states <= 3 {
        assert_eq!(result_max_steps_known(n_states), result.steps_max());
    }
}

fn config_bench(n_states: usize) -> Config {
    Config::builder(n_states)
        .enumerator_full_batch_size_request(BENCH_GENERATOR_BATCH_SIZE_REQUEST_FULL)
        .enumerator_reduced_batch_size_request(BENCH_GENERATOR_BATCH_SIZE_REQUEST_REDUCED)
        .machine_limit(GENERATOR_LIMIT)
        .cpu_utilization(100)
        .build()
}
