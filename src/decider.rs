use std::{
    fmt::Display,
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::Config,
    data_provider::DataProvider,
    data_provider_threaded::DataProviderThreaded,
    decider_hold_u128_long::DeciderHoldU128Long,
    decider_loop_v4::DeciderLoopV4,
    decider_result::{
        BatchResult, DeciderResultStats, DurationDataProvider, EndReason, MachinesUndecided,
        PreDeciderCount,
    },
    decider_result_worker::ResultWorker,
    machine::Machine,
    pre_decider::{run_pre_decider_simple, run_pre_decider_strict, PreDeciderRun},
    reporter::Reporter,
    status::MachineStatus,
    utils::num_cpus_percentage,
};

pub type ResultDecider = std::result::Result<(), DeciderError>;

/// The deciders need to return Self to be able to make a new decider for each thread.
/// This makes them not object save and thus cannot be passed in a Vec.
///
pub enum DeciderEnum {
    // PreDecider(PreDecider),
    LoopV4(DeciderLoopV4),
    HoldLong(DeciderHoldU128Long),
}

pub trait Decider {
    // TODO remove
    /// Creates a new decider. Allows individual initialization code for each decider and is called for each permutation batch.
    fn new_decider(&self) -> Self;

    /// Returns the result of this decider for one single machine. \
    /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus;

    /// Allows to inefficiently test a single machine.
    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus;

    fn decider_run_batch(
        machines: &[Machine],
        run_predecider: PreDeciderRun,
        config: &Config,
    ) -> Option<BatchResult>;

    /// Returns the name of this decider
    fn name(&self) -> &str;
}

// pub struct DeciderDummy;
//
// impl Decider for DeciderDummy {
//     fn new_decider(&self) -> Self {
//         DeciderDummy
//     }
//
//     fn decide_machine(&mut self, _machine: &Machine) -> MachineStatus {
//         MachineStatus::NoDecision
//     }
//
//     fn name(&self) -> String {
//         "Dummy".to_string()
//     }
// }

#[inline]
pub fn decider_generic_run_batch(
    mut decider: impl Decider,
    machines: &[Machine],
    run_predecider: PreDeciderRun,
    config: &Config,
) -> Option<BatchResult> {
    if machines.is_empty() {
        return None;
    }
    // let mut machines_undecided: Vec<MachineUndecided> = Vec::with_capacity(machines.len());
    // TODO optimize undecided. Possible collect only ids, if count is same then to_vec for machines.
    let cap = if run_predecider != PreDeciderRun::DoNotRun {
        // loop decider should run first, which eliminates most machines
        machines.len() / 100
    } else {
        machines.len()
    };
    let mut machines_undecided = MachinesUndecided::new(cap);
    let mut result_decided = DeciderResultStats::new(config);
    match run_predecider {
        PreDeciderRun::DoNotRun => {
            for machine in machines.iter() {
                // TODO self_ref
                let status = decider.decide_machine(machine);
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        machines_undecided.machines.push(*machine);
                        machines_undecided.states.push(status);
                    }
                    _ => {
                        result_decided.add(machine, &status);
                    }
                }
            }
        }
        PreDeciderRun::NormalRun => {
            for machine in machines.iter() {
                let mut status = run_pre_decider_simple(machine.transition_table());
                if status == MachineStatus::NoDecision {
                    status = decider.decide_machine(machine);
                }
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        machines_undecided.machines.push(*machine);
                        machines_undecided.states.push(status);
                    }
                    _ => {
                        result_decided.add(machine, &status);
                    }
                }
            }
        }
        PreDeciderRun::RunWithStart0rb1rbOnly => {
            for machine in machines.iter() {
                let mut status = run_pre_decider_strict(machine.transition_table());
                if status == MachineStatus::NoDecision {
                    status = decider.decide_machine(machine);
                }
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        machines_undecided.machines.push(*machine);
                        machines_undecided.states.push(status);
                    }
                    _ => {
                        result_decided.add(machine, &status);
                    }
                }
            }
        }
    }
    result_decided.add_total(machines.len() as u64);

    // TODO batch no
    Some(BatchResult {
        batch_no: 0,
        num_batches: 0,
        result_decided,
        machines_undecided,
        decider_name: decider.name().to_string(),
    })
}

#[derive(Default)]
pub struct ThreadResultDataProvider {
    pub batch_no: usize,
    pub machines: Vec<Machine>,
    pub pre_decider_count: Option<PreDeciderCount>,
    pub duration: Duration,
}

pub struct ThreadResultDecider {
    pub batch_no: usize,
    pub result: DeciderResultStats,
    pub duration: Duration,
}

// pub fn calculate<F>(data_provider: impl DataProvider, config: &Config, f_result_analyzer: &F)
// where
//     F: Fn(&ResultBatch) -> i32,
// {
// }
//
// pub fn calculate_ref(
//     data_provider: impl DataProvider,
//     config: &Config,
//     f_result_analyzer: &dyn Fn(&ResultBatch) -> i32,
// ) {
// }

