#![allow(dead_code)]

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use bb_challenge::{
    config::Config,
    decider::{self, run_decider_generator_single_thread, Decider},
    decider_loop_v4::{DeciderLoopV4, STEP_LIMIT_DECIDER_LOOP},
    decider_u128::DeciderU128,
    decider_u64::DeciderU64,
    generator::Generator,
    generator_full::GeneratorFull,
    generator_reduced::GeneratorReduced,
    machine::Machine,
    result::result_max_steps_known,
    status::MachineStatus,
    sub_decider::SubDeciderDummy,
    StepType, GENERATOR_BATCH_SIZE_RECOMMENDATION,
};

const WARM_UP_TIME_MS: u64 = 500;
const MEASUREMENT_TIME_MS: u64 = 2000;
const GENERATOR_BATCH_SIZE_REQUEST: usize = GENERATOR_BATCH_SIZE_RECOMMENDATION;

criterion_group!(
    benches,
    benchmark_tape_type,
    // benchmark_generator,
    // benchmark_decider_gen_bb3,
);
criterion_main!(benches);

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

    group.bench_function("u64 hold decider BB4 max function", |b| {
        b.iter(|| bench_decider_hold_u64_bb4_max_function(&machine_p_bb4_max))
    });
    // Removing 'u64 hold decider BB4 max object' from the test
    // results in a 50% higher run-time of 'u128 hold decider BB4 max object'?!
    // Only if sample_size is 50,
    // group.bench_function("u64 hold decider BB4 max object", |b| {
    //     b.iter(|| bench_decider_hold_u64_bb4_max_object(&machine_p_bb4_max))
    // });
    group.bench_function("u128 hold decider BB4 max function", |b| {
        b.iter(|| bench_decider_hold_u128_function(&machine_p_bb4_max, 107))
    });
    // group.bench_function("u128 hold decider BB4 max object", |b| {
    //     b.iter(|| bench_decider_hold_u128_object(&machine_p_bb4_max, 107))
    // });
    group.bench_function("u128 long hold decider BB4 max", |b| {
        b.iter(|| bench_decider_hold_u128_long(&machine_p_bb4_max, 4, 107))
    });
    // group.bench_function("u128 long hold decider BB5 max V1", |b| {
    //     b.iter(|| bench_decider_hold_u128_long_v1(&machine_bb5_max, 47176870))
    // });
    group.bench_function("u128 long hold decider BB5 max", |b| {
        b.iter(|| bench_decider_hold_u128_long(&machine_p_bb5_max, 5, 47176870))
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

fn benchmark_generator(c: &mut Criterion) {
    const GENERATOR_LIMIT: u64 = 50_000_000;

    let mut group = c.benchmark_group("Bench Generator Create Only");

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    group.bench_function("Generator full", |b| {
        b.iter(|| bench_generate_full(GENERATOR_LIMIT))
    });
    group.bench_function("Generator reduced", |b| {
        b.iter(|| bench_generate_reduced(GENERATOR_LIMIT))
    });

    group.finish();
}

fn benchmark_decider_gen_bb3(c: &mut Criterion) {
    const GENERATOR_LIMIT: u64 = 5_000_000;

    let mut group = c.benchmark_group("Bench Decider (Generator Full)");

    group.warm_up_time(Duration::from_millis(WARM_UP_TIME_MS));
    // group.measurement_time(Duration::from_millis(MEASUREMENT_TIME_MS));
    group.sample_size(10);

    // group.bench_function("Decider (Generator Full) BB3", |b| {
    //     b.iter(|| bench_decider_generator_full(3))
    // });
    group.bench_function("Decider P (Generator Full) BB3", |b| {
        b.iter(|| bench_decider_generator_full(3))
    });
    // group.bench_function("Decider (Generator Reduced) BB3", |b| {
    //     b.iter(|| bench_decider_generator_reduced(3))
    // });
    group.bench_function("Decider P (Generator Reduced) BB3", |b| {
        b.iter(|| bench_decider_generator_reduced_p(3))
    });
    group.bench_function("Decider (Generator Full) Threaded BB3", |b| {
        b.iter(|| bench_decider_generator_full_threaded(3))
    });
    group.bench_function("Decider (Generator Reduced) Threaded BB3", |b| {
        b.iter(|| bench_decider_generator_reduced_threaded(3))
    });

    group.finish();
}

fn bench_decider_hold_u64_bb4_max_function(machine: &Machine) {
    let config = Config::new_default(machine.n_states());
    let check_result = DeciderU64::check_hold(&machine, &config);
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(107));
}

fn bench_decider_hold_u64_bb4_max_object(machine: &Machine) {
    let config = Config::new_default(machine.n_states());
    let mut d = DeciderU64::new(&machine, &config);
    let check_result = d.run_check_hold();
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(107));
}

