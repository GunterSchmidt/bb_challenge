//! This crates holds functions to control the decider runs.
//! Mostly relevant are the functions [run_decider_gen] and [run_decider_chain_gen] to execute the
//! generator with the different deciders and to run over the bb_challenge file [run_deciders_bb_challenge_file].
//!

use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::CoreUsage,
    data_provider::{
        // bb_file_reader::BBFileDataProviderBuilder,
        enumerator_binary::{EnumeratorBinary, EnumeratorType},
        DataProvider,
        DataProviderThreaded,
    },
    decider::{
        decider_result::{BatchData, DeciderResultStats, DurationDataProvider, EndReason},
        pre_decider::PreDeciderRun,
        DeciderConfig, ThreadResultDataProvider, ThreadResultDecider,
    },
    reporter::Reporter,
    utils::num_cpus_percentage,
};

/// General function to call a single decider. \
/// See [crate::config::Config] for configuration details. \
/// See [DeciderConfig] on how to add a function to work with the results (e.g. write to file).
/// # Returns
/// Result stats [DeciderResultStats]. \
/// See [crate::config::Config] limit_machines_undecided if some undecided machines should be returned in full.
/// # Example
/// ```
/// use bb_challenge::CoreUsage;
/// use bb_challenge::config::Config;
/// use bb_challenge::decider::DeciderStandard;
/// use bb_challenge::generator::GeneratorStandard;
/// let config_cycler = Config::builder(4)
/// // Set limit to 0 or 10_000_000_000 to test all machines of BB4 (6,975,757,441 machines total).
/// // On a fast machine run for all machines will take less than 2 seconds (release mode).
/// // Luckily the longest machine is in the first 250_000_000 generated entries.
///   .machine_limit(250_000_000)
///   .step_limit_cycler(150)        
///   .build();
/// let mut dc_cycler = DeciderStandard::Cycler.decider_config(&config_cycler);
/// let result = bb_challenge::decider_engine::run_decider(
///     dc_cycler,
///     CoreUsage::MultiCore,
///     GeneratorStandard::GeneratorReduced,
/// );
/// println!("{}", result.to_string_with_duration());
/// assert_eq!(107, result.machine_max_steps().unwrap().steps());
/// ```
pub fn run_decider_gen(
    decider_config: DeciderConfig,
    generator_std: EnumeratorType,
    multi_core: CoreUsage,
) -> DeciderResultStats {
    run_decider_chain_gen(&[decider_config], generator_std, multi_core)
}

/// General function to call a decider chain.
pub fn run_decider_chain_gen(
    decider_config: &[DeciderConfig],
    generator_std: EnumeratorType,
    multi_core: CoreUsage,
) -> DeciderResultStats {
    let first_config = decider_config.first().expect("No decider given").config();
    let generator = EnumeratorBinary::new(generator_std, first_config);
    // let generator = GeneratorReducedForward::new(first_config);
    match multi_core {
        CoreUsage::SingleCore => {
            batch_run_decider_chain_data_provider_single_thread(decider_config, generator)
        }
        CoreUsage::SingleCoreEnumeratorMultiCoreDecider => {
            batch_run_decider_chain_threaded_data_provider_single_thread(decider_config, generator)
        }
        CoreUsage::MultiCore => {
            batch_run_decider_chain_threaded_data_provider_multi_thread(decider_config, generator)
        } // _ => panic!("use 0: single, 1: multi with single generator, 2: multi"),
    }
}

/// General function to call a decider chain.
pub fn run_decider_chain_data_provider_single(
    decider_config: &[DeciderConfig],
    data_provider: impl DataProvider,
    multi_core: CoreUsage,
) -> DeciderResultStats {
    match multi_core {
        CoreUsage::SingleCore => {
            batch_run_decider_chain_data_provider_single_thread(decider_config, data_provider)
        }
        CoreUsage::SingleCoreEnumeratorMultiCoreDecider => {
            batch_run_decider_chain_threaded_data_provider_single_thread(
                decider_config,
                data_provider,
            )
        }
        CoreUsage::MultiCore => {
            panic!("MultiCore requires trait DataProviderThreaded and can't be used here.")
        }
    }
}