/// Runs the given decider in a single thread.
// This is just a convenience function to avoid creating a vector.
pub fn run_decider_data_provider_single_thread<F, W>(
    f_decider_run_batch: F,
    data_provider: impl DataProvider,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    run_decider_chain_data_provider_single_thread_reporting(
        &vec![f_decider_run_batch],
        data_provider,
        Some(Reporter::default()),
        config,
        f_result_worker,
    )
}

pub fn run_decider_chain_data_provider_single_thread<F, W>(
    fs_decider_run_batch: &[F],
    data_provider: impl DataProvider,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    run_decider_chain_data_provider_single_thread_reporting(
        fs_decider_run_batch,
        data_provider,
        Some(Reporter::default()),
        config,
        f_result_worker,
    )
}

pub fn run_decider_chain_data_provider_single_thread_reporting<F, W>(
    fs_decider_run_batch: &[F],
    mut data_provider: impl DataProvider,
    mut reporter: Option<Reporter>,
    config: &Config,
    // Option cannot be used as it requires a type annotation for None.
    // Optional function to do something with the result of each batch run.
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>, // + Send + Copy + 'static,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    // TODO check filled
    let start = Instant::now();
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    let n_states = data_provider.n_states();
    data_provider.set_batch_size_for_num_threads(1);
    let mut result = DeciderResultStats::new_deprecated(n_states, 0);
    let requires_pre_decider_check = data_provider.requires_pre_decider_check();
    result.set_record_machines_max_steps(data_provider.config().limit_machines_max_steps());
    result.set_limit_machines_undecided(data_provider.config().limit_machines_undecided());
    let mut batch_no = 0;
    // copy config so init steps can be updated
    // TODO maybe handle init_steps differently?
    let mut config = config.clone();
    loop {
        let start_gen = Instant::now();
        // if batch_no >= 409 {
        //     println!()
        // }
        let data = data_provider.machine_batch_next();
        batch_no += 1;
        // if batch_no % 100 == 0 {
        // println!("Batch no. {batch_no} / {}", data_provider.num_batches());
        // }
        if let Some(pre) = data.pre_decider_count {
            result.add_pre_decider_count(&pre);
            result.add_total(pre.num_total());
        }
        duration_generator += start_gen.elapsed();
        let start_decider = Instant::now();
        // run first decider which includes pre-decider elimination
        let mut undecided_available = true;
        let mut stop_run = false;
        if let Some(br) =
            fs_decider_run_batch[0](&data.machines, requires_pre_decider_check, &config)
        {
            result.add_result(&br.result_decided);
            // call user analyzer/worker so result can be dealt with individually (e.g. save)
            if let Err(e) = f_result_worker(&br, &config) {
                result.end_reason = EndReason::Error(e.to_string());
                stop_run = true;
                // eprintln!("{}", e);
            }
            config.increase_init_step_max(br.result_decided.steps_max());
            let mut m_undecided = br.machines_undecided;

            if !stop_run {
                // run other deciders
                for f in fs_decider_run_batch.iter().skip(1) {
                    if !m_undecided.machines.is_empty() {
                        if let Some(br) = f(&m_undecided.machines, PreDeciderRun::DoNotRun, &config)
                        {
                            result.add_result(&br.result_decided);
                            m_undecided = br.machines_undecided;
                        }
                    }
                }
            }

            // add remaining undecided to final result
            for (i, m) in m_undecided.machines.iter().enumerate() {
                if !result.add(m, &m_undecided.states[i]) {
                    result.end_reason =
                        EndReason::UndecidedLimitReached(result.limit_machines_undecided());
                    undecided_available = false;
                    break;
                }
            }
        }
        // println!(
        //     "batch {batch_no}: total {}, should be {}",
        //     result.num_checked_total(),
        //     batch_no * data_provider.batch_size()
        // );

        // let undecided_available = result.add_result(&br.result_decided);
        duration_decider += start_decider.elapsed();

        // end if undecided limit has been reached
        if stop_run || !undecided_available {
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
    // TODO name
    result.name = format!("BB{}: ", n_states) + "decider.name()";

    result
}

/// Runs the given decider in a single thread.
// This is just a convenience function to avoid creating a vector.
pub fn run_decider_data_provider_threaded<F, G, W>(
    f_decider_run_batch: F,
    data_provider: G,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProviderThreaded + Send + 'static,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    run_decider_chain_data_provider_threaded_reporting(
        &vec![f_decider_run_batch],
        data_provider,
        Some(Reporter::default()),
        config,
        f_result_worker,
    )
}

/// Runs the check in separate threads using the standard reporter.
/// The generation of the permutations is not threaded.
pub fn run_decider_chain_data_provider_threaded<F, G, W>(
    fs_decider_run_batch: &[F],
    data_provider: G,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProviderThreaded + Send + 'static,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    run_decider_chain_data_provider_threaded_reporting(
        fs_decider_run_batch,
        data_provider,
        Some(Reporter::default()),
        config,
        f_result_worker,
    )
}

