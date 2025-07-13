#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use bb_challenge::{
    config::{Config, StepTypeBig},
    decider::{Decider, DeciderStandard},
    decider_engine::{self},
    decider_result::result_max_steps_known,
    generator::Generator,
    generator_full::GeneratorFull,
    generator_reduced::GeneratorReduced,
    machine::Machine,
    status::MachineStatus,
    Cores,
};

const WARM_UP_TIME_MS: u64 = 500;
const MEASUREMENT_TIME_MS: u64 = 2000;
const BENCH_GENERATOR_BATCH_SIZE_REQUEST_FULL: usize = 500_000;
const BENCH_GENERATOR_BATCH_SIZE_REQUEST_REDUCED: usize = 1_000_000;
const GENERATOR_LIMIT: u64 = 50_000_000;

criterion_group!(
    benches,
    benchmark_tape_type,
    // benchmark_generator,
    // benchmark_decider_gen_bb3,
    // benchmark_decider_gen_bb4,
);
criterion_main!(benches);

fn benchmark_generator(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Generator Create Only");

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    group.bench_function("Generator full", |b| b.iter(|| bench_generate_full()));
    group.bench_function("Generator reduced", |b| b.iter(|| bench_generate_reduced()));

    group.finish();
}

fn benchmark_decider_gen_bb3(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Decider Loop BB3");
    let config = config_bench(3);

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    // full single
    group.bench_function("Decider (Data Provider Generator Full) BB3", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, false, Cores::SingleCore))
    });

    // reduced single
    group.bench_function("Decider (Data Provider Generator Reduced) BB3", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, true, Cores::SingleCore))
    });

    // full threaded
    group.bench_function("Decider (Generator Full) Threaded BB3", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, false, Cores::MultiCore))
    });

    // full reduced
    group.bench_function("Decider (Generator Reduced) Threaded BB3", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, true, Cores::MultiCore))
    });

    group.finish();
}

fn benchmark_decider_gen_bb4(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bench Decider Loop BB4");
    let config = config_bench(4);

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    // full single
    group.bench_function("Decider V2 (Data Provider Generator Full) BB4", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, false, Cores::SingleCore))
    });

    // reduced single
    group.bench_function("Decider V2 (Data Provider Generator Reduced) BB4", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, true, Cores::SingleCore))
    });

    // full threaded
    group.bench_function("Decider (Generator Full) Threaded BB4", |b| {
        b.iter(|| bench_decider_data_provider_gen(&config, false, Cores::MultiCore))
    });

    // reduced threaded
    group.bench_function(
        "Decider (Data Provider Generator Reduced) Threaded BB4",
        |b| b.iter(|| bench_decider_data_provider_gen(&config, true, Cores::MultiCore)),
    );

    group.finish();
}

fn benchmark_tape_type(c: &mut Criterion) {
    // let input = aoc_file_reader::read_file(FILENAME_PART_1);
    // let machine_bb4_max = MachineCompact::build_machine("BB4_MAX").unwrap();
    // let mut machine_bb5_max = MachineCompactDeprecated::build_machine("BB5_MAX").unwrap();
    // machine_bb5_max.step_limit = 50_000_000;
    let machine_p_bb4_max = Machine::build_machine("BB4_MAX").unwrap();
    let machine_p_bb5_max = Machine::build_machine("BB5_MAX").unwrap();

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
    group.bench_function("u128 long hold decider BB4 max", |b| {
        b.iter(|| bench_decider_hold_u128_long_v2(&machine_p_bb4_max, 4, 107))
    });
    // group.bench_function("u128 long hold decider BB5 max V1", |b| {
    //     b.iter(|| bench_decider_hold_u128_long_v1(&machine_bb5_max, 47176870))
    // });
    group.bench_function("u128 long hold decider BB5 max", |b| {
        b.iter(|| bench_decider_hold_u128_long_v2(&machine_p_bb5_max, 5, 47176870))
    });

    group.bench_function("u128 long hold decider BB4 max V3", |b| {
        b.iter(|| bench_decider_hold_u128_long_v3(&machine_p_bb4_max, 4, 107))
    });
    group.bench_function("u128 long hold decider BB5 max V3", |b| {
        b.iter(|| bench_decider_hold_u128_long_v3(&machine_p_bb5_max, 5, 47176870))
    });

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

// fn bench_decider_hold_u128_long_v1(machine: &MachineCompactDeprecated, steps: StepType) {
//     let mut d = bb_challenge::decider_u128_long_v1::DeciderU128LongV1::new(&machine);
//     let check_result = d.run_check_hold();
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
// }

fn bench_decider_hold_u128_long_v2(machine: &Machine, n_states: usize, steps_result: StepTypeBig) {
    let config = Config::new_default(n_states);
    // let mut d = bb_challenge::decider_u128_long::DeciderU128Long::new(&machine, STEP_LIMIT_DEFAULT);
    let check_result =
        bb_challenge::decider_hold_u128_long_v2::DeciderHoldU128Long::decide_single_machine(
            &machine, &config,
        );
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(steps_result));
}

fn bench_decider_hold_u128_long_v3(machine: &Machine, n_states: usize, steps_result: StepTypeBig) {
    let config = Config::new_default(n_states);
    // let mut d = bb_challenge::decider_u128_long::DeciderU128Long::new(&machine, STEP_LIMIT_DEFAULT);
    let check_result =
        bb_challenge::decider_hold_u128_long_v3::DeciderHoldU128Long::decide_single_machine(
            &machine, &config,
        );
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(steps_result));
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
    let mut generator = GeneratorFull::new(&config_bench(5));
    loop {
        let (_permutations, is_last_batch) = generator.generate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

fn bench_generate_reduced() {
    let mut generator = GeneratorReduced::new(&config_bench(5));
    loop {
        let (_permutations, is_last_batch) = generator.generate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

fn bench_decider_data_provider_gen(config: &Config, gen_reduced: bool, cores: Cores) {
    let dc_cycler = DeciderStandard::Cycler.decider_config(config);
    let result = if gen_reduced {
        match cores {
            Cores::SingleCore => {
                decider_engine::run_decider_chain_data_provider_single_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorReduced::new(config),
                    None,
                )
            }
            Cores::SingleCoreGeneratorMultiCoreDecider => {
                decider_engine::run_decider_chain_threaded_data_provider_single_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorReduced::new(config),
                    None,
                )
            }
            Cores::MultiCore => {
                decider_engine::run_decider_chain_threaded_data_provider_multi_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorReduced::new(config),
                    None,
                )
            }
        }
    } else {
        match cores {
            Cores::SingleCore => {
                decider_engine::run_decider_chain_data_provider_single_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorFull::new(config),
                    None,
                )
            }
            Cores::SingleCoreGeneratorMultiCoreDecider => {
                decider_engine::run_decider_chain_threaded_data_provider_single_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorFull::new(config),
                    None,
                )
            }
            Cores::MultiCore => {
                decider_engine::run_decider_chain_threaded_data_provider_multi_thread_reporting(
                    &vec![dc_cycler],
                    GeneratorFull::new(config),
                    None,
                )
            }
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
        .generator_full_batch_size_request(BENCH_GENERATOR_BATCH_SIZE_REQUEST_FULL)
        .generator_reduced_batch_size_request(BENCH_GENERATOR_BATCH_SIZE_REQUEST_REDUCED)
        .machine_limit(GENERATOR_LIMIT)
        .cpu_utilization(100)
        .build()
}
