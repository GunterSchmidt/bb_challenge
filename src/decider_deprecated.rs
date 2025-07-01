#![allow(deprecated)]
use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::Config,
    data_provider::DataProvider,
    data_provider_threaded::DataProviderThreaded,
    decider::{ThreadResultDataProvider, ThreadResultDecider},
    decider_result::{
        BatchResult, DeciderResultStats, DurationDataProvider, EndReason, ResultBatchInfo,
    },
    generator::Generator,
    machine::Machine,
    pre_decider::PreDeciderRun,
    reporter::Reporter,
    status::MachineStatus,
    utils::num_cpus_percentage,
};

pub trait DeciderDeprecated {
    // TODO remove
    /// Creates a new decider. Allows individual initialization code for each decider and is called for each permutation batch.
    fn new_decider(&self) -> Self;

    /// Returns the result of this decider for one single machine. \
    /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
    fn decide_machine_deprecated(&mut self, machine: &Machine) -> MachineStatus;

    /// Allows to inefficiently test a single machine.
    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus;

    fn decider_run_batch(
        machines: &[Machine],
        run_predecider: PreDeciderRun,
        config: &Config,
    ) -> Option<BatchResult>;

    /// Returns the name of this decider
    fn name_deprecated(&self) -> &str;
}

// #[deprecated]
pub fn run_decider_generator_single_thread_deprecated(
    decider: impl DeciderDeprecated,
    generator: impl Generator,
) -> DeciderResultStats {
    run_decider_generator_single_thread_reporting(decider, generator, Some(Reporter::default()))
}