/// Runs the data provider and the decider in separate threads (both can have multiple threads)
/// using a custom reporter (or None to omit reporting).
/// My tests showed that larger batch sizes (e.g. 50 million) are faster. On my machine with Hyper-Threading
/// a CPU percentage of 80% is almost as fast as 100% on Linux. On windows better results were achieved with 120%.
/// Batch size and CPU percentage need to be tested to find the fastest combination.
// TODO Write test function to find best combination.
// First only the data provider (generator) runs and collects the machines for the deciders in
// ThreadResultDataProvider. Then the first decider takes this data and evaluates them, sending the
// result in ThreadResultDecider. This will be collected in the main thread to get the final result.
// A buffer of data provider results is created so the decider always has batch data. When the buffer
// gets too large, the number of threads for the data provider are reduced, so that more deciders can
// work in parallel and vice versa.
// Contains a lot of code to optimize thread usage.
// TODO thread recycling.
pub fn run_decider_chain_data_provider_threaded_reporting<F, G, W>(
    fs_decider_run_batch: &[F],
    data_provider: G,
    mut reporter: Option<Reporter>,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProviderThreaded + Send + 'static,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let max_threads = num_cpus_percentage(data_provider.config().cpu_utilization_percent());
    // if single thread run single
    if max_threads == 1 {
        return run_decider_chain_data_provider_single_thread_reporting(
            fs_decider_run_batch,
            data_provider,
            reporter,
            config,
            f_result_worker,
        );
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
    let mut config = config.clone();

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
                let data = data_provider_batch.machine_batch_no(batch_no);

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
            if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                result.add_pre_decider_count(pre_decider_count);
                result.add_total(pre_decider_count.num_total());
            }
            // let batch_info = result.batch_info();
            config.increase_init_step_max(result.steps_max());
            config.set_limit_machines_undecided(result.num_undecided_free());
            // TODO why does &config work?
            let config_thread = config.clone();
            let requires_pre_decider_check = data_provider.requires_pre_decider_check();
            let fs_run_thread = fs_decider_run_batch.to_vec();
            // Output thread information
            // println!(
            //     "Decider batch {}/{} spawned, max steps; {}",
            //     gen_result.batch_no + 1,
            //     data_provider.num_batches(),
            //     batch_info.steps_max,
            // );
            thread::spawn(move || {
                let start = Instant::now();
                let mut undecided_available = true;
                let mut dr;
                if let Some(br) = fs_run_thread[0](
                    &gen_result.machines,
                    requires_pre_decider_check,
                    &config_thread,
                    // TODO should be a problem when moved
                    // config_thread
                ) {
                    // result.add_result(&br.result_decided);
                    let mut m_undecided = br.machines_undecided;
                    dr = br.result_decided;
                    // run other deciders
                    for f in fs_run_thread.iter().skip(1) {
                        if !m_undecided.machines.is_empty() {
                            // let s = format!(
                            //     "2nd decider with {} machines.",
                            //     m_undecided.machines.len()
                            // );
                            // let d2_start = Instant::now();
                            if let Some(br) = f(
                                &m_undecided.machines,
                                PreDeciderRun::DoNotRun,
                                &config_thread,
                            ) {
                                // let duration = d2_start.elapsed();
                                // let decided = m_undecided.machines.len()
                                //     - br.machines_undecided.machines.len();
                                // if decided > 0 {
                                //     print!("{s} {decided} decided. ***");
                                // } else {
                                //     print!("{s} None decided.");
                                // }
                                // let d_per_machine =
                                //     duration.as_millis() / m_undecided.machines.len() as u128;
                                // println!(
                                //     " Duration: {}, per machine: {d_per_machine}",
                                //     duration.as_millis()
                                // );
                                dr.add_result(&br.result_decided);
                                m_undecided = br.machines_undecided;
                            }
                        }
                    }

                    // add remaining undecided to final result
                    for (i, m) in m_undecided.machines.iter().enumerate() {
                        if !dr.add(m, &m_undecided.states[i]) {
                            undecided_available = false;
                            break;
                        }
                    }
                } else {
                    dr = DeciderResultStats::new(&config_thread);
                }

                // if let Some(pre_decider_count) = gen_result.pre_decider_count.as_ref() {
                //     dr.add_pre_decider_count(pre_decider_count);
                //     dr.add_total(pre_decider_count.num_total());
                // }
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
    result.name = format!("BB{}: ", n_states) + "decider.name().as_str()" + " threaded";

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

#[derive(Debug)]
#[non_exhaustive]
pub enum DeciderError {
    ResultWorker(String),
}

impl std::error::Error for DeciderError {}

// impl From<std::io::Error> for DeciderError {
//     fn from(error: std::io::Error) -> Self {
//         ResultWorkerError::FileError(error.to_string())
//     }
// }

impl Display for DeciderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ResultWorkerError::StopRun(message) => write!(f, "{message}"),
            // ResultWorkerError::FileError(message) => write!(f, "{message}"),
            DeciderError::ResultWorker(message) => write!(f, "{message}"),
        }
    }
}