fn bench_decider_hold_u128_function(machine: &Machine, steps: StepType) {
    let config = Config::new_default(machine.n_states());
    let check_result = DeciderU128::check_hold(&machine, &config);
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
}

fn bench_decider_hold_u128_object(machine: &Machine, steps: StepType) {
    let config = Config::new_default(machine.n_states());
    let mut d = DeciderU128::new(&machine, &config);
    let check_result = d.run_check_hold();
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
}

// fn bench_decider_hold_u128_long_v1(machine: &MachineCompactDeprecated, steps: StepType) {
//     let mut d = bb_challenge::decider_u128_long_v1::DeciderU128LongV1::new(&machine);
//     let check_result = d.run_check_hold();
//     // println!("{}", check_result);
//     assert_eq!(check_result, MachineStatus::DecidedHolds(steps));
// }

fn bench_decider_hold_u128_long(machine: &Machine, n_states: usize, steps_result: StepType) {
    let config = Config::new_default(n_states);
    let mut d: bb_challenge::decider_u128_long::DeciderU128Long<SubDeciderDummy> =
        bb_challenge::decider_u128_long::DeciderU128Long::new(&config);
    // let mut d = bb_challenge::decider_u128_long::DeciderU128Long::new(&machine, STEP_LIMIT_DEFAULT);
    let check_result = d.decide_machine(machine);
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(steps_result));
}

fn bench_decider_hold_u64_applies_not_bb5_max(machine: &Machine) {
    // BB5 Max
    let config = Config::new_default(machine.n_states());
    let mut d = DeciderU64::new(&machine, &config);
    let check_result = d.run_check_hold();
    // println!("{}", check_result);
    let okay = match check_result {
        MachineStatus::Undecided(_, _, _) => true,
        _ => false,
    };

    assert!(okay);
}

fn bench_decider_hold_u128_applies_not_bb5_max(machine: &Machine) {
    // BB5 Max
    let config = Config::new_default(machine.n_states());
    let mut decider = DeciderU128::new(&machine, &config);
    let check_result = decider.run_check_hold();
    // println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::UndecidedFastTapeBoundReached);
}

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

fn bench_generate_full(generate_limit: u64) {
    let config = Config::builder(5).generate_limit(generate_limit).build();
    let mut generator = GeneratorFull::new(&config);
    loop {
        let (_permutations, is_last_batch) = generator.generate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

fn bench_generate_reduced(generate_limit: u64) {
    let config = Config::builder(5).generate_limit(generate_limit).build();
    let mut generator = GeneratorReduced::new(&config);
    loop {
        let (_permutations, is_last_batch) = generator.generate_permutation_batch_next();
        if is_last_batch {
            break;
        }
    }
}

fn bench_decider_generator_full(n_states: usize) {
    let config = Config::new_default(n_states);
    let generator = GeneratorFull::new(&config);
    let decider = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
    let result = run_decider_generator_single_thread(decider, generator);
    // println!("{}", result);
    assert_eq!(result_max_steps_known(n_states), result.steps_max());
}

fn bench_decider_generator_full_threaded(n_states: usize) {
    let config = Config::new_default(n_states);
    let generator = GeneratorFull::new(&config);
    let decider = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
    let result = decider::run_decider_generator_threaded(decider, generator, 100);
    // println!("{}", result);
    assert_eq!(result_max_steps_known(n_states), result.steps_max());
}

// fn bench_decider_generator_reduced(n_states: usize) {
//     let generator = GeneratorReduced::new(
//         n_states,
//         GENERATOR_BATCH_SIZE_RECOMMENDATION,
//         limit(n_states),
//     );
//     let result = run_decider_generator(generator);
//     // println!("{}", result);
//     assert_eq!(result_max_steps(n_states), result.steps_max);
// }

fn bench_decider_generator_reduced_p(n_states: usize) {
    let config = Config::new_default(n_states);
    let generator = GeneratorReduced::new(&config);
    let decider = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
    let result = run_decider_generator_single_thread(decider, generator);
    // println!("{}", result);
    assert_eq!(result_max_steps_known(n_states), result.steps_max());
}

fn bench_decider_generator_reduced_threaded(n_states: usize) {
    let config = Config::new_default(n_states);
    let generator = GeneratorReduced::new(&config);
    let decider = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
    let result = decider::run_decider_generator_threaded(decider, generator, 100);
    // println!("{}", result);
    assert_eq!(result_max_steps_known(n_states), result.steps_max());
}