/// Runs the deciders (using the thread called from). \
/// This is build as an internal function but can be used if own data provider handling is used.
/// Return DeciderResultStats with an EndReason which needs to be evaluated.
pub fn decide_batch_chain(
    batch_data: BatchData,
    // data: DataProviderResult,
    // num_batches: usize,
    // run_predecider: PreDeciderRun,
    decider_configs: &[DeciderConfig],
) -> DeciderResultStats {
    let start_decider = Instant::now();
    // interestingly this is required
    let mut batch_data = batch_data;
    let first_decider = decider_configs.first().expect("No decider!");
    let mut result_batch = DeciderResultStats::new_init_steps_max(
        first_decider.config(),
        batch_data.result_decided.steps_max(),
    );
    for dc in decider_configs.iter().skip(1) {
        result_batch.enhance_machines_un_decided(dc.config());
    }
    // run first decider which includes pre-decider elimination
    // let mut undecided_available = true;
    let mut stop_run = false;

    match first_decider.f_decider()(&mut batch_data) {
        Ok(_) => {
            // Call user analyzer/worker so result can be dealt with individually (e.g. save), also in case of error.
            if let Some(fnr) = first_decider.fo_result_worker() {
                if let Err(e) = fnr(&mut batch_data) {
                    result_batch.end_reason = e;
                    stop_run = true;
                    // eprintln!("{}", e);
                }
            }
            if !result_batch.add_result(&batch_data.result_decided) {
                stop_run = true
            }

            let mut m_undecided;
            let batch_no = batch_data.batch_no;
            let num_batches = batch_data.num_batches;
            // run other deciders
            for d in decider_configs.iter().skip(1) {
                if !stop_run && !batch_data.machines_undecided.machines.is_empty() {
                    m_undecided = batch_data.machines_undecided.machines;
                    // borrow checker requires new object instead of just updating ref to machines
                    batch_data = BatchData {
                        machines: &m_undecided,
                        // TODO id required?
                        ids: &None,
                        result_decided: DeciderResultStats::new_init_steps_max(
                            first_decider.config(),
                            result_batch.steps_max(),
                        ),
                        machines_decided: Default::default(),
                        machines_undecided: Default::default(),
                        batch_no,
                        num_batches,
                        decider_id: d.decider_id(),
                        config: d.config(),
                        run_predecider: PreDeciderRun::DoNotRun,
                    };

                    match d.f_decider()(&mut batch_data) {
                        Ok(()) => {
                            batch_data.result_decided.clear_total();
                            // call user analyzer/worker so result can be dealt with individually (e.g. save)
                            if d.fo_result_worker().is_some() {
                                if let Err(e) = d.fo_result_worker().unwrap()(&mut batch_data) {
                                    result_batch.end_reason = e;
                                    stop_run = true;
                                    // eprintln!("{}", e);
                                }
                            }
                            result_batch.add_result(&batch_data.result_decided);
                        }
                        Err(e) => {
                            result_batch.end_reason = e;
                            stop_run = true;
                        }
                    }
                }
            }

            // add remaining undecided to final result
            for (i, m) in batch_data.machines_undecided.machines.iter().enumerate() {
                if !result_batch.add(m, &batch_data.machines_undecided.states[i]) {
                    // println!("result decided/undecided full");
                    break;
                }
            }
        }
        Err(e) => result_batch.end_reason = e,
    }

    result_batch.duration = DurationDataProvider {
        duration_decider: start_decider.elapsed(),
        ..Default::default()
    };

    result_batch
}

/// Runs the data provider and the deciders both on the main thread
/// using the standard reporter.
pub fn batch_run_decider_chain_data_provider_single_thread(
    decider_configs: &[DeciderConfig],
    data_provider: impl DataProvider,
) -> DeciderResultStats {
    let total = data_provider.num_machines_to_process();
    batch_run_decider_chain_data_provider_single_thread_reporting(
        decider_configs,
        data_provider,
        Some(Reporter::new_default(total)),
    )
}