pub fn run_decider_generator_single_thread_reporting(
    decider: impl DeciderDeprecated,
    mut generator: impl Generator,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats {
    let start = Instant::now();
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    let n_states = generator.n_states();
    generator.check_generator_batch_size_request_single_thread();
    let mut result = DeciderResultStats::new_deprecated(n_states, 0);
    let requires_pre_decider_check = generator.requires_pre_decider_check();
    result.set_record_machines_max_steps(generator.config().limit_machines_max_steps());
    result.set_limit_machines_undecided(generator.config().limit_machines_undecided());
    // let mut batch_no = 0;
    loop {
        let start_gen = Instant::now();
        let (machines, is_last_batch) = generator.generate_permutation_batch_next();
        // batch_no += 1;
        // if batch_no % 100 == 0 {
        //     println!("Batch no. {batch_no} / {}", generator.num_batches());
        // }
        result.add_pre_decider_count(&generator.pre_decider_count());
        result.add_total(generator.num_eliminated());
        duration_generator += start_gen.elapsed();
        let start_decider = Instant::now();
        let decider = decider.new_decider();
        let r = if requires_pre_decider_check != PreDeciderRun::DoNotRun {
            decider_batch_run_with_pre_deciders(decider, &machines, &result.batch_info())
        } else {
            decider_batch_run_without_pre_deciders(decider, &machines, &result.batch_info())
        };
        let undecided_has_room = result.add_result(&r);
        duration_decider += start_decider.elapsed();

        if is_last_batch {
            result.end_reason = EndReason::AllMachinesChecked;
            break;
        }
        if !undecided_has_room {
            result.end_reason = EndReason::UndecidedLimitReached(result.limit_machines_undecided());
            break;
        }

        // Output info on progress
        if let Some(reporter) = reporter.as_mut() {
            if reporter.is_due_progress() {
                let s = reporter.report(result.num_checked_total(), generator.limit(), &result);
                println!("{s}");
            }
        }
    }
    result.duration = DurationDataProvider {
        duration_data_provider: duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };

    // Add the name at the end or it will result in a little performance loss. Reason unknown.
    result.add_name(&(format!("BB{}: ", n_states) + decider.name_deprecated()));

    result
}

/// Runs the check in separate threads using the standard reporter.  
/// The generation of the permutations is not threaded.
pub fn run_decider_generator_threaded_deprecated<D, G>(
    decider: D,
    generator: G,
) -> DeciderResultStats
where
    D: DeciderDeprecated + Send + 'static,
    G: Generator + Send + 'static,
{
    run_decider_generator_threaded_reporting(decider, generator, Some(Reporter::default()))
}

/// Runs the check in separate threads using a custom reporter (or None to omit reporting).
/// The generation of the permutations is not threaded.
// Contains a lot of code to optimize thread usage.
pub fn run_decider_generator_threaded_reporting<D, G>(
    decider: D,
    generator: G,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats
where
    D: DeciderDeprecated + Send + 'static,
    G: Generator + Send + 'static,
{
    let max_threads = num_cpus_percentage(generator.config().cpu_utilization_percent());
    // if single thread run single
    if max_threads == 1 {
        return run_decider_generator_single_thread_reporting(decider, generator, reporter);
    }

    let start = Instant::now();
    let n_states = generator.n_states();
    let mut result =
        DeciderResultStats::new_deprecated(n_states, DeciderResultStats::init_steps_max(n_states));
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    result.set_record_machines_max_steps(generator.config().limit_machines_max_steps());
    result.set_limit_machines_undecided(generator.config().limit_machines_undecided());
    let mut max_threads_gen = (max_threads / 2 + 1).max(1);
    let mut batch_no = 0;
    let (send_finished_thread_generator, receive_finished_thread_generator) =
        std::sync::mpsc::channel();
    let (send_finished_thread_decider, receive_finished_thread_decider) =
        std::sync::mpsc::channel::<ThreadResultDecider>();
    let mut num_threads_generator_running = 0;
    let mut num_threads_decider_running = 0;
    let mut buffer_gen_result = Vec::new();
    let max_buffer_gen = (max_threads - 1).max(max_threads + 4);
    let mut last_gen_thread_change_batch_no = 0;
    let mut last_buf_len = 0;
    let mut is_gen_finished = false;
    // let mut count_gen_spawn = 0;

    loop {
        // triggers a thread sleep if none have finished
        let mut do_sleep = true;
        if num_threads_generator_running < max_threads_gen
            && buffer_gen_result.len() < max_buffer_gen
        // && batch_no < generator.num_batches() // not required, checked within
        {
            do_sleep = false;
            num_threads_generator_running += 1;
            let send_finished_thread_gen = send_finished_thread_generator.clone();
            let mut generator_batch = generator.new_from_generator_deprecated();
            // count_gen_spawn += 1;
            // println!(
            //     "Generator batch {}/{} spawned",
            //     batch_no + 1,
            //     generator.num_batches(),
            // );
            // Handle not used as it would create a very large vector to maintain
            thread::spawn(move || {
                let start = Instant::now();
                let (permutations, _is_last_batch) =
                    generator_batch.generate_permutation_batch_no(batch_no);

                // Send finished signal to allow decider to run.
                let result = Box::new(ThreadResultDataProvider {
                    batch_no,
                    machines: permutations,
                    pre_decider_count: Some(generator_batch.pre_decider_count()),
                    duration: start.elapsed(),
                });
                // TODO sending result doubles the time as the data is copied within memory, or not? Should be moved.
                // Single thread is twice as fast, maybe cache issue
                // use Arc?
                // https://doc.rust-lang.org/rust-by-example/std/arc.html
                send_finished_thread_gen.send(result).unwrap();
            });
            batch_no += 1;
            if batch_no == generator.num_batches() {
                // turn off generator threads, they are not needed any more
                max_threads_gen = 0;
                is_gen_finished = true;
            }

            // Spawn parallel generators for all CPUs to build a buffer
            // if num_threads_generator_running < threads_generator
            //     && num_threads_decider_running + num_threads_generator_running < max_threads
            // {
            //     continue;
            // }
        }

        // print running threads information
        // #[cfg(all(debug_assertions, feature = "debug"))]
        // println!(
        //     "Threads {} / {max_threads} ({num_threads_generator_running}, {num_threads_decider_running}) - batch {batch_no}/{}, buffer {}",
        //     num_threads_generator_running + num_threads_decider_running,
        //     generator.num_batches(),
        //     buffer_gen_result.len(),
        // );

        // adjust threads between generator and decider to optimize usage, but not too often
        if batch_no - last_gen_thread_change_batch_no > max_threads_gen
            || (max_threads_gen == 0 && buffer_gen_result.len() < max_threads)
        {
            if buffer_gen_result.len() >= max_buffer_gen // buffer used up
                && buffer_gen_result.len() >= last_buf_len // changed buf len
                // keep one thread
                && max_threads_gen > 0
            {
                // generator is too fast, decider cannot keep up
                max_threads_gen -= 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads reduced to   {max_threads_gen} (batch no. {batch_no}), buffer gen {}", buffer_gen_result.len());
            } else if buffer_gen_result.len() < max_threads // low bound is buffer for all threads so decider always finds a buffered gen
                && buffer_gen_result.len() <= last_buf_len
                // keep one for decider
                && max_threads_gen < max_threads - 1
                // set to 0 when all batches have been generated
                && !is_gen_finished
            {
                max_threads_gen += 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads increased to {max_threads_gen} (batch no. {batch_no}), buffer gen {}", buffer_gen_result.len());
            }
        }

        // Wait until a permutation pack is available, then run decider. This also frees one CPU.
        if num_threads_generator_running > 0 {
            // if buffer_gen_result.is_empty() {
            //     // no data to work for decider, must wait
            //     let thread_result_gen = receive_finished_thread_generator.recv().unwrap();
            //     duration_generator += thread_result_gen.duration;
            //     buffer_gen_result.push(*thread_result_gen);
            //     num_threads_generator_running -= 1;
            //     has_finished = true;
            // } else {
            // collect all finished permutation batches
            while let Ok(thread_result_gen) = receive_finished_thread_generator.try_recv() {
                duration_generator += thread_result_gen.duration;
                buffer_gen_result.push(*thread_result_gen);
                num_threads_generator_running -= 1;
                do_sleep = false;
            }
            // }
        }

        // Check if new decider thread can be started
        // check available threads, keep one open for next generator
        if !buffer_gen_result.is_empty()
            && max_threads - max_threads_gen.max(num_threads_generator_running)
                > num_threads_decider_running
        {
            // Thread is available, start decider
            do_sleep = false;
            num_threads_decider_running += 1;
            let send_finished_thread_dec = send_finished_thread_decider.clone();
            // move result out of vector to move into thread
            let gen_result = buffer_gen_result.remove(0);
            let batch_info = result.batch_info();
            let requires_pre_decider_run = generator.requires_pre_decider_check();
            let decider = decider.new_decider();
            // println!(
            //     "Decider batch {}/{} spawned, max steps; {}",
            //     gen_result.batch_no + 1,
            //     generator.num_batches(),
            //     batch_info.steps_max,
            // );
            thread::spawn(move || {
                let start = Instant::now();
                let mut dr = if requires_pre_decider_run != PreDeciderRun::DoNotRun {
                    decider_batch_run_with_pre_deciders(decider, &gen_result.machines, &batch_info)
                } else {
                    decider_batch_run_without_pre_deciders(
                        decider,
                        &gen_result.machines,
                        &batch_info,
                    )
                };
                if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                    dr.add_pre_decider_count(pre_decider_count);
                    dr.add_total(pre_decider_count.num_total());
                }
                let decider_result = ThreadResultDecider {
                    batch_no: gen_result.batch_no,
                    result: dr,
                    duration: start.elapsed(),
                };
                // println!(
                //     "Decider batch {}/{} finished send",
                //     decider_result.batch_no, num_batches
                // );
                send_finished_thread_dec.send(decider_result).unwrap();
            });
        }

        // Check if deciders have finished
        while let Ok(thread_result_dec) = receive_finished_thread_decider.try_recv() {
            result.add_result(&thread_result_dec.result);
            duration_decider += thread_result_dec.duration;
            num_threads_decider_running -= 1;
            // println!(
            //     "Decider batch {}/{} finished",
            //     thread_result_dec.batch_no + 1,
            //     generator.num_batches()
            // );

            // Output info on progress
            if let Some(reporter) = reporter.as_mut() {
                if reporter.is_due_progress() {
                    let s = reporter.report(result.num_checked_total(), generator.limit(), &result);
                    println!("{s}");
                }
            }
        }

        // check if finished all batches
        if num_threads_generator_running + num_threads_decider_running == 0
            && buffer_gen_result.is_empty()
        {
            if batch_no < generator.num_batches() {
                panic!(
                    "All empty! Threads max gen {max_threads_gen}, batch {batch_no}/{}, buffer {}",
                    generator.num_batches(),
                    buffer_gen_result.len(),
                );
            }
            break;
        }

        if do_sleep {
            // print!("w ");
            thread::sleep(Duration::from_micros(100));
        }
    }
    result.duration = DurationDataProvider {
        duration_data_provider: duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };
    result.add_name(&(format!("BB{}: ", n_states) + decider.name_deprecated() + " threaded"));

    //     println!("\n{}", result);
    //     if let Some(m) = result.machine_max_steps() {
    //         println!("Most Steps:\n{}", m);
    //     }
    //
    //     println!(
    //         "\nTotal time used for parallel run with {} machines: generator {:?}, decider {:?}, total time {:?}",
    //         result.num_checked, duration_generate, duration_run_batches, duration_total
    //     );

    result
}

pub fn run_decider_data_provider_single_thread_deprecated(
    decider: impl DeciderDeprecated,
    data_provider: impl DataProvider,
) -> DeciderResultStats {
    run_decider_data_provider_single_thread_reporting(
        decider,
        data_provider,
        Some(Reporter::default()),
    )
}

pub fn run_decider_data_provider_single_thread_reporting(
    decider: impl DeciderDeprecated,
    mut data_provider: impl DataProvider,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats {
    let start = Instant::now();
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    let n_states = data_provider.n_states();
    data_provider.set_batch_size_for_num_threads(1);
    let mut result = DeciderResultStats::new_deprecated(n_states, 0);
    let requires_pre_decider_check = data_provider.requires_pre_decider_check();
    result.set_record_machines_max_steps(data_provider.config().limit_machines_max_steps());
    result.set_limit_machines_undecided(data_provider.config().limit_machines_undecided());
    // let mut batch_no = 0;
    loop {
        let start_gen = Instant::now();
        let data = data_provider.machine_batch_next();
        // batch_no += 1;
        // if batch_no % 100 == 0 {
        //     println!("Batch no. {batch_no} / {}", generator.num_batches());
        // }
        if let Some(pre) = data.pre_decider_count {
            // println!("{pre}");
            result.add_pre_decider_count(&pre);
            result.add_total(pre.num_total());
        }
        duration_generator += start_gen.elapsed();
        let start_decider = Instant::now();
        let decider = decider.new_decider();
        let r = if requires_pre_decider_check != PreDeciderRun::DoNotRun {
            decider_batch_run_with_pre_deciders(decider, &data.machines, &result.batch_info())
        } else {
            decider_batch_run_without_pre_deciders(decider, &data.machines, &result.batch_info())
        };
        let undecided_available = result.add_result(&r);
        duration_decider += start_decider.elapsed();

        // end if undecided limit has been reached
        if !undecided_available {
            result.end_reason = EndReason::UndecidedLimitReached(result.limit_machines_undecided());
            break;
        }
        if data.end_reason == EndReason::IsLastBatch {
            result.end_reason = EndReason::AllMachinesChecked;
            break;
        }

        // Output info on progress
        if let Some(reporter) = reporter.as_mut() {
            if reporter.is_due_progress() {
                let s = reporter.report(
                    result.num_checked_total(),
                    data_provider.num_machines_total(),
                    &result,
                );
                println!("{s}");
            }
        }
    }
    result.duration = DurationDataProvider {
        duration_data_provider: duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };

    // Add the name at the end or it will result in a little performance loss. Reason unknown.
    result.add_name(&(format!("BB{}: ", n_states) + decider.name_deprecated()));

    result
}

// pub fn run_deciders_generator_single_thread(
//     decider_chain: Vec<DeciderEnum>,
//     generator: impl Generator,
// ) -> ResultDecider {
//     run_deciders_generator_single_thread_reporting(
//         decider_chain,
//         generator,
//         Some(Reporter::default()),
//     )
// }
//
// pub fn run_deciders_generator_single_thread_reporting(
//     decider_chain: Vec<DeciderEnum>,
//     mut generator: impl Generator,
//     mut reporter: Option<Reporter>,
// ) -> ResultDecider {
//     let start = Instant::now();
//     let mut duration_generator = Duration::default();
//     let mut duration_decider = Duration::default();
//     let n_states = generator.n_states();
//     generator.check_generator_batch_size_request_single_thread();
//     let mut result = ResultDecider::new(n_states, 0);
//     let requires_pre_decider_check = generator.requires_pre_decider_check();
//     result.set_record_machines_max_steps(generator.config().num_record_machines_max_steps());
//     result.set_record_machines_undecided(generator.config().num_record_machines_undecided());
//     // let mut batch_no = 0;
//     loop {
//         let start_gen = Instant::now();
//         let (machines, is_last_batch) = generator.generate_permutation_batch_next();
//         // batch_no += 1;
//         // if batch_no % 100 == 0 {
//         //     println!("Batch no. {batch_no} / {}", generator.num_batches());
//         // }
//         result.add_pre_decider_count(&generator.pre_decider_count());
//         result.add_total(generator.num_eliminated());
//         duration_generator += start_gen.elapsed();
//         let start_decider = Instant::now();
//         let decider: dyn Decider = match decider_chain[0] {
//             DeciderEnum::PreDecider(pre_decider) => pre_decider,
//             DeciderEnum::LoopV4(decider_loop_v4) => decider_loop_v4,
//             DeciderEnum::HoldLong(decider_hold_u128_long) => decider_hold_u128_long,
//         };
//         let r = if requires_pre_decider_check {
//             decider_batch_run_with_pre_deciders(decider, &machines, &result.batch_info())
//         } else {
//             decider_batch_run_without_pre_deciders(decider, &machines, &result.batch_info())
//         };
//         result.add_result(&r);
//         duration_decider += start_decider.elapsed();
//
//         if is_last_batch {
//             break;
//         }
//
//         // Output info on progress
//         if let Some(reporter) = reporter.as_mut() {
//             if reporter.is_due_progress() {
//                 let s = reporter.report(result.num_checked_total(), generator.limit(), &result);
//                 println!("{s}");
//             }
//         }
//     }
//     result.duration = DurationGenerator {
//         duration_generator,
//         duration_decider,
//         duration_total: start.elapsed(),
//     };
//
//     // Add the name at the end or it will result in a little performance loss. Reason unknown.
//     result.name = format!("BB{}: ", n_states) + decider_chain[0].name().as_str();
//
//     result
// }

/// Runs the check in separate threads using the standard reporter.
/// The generation of the permutations is not threaded.
pub fn run_decider_data_provider_threaded_deprecated<D, G>(
    decider: D,
    data_provider: G,
) -> DeciderResultStats
where
    D: DeciderDeprecated + Send + 'static,
    G: DataProviderThreaded + Send + 'static,
{
    run_decider_data_provider_threaded_reporting(decider, data_provider, Some(Reporter::default()))
}

/// Runs the check in separate threads using a custom reporter (or None to omit reporting).
/// The generation of the permutations is not threaded.
// Contains a lot of code to optimize thread usage.
pub fn run_decider_data_provider_threaded_reporting<D, G>(
    decider: D,
    data_provider: G,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats
where
    D: DeciderDeprecated + Send + 'static,
    G: DataProviderThreaded + Send + 'static,
{
    let max_threads = num_cpus_percentage(data_provider.config().cpu_utilization_percent());
    // if single thread run single
    if max_threads == 1 {
        return run_decider_data_provider_single_thread_reporting(decider, data_provider, reporter);
    }

    let start = Instant::now();
    let n_states = data_provider.n_states();
    let mut result =
        DeciderResultStats::new_deprecated(n_states, DeciderResultStats::init_steps_max(n_states));
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    result.set_record_machines_max_steps(data_provider.config().limit_machines_max_steps());
    result.set_limit_machines_undecided(data_provider.config().limit_machines_undecided());
    let mut max_threads_gen = (max_threads / 2 + 1).max(1);
    let mut batch_no = 0;
    let (send_finished_thread_data_provider, receive_finished_thread_data_provider) =
        std::sync::mpsc::channel();
    let (send_finished_thread_decider, receive_finished_thread_decider) =
        std::sync::mpsc::channel::<ThreadResultDecider>();
    let mut num_threads_data_provider_running = 0;
    let mut num_threads_decider_running = 0;
    let mut buffer_gen_result = Vec::new();
    let max_buffer_gen = (max_threads - 1).max(max_threads + 4);
    let mut last_gen_thread_change_batch_no = 0;
    let mut last_buf_len = 0;
    let mut is_gen_finished = false;
    // let mut count_gen_spawn = 0;

    loop {
        // triggers a thread sleep if none have finished
        let mut do_sleep = true;
        if num_threads_data_provider_running < max_threads_gen
            && buffer_gen_result.len() < max_buffer_gen
        // && batch_no < data_provider.num_batches() // not required, checked within
        {
            do_sleep = false;
            num_threads_data_provider_running += 1;
            let send_finished_thread_gen = send_finished_thread_data_provider.clone();
            let mut data_provider_batch = data_provider.new_from_data_provider();
            // count_gen_spawn += 1;
            // println!(
            //     "Generator batch {}/{} spawned",
            //     batch_no + 1,
            //     data_provider.num_batches(),
            // );
            // Handle not used as it would create a very large vector to maintain
            thread::spawn(move || {
                let start = Instant::now();
                // let (permutations, _is_last_batch) = data_provider_batch.machine_batch_no(batch_no);
                let data = data_provider_batch.batch_no(batch_no);

                // Send finished signal to allow decider to run.
                let result = Box::new(ThreadResultDataProvider {
                    batch_no,
                    machines: data.machines,
                    pre_decider_count: data.pre_decider_count,
                    duration: start.elapsed(),
                });
                // TODO sending result doubles the time as the data is copied within memory, or not? Should be moved.
                // Single thread is twice as fast, maybe cache issue
                // use Arc?
                // https://doc.rust-lang.org/rust-by-example/std/arc.html
                send_finished_thread_gen.send(result).unwrap();
            });
            batch_no += 1;
            if batch_no == data_provider.num_batches() {
                // turn off data_provider threads, they are not needed any more
                max_threads_gen = 0;
                is_gen_finished = true;
            }

            // Spawn parallel data_providers for all CPUs to build a buffer
            // if num_threads_data_provider_running < threads_data_provider
            //     && num_threads_decider_running + num_threads_data_provider_running < max_threads
            // {
            //     continue;
            // }
        }

        // print running threads information
        // #[cfg(all(debug_assertions, feature = "debug"))]
        // println!(
        //     "Threads {} / {max_threads} ({num_threads_data_provider_running}, {num_threads_decider_running}) - batch {batch_no}/{}, buffer {}",
        //     num_threads_data_provider_running + num_threads_decider_running,
        //     data_provider.num_batches(),
        //     buffer_gen_result.len(),
        // );

        // adjust threads between data_provider and decider to optimize usage, but not too often
        if batch_no - last_gen_thread_change_batch_no > max_threads_gen
            || (max_threads_gen == 0 && buffer_gen_result.len() < max_threads)
        {
            if buffer_gen_result.len() >= max_buffer_gen // buffer used up
                && buffer_gen_result.len() >= last_buf_len // changed buf len
                // keep one thread
                && max_threads_gen > 0
            {
                // data_provider is too fast, decider cannot keep up
                max_threads_gen -= 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads reduced to   {max_threads_gen} (batch no. {batch_no}), buffer gen {}", buffer_gen_result.len());
            } else if buffer_gen_result.len() < max_threads // low bound is buffer for all threads so decider always finds a buffered gen
                && buffer_gen_result.len() <= last_buf_len
                // keep one for decider
                && max_threads_gen < max_threads - 1
                // set to 0 when all batches have been generated
                && !is_gen_finished
            {
                max_threads_gen += 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads increased to {max_threads_gen} (batch no. {batch_no}), buffer gen {}", buffer_gen_result.len());
            }
        }

        // Wait until a permutation pack is available, then run decider. This also frees one CPU.
        if num_threads_data_provider_running > 0 {
            // if buffer_gen_result.is_empty() {
            //     // no data to work for decider, must wait
            //     let thread_result_gen = receive_finished_thread_data_provider.recv().unwrap();
            //     duration_data_provider += thread_result_gen.duration;
            //     buffer_gen_result.push(*thread_result_gen);
            //     num_threads_data_provider_running -= 1;
            //     has_finished = true;
            // } else {
            // collect all finished permutation batches
            while let Ok(thread_result_gen) = receive_finished_thread_data_provider.try_recv() {
                duration_data_provider += thread_result_gen.duration;
                buffer_gen_result.push(*thread_result_gen);
                num_threads_data_provider_running -= 1;
                do_sleep = false;
            }
            // }
        }

        // Check if new decider thread can be started
        // check available threads, keep one open for next data_provider
        if !buffer_gen_result.is_empty()
            && max_threads - max_threads_gen.max(num_threads_data_provider_running)
                > num_threads_decider_running
        {
            // Thread is available, start decider
            do_sleep = false;
            num_threads_decider_running += 1;
            let send_finished_thread_dec = send_finished_thread_decider.clone();
            // move result out of vector to move into thread
            let gen_result = buffer_gen_result.remove(0);
            let batch_info = result.batch_info();
            let requires_pre_decider_run = data_provider.requires_pre_decider_check();
            let decider = decider.new_decider();
            // println!(
            //     "Decider batch {}/{} spawned, max steps; {}",
            //     gen_result.batch_no + 1,
            //     data_provider.num_batches(),
            //     batch_info.steps_max,
            // );
            thread::spawn(move || {
                let start = Instant::now();
                let mut dr = if requires_pre_decider_run != PreDeciderRun::DoNotRun {
                    decider_batch_run_with_pre_deciders(decider, &gen_result.machines, &batch_info)
                } else {
                    decider_batch_run_without_pre_deciders(
                        decider,
                        &gen_result.machines,
                        &batch_info,
                    )
                };
                if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                    dr.add_pre_decider_count(pre_decider_count);
                    dr.add_total(pre_decider_count.num_total());
                }
                let decider_result = ThreadResultDecider {
                    batch_no: gen_result.batch_no,
                    result: dr,
                    duration: start.elapsed(),
                };
                // println!(
                //     "Decider batch {}/{} finished send",
                //     decider_result.batch_no, num_batches
                // );
                send_finished_thread_dec.send(decider_result).unwrap();
            });
        }

        // Check if deciders have finished
        while let Ok(thread_result_dec) = receive_finished_thread_decider.try_recv() {
            result.add_result(&thread_result_dec.result);
            duration_decider += thread_result_dec.duration;
            num_threads_decider_running -= 1;
            // println!(
            //     "Decider batch {}/{} finished",
            //     thread_result_dec.batch_no + 1,
            //     data_provider.num_batches()
            // );

            // Output info on progress
            if let Some(reporter) = reporter.as_mut() {
                if reporter.is_due_progress() {
                    let s = reporter.report(
                        result.num_checked_total(),
                        data_provider.num_machines_total(),
                        &result,
                    );
                    println!("{s}");
                }
            }
        }

        // check if finished all batches
        if num_threads_data_provider_running + num_threads_decider_running == 0
            && buffer_gen_result.is_empty()
        {
            if batch_no < data_provider.num_batches() {
                panic!(
                    "All empty! Threads max gen {max_threads_gen}, batch {batch_no}/{}, buffer {}",
                    data_provider.num_batches(),
                    buffer_gen_result.len(),
                );
            }
            break;
        }

        if do_sleep {
            // print!("w ");
            thread::sleep(Duration::from_micros(100));
        }
    }
    result.duration = DurationDataProvider {
        duration_data_provider,
        duration_decider,
        duration_total: start.elapsed(),
    };
    result.set_name(format!("BB{}: ", n_states) + decider.name_deprecated() + " threaded");

    //     println!("\n{}", result);
    //     if let Some(m) = result.machine_max_steps() {
    //         println!("Most Steps:\n{}", m);
    //     }
    //
    //     println!(
    //         "\nTotal time used for parallel run with {} machines: data_provider {:?}, decider {:?}, total time {:?}",
    //         result.num_checked, duration_generate, duration_run_batches, duration_total
    //     );

    result
}

pub fn decider_batch_run_with_pre_deciders(
    mut decider: impl DeciderDeprecated,
    machines: &[Machine],
    batch_info: &ResultBatchInfo,
) -> DeciderResultStats {
    if machines.is_empty() {
        return DeciderResultStats::new_deprecated(batch_info.n_states, 0);
    }
    let mut result = DeciderResultStats::new_batch_deprecated(batch_info);
    for machine in machines.iter() {
        let mut status = crate::pre_decider::run_pre_decider_simple(machine.transition_table());
        if status == MachineStatus::NoDecision {
            status = decider.decide_machine_deprecated(machine);
        }

        // if machine.id() == 331136 {
        //     println!("{machine}");
        //     println!("{status}");
        // }
        result.add(machine, &status);
    }
    result.add_total(machines.len() as u64);

    result
}

pub fn decider_batch_run_without_pre_deciders(
    mut decider: impl DeciderDeprecated,
    machines: &[Machine],
    batch_info: &ResultBatchInfo,
) -> DeciderResultStats {
    if machines.is_empty() {
        return DeciderResultStats::new_deprecated(batch_info.n_states, 0);
    }
    let mut result = DeciderResultStats::new_batch_deprecated(batch_info);
    for machine in machines.iter() {
        let status = decider.decide_machine_deprecated(machine);
        result.add(machine, &status);
    }
    result.add_total(machines.len() as u64);

    result
}