/// Runs the data provider and the deciders both on the main thread
/// using a custom reporter (or None to omit reporting).
// TODO check end_result like in multi
pub fn batch_run_decider_chain_data_provider_single_thread_reporting(
    decider_configs: &[DeciderConfig],
    mut data_provider: impl DataProvider,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats {
    let first_config = decider_configs.first().expect("No decider given").config();

    let start = Instant::now();
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    // data_provider.set_batch_size_for_num_threads(1);
    let mut result_main = DeciderResultStats::new(first_config);
    for dc in decider_configs.iter() {
        result_main.enhance_machines_un_decided(dc.config());
    }
    loop {
        // generate or get one batch of machines
        let start_gen = Instant::now();
        let r = data_provider.machine_batch_next();
        match r {
            Ok(data) => {
                if let Some(pre) = data.pre_decider_count {
                    result_main.add_pre_decider_count(&pre);
                    result_main.add_total(pre.num_total());
                }
                duration_data_provider += start_gen.elapsed();
                if !data.machines.is_empty() {
                    // TODO check on end_reason
                    // end_reason is checked later to decide machines before end

                    // run deciders
                    let start_decider = Instant::now();
                    // run first decider which includes pre-decider elimination
                    // let mut undecided_available = true;
                    // let mut stop_run = false;
                    // let first_decider_config = decider_config.first().expect("No decider!");
                    let batch_data = BatchData {
                        machines: &data.machines,
                        // TODO id required?
                        ids: &None,
                        result_decided: DeciderResultStats::new_init_steps_max(
                            first_config,
                            result_main.steps_max(),
                        ),
                        machines_decided: Default::default(),
                        machines_undecided: Default::default(),
                        batch_no: data.batch_no,
                        num_batches: data_provider.num_batches(),
                        decider_id: decider_configs[0].decider_id(),
                        config: first_config,
                        run_predecider: data_provider.requires_pre_decider_check(),
                    };
                    let dc_result = decide_batch_chain(batch_data, decider_configs);
                    result_main.add_result(&dc_result);
                    duration_decider += start_decider.elapsed();
                    match dc_result.end_reason {
                        EndReason::AllMachinesChecked => todo!(),
                        EndReason::Error(_, _) => todo!(),
                        EndReason::IsLastBatch => todo!(),
                        EndReason::MachineLimitReached(_) => todo!(),
                        EndReason::NoBatchData => todo!(),
                        EndReason::NoMoreData => todo!(),
                        EndReason::RecordLimitDecidedReached(_) => break,
                        EndReason::RecordLimitUndecidedReached(_) => break,
                        EndReason::StopRequested(_, _) => break,
                        EndReason::None => {}
                    };
                    // let undecided_available = result.add_result(&br.result_decided);

                    // end if undecided limit has been reached
                    // if stop_run || !undecided_available {
                    //     break;
                    // }
                }
                match data.end_reason {
                    EndReason::AllMachinesChecked => todo!(),
                    EndReason::Error(_, _) => todo!(),
                    EndReason::IsLastBatch => {
                        result_main.end_reason = EndReason::AllMachinesChecked;
                        break;
                    }
                    EndReason::MachineLimitReached(_) => todo!(),
                    EndReason::NoBatchData => todo!(),
                    EndReason::NoMoreData => {
                        result_main.end_reason = data.end_reason;
                        break;
                    }
                    EndReason::RecordLimitDecidedReached(_) => todo!(),
                    EndReason::RecordLimitUndecidedReached(_) => todo!(),
                    EndReason::StopRequested(_, _) => todo!(),
                    EndReason::None => {}
                }

                // Output info on progress
                if let Some(reporter) = reporter.as_mut() {
                    if reporter.is_due_progress() {
                        let s =
                            reporter.report_stats(result_main.num_processed_total(), &result_main);
                        println!("{s}");
                    }
                }
            }
            Err(_) => todo!(),
        }
    }
    result_main.duration = DurationDataProvider {
        duration_data_provider,
        duration_decider,
        duration_total: start.elapsed(),
    };

    // Add the name at the end or it will result in a little performance loss. Reason unknown.
    // TODO name
    result_main.set_name(format!(
        "BB{}: '{}'",
        first_config.n_states(),
        "decider.name()"
    ));

    result_main
}

/// Runs the data provider and the deciders in separate threads (deciders can have multiple threads)
/// using the standard reporter.
pub fn batch_run_decider_chain_threaded_data_provider_single_thread(
    decider_configs: &[DeciderConfig],
    data_provider: impl DataProvider,
) -> DeciderResultStats {
    let total = data_provider.num_machines_to_process();
    batch_run_decider_chain_threaded_data_provider_single_thread_reporting(
        decider_configs,
        data_provider,
        Some(Reporter::new_default(total)),
    )
}

/// Runs the data provider and the deciders in separate threads (deciders can have multiple threads)
/// using a custom reporter (or None to omit reporting).
// TODO check end_result like in multi
pub fn batch_run_decider_chain_threaded_data_provider_single_thread_reporting(
    decider_configs: &[DeciderConfig],
    mut data_provider: impl DataProvider,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats {
    let start = Instant::now();
    let first_config = decider_configs
        .first()
        .expect("No decider given")
        .config_clone();
    let max_threads = num_cpus_percentage(first_config.cpu_utilization_percent());
    // if single thread run single
    if max_threads == 1 {
        return batch_run_decider_chain_data_provider_single_thread_reporting(
            decider_configs,
            data_provider,
            reporter,
        );
    }
    let mut result_main = DeciderResultStats::new(*first_config);
    for dc in decider_configs.iter() {
        result_main.enhance_machines_un_decided(dc.config());
    }
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();

    // Make a Thread Scope so that references can be accessed
    thread::scope(|s| {
        // TODO some fine tuning. Now the decider uses all threads, which leads to more load than CPUs are available.
        // If we leave one open, then CPU is not used in case of quick data provider.
        let max_threads_decider = max_threads;

        // let mut max_threads_gen = (max_threads / 2 + 1).max(1);
        let (send_finished_thread_decider, receive_finished_thread_decider) =
            std::sync::mpsc::channel::<ThreadResultDecider>();
        let mut num_threads_decider_running = 0;
        let mut buffer_gen_result = Vec::new();
        let max_buffer_gen = (max_threads - 1).max(max_threads + 4);
        let mut is_gen_finished = false;

        // loop over all batch packages
        loop {
            // triggers a thread sleep if none have finished
            let mut do_sleep = true;
            if !is_gen_finished && buffer_gen_result.len() < max_buffer_gen {
                do_sleep = false;
                let start = Instant::now();
                let r = data_provider.machine_batch_next();
                match r {
                    Ok(batch) => {
                        // TODO handle other end reasons
                        match batch.end_reason {
                            EndReason::Error(_, _) => todo!(),
                            EndReason::IsLastBatch => is_gen_finished = true,
                            EndReason::MachineLimitReached(_) => todo!(),
                            EndReason::NoBatchData => todo!(),
                            EndReason::NoMoreData => todo!(),
                            EndReason::None => {}
                            EndReason::AllMachinesChecked => todo!(),
                            EndReason::StopRequested(_, _) => todo!(),
                            EndReason::RecordLimitDecidedReached(_) => todo!(),
                            EndReason::RecordLimitUndecidedReached(_) => todo!(),
                        }
                        // println!(
                        //     "Generator batch {}/{} created",
                        //     batch.batch_no + 1,
                        //     data_provider.num_batches(),
                        // );
                        buffer_gen_result.push(batch);
                    }
                    Err(_) => todo!(),
                }

                duration_data_provider += start.elapsed();
            }

            // print running threads information
            // #[cfg(all(debug_assertions, feature = "debug"))]
            // println!(
            //     "Threads {} / {max_threads} ({num_threads_data_provider_running}, {num_threads_decider_running}) - batch {batch_no}/{}, buffer {}",
            //     num_threads_data_provider_running + num_threads_decider_running,
            //     data_provider.num_batches(),
            //     buffer_gen_result.len(),
            // );

            // Check if new decider thread can be started
            // check available threads, keep one open for next data_provider
            if !buffer_gen_result.is_empty() && max_threads_decider > num_threads_decider_running {
                // Thread is available, start decider
                do_sleep = false;
                num_threads_decider_running += 1;
                let send_finished_thread_dec = send_finished_thread_decider.clone();
                // move result out of vector to move into thread
                let gen_result = buffer_gen_result.remove(0);
                if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                    result_main.add_pre_decider_count(pre_decider_count);
                    result_main.add_total(pre_decider_count.num_total());
                }

                let run_predecider = data_provider.requires_pre_decider_check();
                let num_batches = data_provider.num_batches();
                let result_decided =
                    DeciderResultStats::new_init_steps_max(*first_config, result_main.steps_max());
                let config = *first_config;
                // Output thread information
                // println!(
                //     "Decider batch {}/{} spawned, max steps; {}",
                //     gen_result.batch_no + 1,
                //     data_provider.num_batches(),
                //     result_main.steps_max(),
                // );
                s.spawn(move || {
                    let start = Instant::now();
                    // gen_result is moved and not used further
                    // let machines = gen_result.machines;
                    // create batch data for first decider
                    let batch_data = BatchData {
                        machines: &gen_result.machines,
                        ids: &gen_result.ids,
                        result_decided,
                        machines_decided: Default::default(),
                        machines_undecided: Default::default(),
                        batch_no: gen_result.batch_no,
                        num_batches,
                        decider_id: decider_configs[0].decider_id(),
                        config: &config,
                        run_predecider,
                    };
                    let dr = decide_batch_chain(batch_data, decider_configs);
                    let decider_result = ThreadResultDecider {
                        batch_no: gen_result.batch_no,
                        result: dr,
                        duration: start.elapsed(),
                    };
                    // println!(
                    //     "Decider batch {}/{} finished send",
                    //     decider_result.batch_no, num_batches
                    // );
                    // unwrap error can occur if stop is requested while other threads are still running
                    send_finished_thread_dec
                        .send(decider_result)
                        .unwrap_or_default();
                });
            }

            // Check if deciders have finished
            while let Ok(thread_result_dec) = receive_finished_thread_decider.try_recv() {
                result_main.add_result(&thread_result_dec.result);
                duration_decider += thread_result_dec.duration;
                num_threads_decider_running -= 1;
                // println!(
                //     "Decider batch {}/{} finished",
                //     thread_result_dec.batch_no + 1,
                //     data_provider.num_batches()
                // );
            }

            // Output info on progress
            if let Some(reporter) = reporter.as_mut() {
                if reporter.is_due_progress() {
                    let s = reporter.report_stats(result_main.num_processed_total(), &result_main);
                    println!("{s}");
                }
            }

            // check if finished all batches
            // dbg!(
            //     batch_no,
            //     // data_provider.num_batches(),
            //     // is_gen_finished,
            //     num_threads_decider_running,
            //     buffer_gen_result.len()
            // );
            // println!(
            //     "batch no {batch_no}, threads: {num_threads_decider_running}, buffer: {}",
            //     buffer_gen_result.len()
            // );
            if is_gen_finished && num_threads_decider_running == 0 && buffer_gen_result.is_empty() {
                // TODO check is_gen_finished
                // if batch_no + 1 < data_provider.num_batches() {
                //     panic!(
                //         "All empty! Batch {batch_no}/{}, buffer {}",
                //         data_provider.num_batches(),
                //         buffer_gen_result.len(),
                //     );
                // }
                break;
            }
            match result_main.end_reason {
                EndReason::AllMachinesChecked => todo!(),
                EndReason::Error(_, _) => todo!(),
                EndReason::IsLastBatch => todo!(),
                EndReason::MachineLimitReached(_) => todo!(),
                EndReason::NoMoreData => todo!(),
                EndReason::StopRequested(_, _) => break,
                EndReason::RecordLimitDecidedReached(_) => break,
                EndReason::RecordLimitUndecidedReached(_) => break,
                EndReason::NoBatchData => todo!(),
                EndReason::None => {}
            }

            if do_sleep {
                // print!("w ");
                thread::sleep(Duration::from_micros(100));
            }
        }
    });
    result_main.duration = DurationDataProvider {
        duration_data_provider,
        duration_decider,
        duration_total: start.elapsed(),
    };

    for d in decider_configs {
        result_main.add_name(&format!(
            "BB{} threaded: {}",
            first_config.n_states(),
            d.decider_id().name
        ));
    }

    result_main
}

/// Runs the data provider and the deciders in separate threads (both can have multiple threads)
/// using the standard reporter.
pub fn batch_run_decider_chain_threaded_data_provider_multi_thread(
    decider_configs: &[DeciderConfig],
    data_provider: impl DataProviderThreaded + std::marker::Send,
) -> DeciderResultStats {
    let total = data_provider.num_machines_to_process();
    batch_run_decider_chain_threaded_data_provider_multi_thread_reporting(
        decider_configs,
        data_provider,
        Some(Reporter::new_default(total)),
    )
}

/// Runs the data provider and the decider in separate threads (both can have multiple threads)
/// using a custom reporter (or None to omit reporting).
/// My tests showed that larger batch sizes (e.g. 50 million) are faster. On my machine with Hyper-Threading
/// a CPU percentage of 80% is almost as fast as 100% on Linux. On windows better results were achieved with 120%.
/// Batch size and CPU percentage need to be tested to find the fastest combination.
// TODO Write test function to find best combination.
// How it works:
// First only the data provider (generator) runs and collects the machines for the deciders into
// ThreadResultDataProvider. Then the first decider takes this data and evaluates them, sending the
// result into ThreadResultDecider. This will be collected in the main thread to get the final result.
// A buffer of data provider results is created so the decider threads always has batch data. When the buffer
// gets too large, the number of threads for the data provider are reduced, so that more deciders can
// work in parallel and vice versa.
// Contains a lot of code to optimize thread usage.
// TODO thread recycling.
pub fn batch_run_decider_chain_threaded_data_provider_multi_thread_reporting(
    decider_configs: &[DeciderConfig],
    data_provider: impl DataProviderThreaded + std::marker::Send,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats {
    let start = Instant::now();
    let first_config = decider_configs
        .first()
        .expect("No decider given")
        .config_clone();
    let max_threads = num_cpus_percentage(first_config.cpu_utilization_percent());
    // if single thread run single
    if max_threads == 1 {
        return batch_run_decider_chain_data_provider_single_thread_reporting(
            decider_configs,
            data_provider,
            reporter,
        );
    }

    let mut result_main = DeciderResultStats::new(*first_config);
    for dc in decider_configs.iter().skip(1) {
        result_main.enhance_machines_un_decided(dc.config());
    }
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();

    // Make a Thread Scope so that references can be accessed
    thread::scope(|s| {
        let mut max_threads_gen = (max_threads / 2 + 1).max(1);
        let mut batch_no = 0;
        let (send_finished_thread_data_provider, receive_finished_thread_data_provider) =
            std::sync::mpsc::channel();
        let (send_finished_thread_decider, receive_finished_thread_decider) =
            std::sync::mpsc::channel::<ThreadResultDecider>();
        let mut num_threads_data_provider_running = 0;
        let mut num_threads_decider_running = 0;
        let mut buffer_gen_result: Vec<ThreadResultDataProvider> = Vec::new();
        let max_buffer_gen = (max_threads - 1).max(max_threads + 4);
        let mut last_gen_thread_change_batch_no = 0;
        let mut last_buf_len = 0;
        let mut is_gen_finished = false;

        // loop over all batch packages
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
                // let r = data_provider.machine_batch_next();
                // match r {
                //     Ok(_) => todo!(),
                //     Err(_) => todo!(),
                // }
                let mut data_provider_batch = data_provider.new_from_data_provider();
                // count_gen_spawn += 1;
                // println!(
                //     "Generator batch {}/{} spawned",
                //     batch_no + 1,
                //     data_provider.num_batches(),
                // );
                // Handle not used as it would create a very large vector to maintain
                s.spawn(move || {
                    let start = Instant::now();
                    let data = data_provider_batch.batch_no(batch_no);
                    // println!("batch_no: {batch_no} = {}", data.machines.len());

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
                    // unwrap can fail when stop is requested
                    send_finished_thread_gen.send(result).unwrap_or_default();
                });
                batch_no += 1;
                if batch_no == data_provider.num_batches() {
                    // turn off data_provider threads, they are not needed any more
                    max_threads_gen = 0;
                    is_gen_finished = true;
                }
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
                // collect all finished permutation batches
                while let Ok(thread_result_gen) = receive_finished_thread_data_provider.try_recv() {
                    duration_data_provider += thread_result_gen.duration;
                    buffer_gen_result.push(*thread_result_gen);
                    num_threads_data_provider_running -= 1;
                    do_sleep = false;
                }
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
                if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                    result_main.add_pre_decider_count(pre_decider_count);
                    result_main.add_total(pre_decider_count.num_total());
                }
                let config = *first_config;
                let run_predecider = data_provider.requires_pre_decider_check();
                let num_batches = data_provider.num_batches();
                let result_decided =
                    DeciderResultStats::new_init_steps_max(*first_config, result_main.steps_max());
                // Output thread information
                // println!(
                //     "Decider batch {}/{} spawned, max steps; {}",
                //     gen_result.batch_no + 1,
                //     data_provider.num_batches(),
                //     result_main.steps_max(),
                // );
                s.spawn(move || {
                    let start = Instant::now();
                    // gen_result is moved and not used further
                    // let machines = gen_result.machines;
                    // create batch data for first decider
                    let batch_data = BatchData {
                        machines: &gen_result.machines,
                        // TODO id required?
                        ids: &None,
                        result_decided,
                        machines_decided: Default::default(),
                        machines_undecided: Default::default(),
                        batch_no: gen_result.batch_no,
                        num_batches,
                        decider_id: decider_configs[0].decider_id(),
                        config: &config,
                        run_predecider,
                    };
                    // println!(
                    //     "Decider batch {}/{} send b {}",
                    //     batch_data.batch_no + 1,
                    //     num_batches,
                    //     batch_data.machines.len(),
                    // );
                    let dr = decide_batch_chain(batch_data, decider_configs);
                    let decider_result = ThreadResultDecider {
                        batch_no: gen_result.batch_no,
                        result: dr,
                        duration: start.elapsed(),
                    };
                    // unwrap error can occur if stop is requested while other threads are still running
                    send_finished_thread_dec
                        .send(decider_result)
                        .unwrap_or_default();
                });
            }

            // Check if deciders have finished
            while let Ok(thread_result_dec) = receive_finished_thread_decider.try_recv() {
                // println!(
                //     "Decider batch {}/{} finished, r {}, size {}",
                //     thread_result_dec.batch_no + 1,
                //     data_provider.num_batches(),
                //     result_main.num_processed_total(),
                //     thread_result_dec.result.num_processed_total()
                // );
                result_main.add_result(&thread_result_dec.result);
                duration_decider += thread_result_dec.duration;
                num_threads_decider_running -= 1;

                // Output info on progress
                if let Some(reporter) = reporter.as_mut() {
                    if reporter.is_due_progress() {
                        let s =
                            reporter.report_stats(result_main.num_processed_total(), &result_main);
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
                result_main.end_reason = EndReason::AllMachinesChecked;
                break;
            }
            match result_main.end_reason {
                EndReason::AllMachinesChecked => todo!(),
                EndReason::Error(_, _) => todo!(),
                EndReason::IsLastBatch => todo!(),
                EndReason::MachineLimitReached(_) => todo!(),
                EndReason::NoMoreData => todo!(),
                EndReason::StopRequested(_, _) => break,
                EndReason::RecordLimitDecidedReached(_) => break,
                EndReason::RecordLimitUndecidedReached(_) => break,
                EndReason::NoBatchData => todo!(),
                EndReason::None => {}
            }

            if do_sleep {
                // print!("w ");
                thread::sleep(Duration::from_micros(100));
            }
        }
    });
    result_main.duration = DurationDataProvider {
        duration_data_provider,
        duration_decider,
        duration_total: start.elapsed(),
    };
    result_main.set_name(format!(
        "BB{}: '{}' threaded",
        first_config.n_states(),
        "decider.name()"
    ));

    result_main
}
