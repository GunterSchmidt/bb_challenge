use std::{
    fmt::Display,
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::Config,
    data_provider::DataProvider,
    data_provider_threaded::DataProviderThreaded,
    decider_cycler_v4::DeciderCyclerV4,
    decider_result::{
        BatchData, BatchResult, DeciderResultStats, DurationDataProvider, EndReason,
        MachinesUndecided, PreDeciderCount,
    },
    decider_result_worker::ResultWorker,
    machine::Machine,
    pre_decider::{run_pre_decider_simple, run_pre_decider_strict, PreDecider, PreDeciderRun},
    reporter::Reporter,
    status::MachineStatus,
    utils::num_cpus_percentage,
    ResultUnitEndReason,
};

pub type ResultDecider = std::result::Result<(), DeciderError>;
pub type FnDeciderRunBatchV2 = fn(&mut BatchData) -> ResultUnitEndReason;
pub type FnResultWorker = fn(&BatchData) -> ResultUnitEndReason;

/// The deciders need to return Self to be able to make a new decider for each thread.
/// This makes them not object save and thus cannot be passed in a Vec.
pub enum DeciderEnum {
    PreDecider(Box<crate::pre_decider::PreDecider>),
    LoopV4(Box<crate::decider_cycler_v4::DeciderCyclerV4>),
    HoldLong(Box<crate::decider_hold_u128_long::DeciderHoldU128Long>),
}

pub enum DeciderEnumV2<'a> {
    PreDecider(&'a mut PreDecider),
    LoopV4(&'a mut DeciderCyclerV4),
    // LoopV4(Box<crate::decider_loop_v4::DeciderLoopV4>),
    // HoldLong(Box<crate::decider_hold_u128_long::DeciderHoldU128Long>),
}

// TODO implement
pub struct DeciderConfig<'a> {
    f_decider: FnDeciderRunBatchV2,
    // TODO move execution to thread, requires thread safety
    f_result_worker: FnResultWorker,
    config: &'a Config,
}

impl<'a> DeciderConfig<'a> {
    pub fn new(
        f_decider: FnDeciderRunBatchV2,
        f_result_worker: FnResultWorker,
        config: &'a Config,
    ) -> Self {
        Self {
            f_decider,
            f_result_worker,
            config,
        }
    }

    // pub fn new_no_result_worker(
    //     f_decider: &'a fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>,
    //     config: &'a Config,
    // ) -> Self {
    //     let f_result_worker = decider_result_worker::no_worker;
    //     Self {
    //         f_decider,
    //         f_result_worker: &f_result_worker,
    //         config,
    //     }
    // }

    pub fn f_decider(&self) -> FnDeciderRunBatchV2 {
        self.f_decider
    }

    pub fn f_result_worker(&self) -> FnResultWorker {
        self.f_result_worker
    }

    pub fn config(&self) -> &'a Config {
        self.config
    }
}

pub struct DeciderConfigTest<'a> {
    // f_decider: &'a fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>,
    // // TODO move execution to thread, requires thread safety
    pub f_decider: FnDeciderRunBatchV2,
    pub f_result_worker: FnResultWorker,
    pub config: &'a Config,
}

impl<'a> DeciderConfigTest<'a> {
    pub fn new(
        f_decider: FnDeciderRunBatchV2,
        f_result_worker: FnResultWorker,
        config: &'a Config,
    ) -> Self {
        Self {
            f_decider,
            f_result_worker,
            config,
        }
    }
}

// pub struct DeciderConfigThreaded<'a> {
//     f_decider:
//         (fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>) + Send + Copy + 'static,
//     // TODO move to thread
//     f_result_worker:
//         &'a fn(&BatchResult, &Config) -> Result<(), decider_result_worker::ResultWorkerError>,
//     config: &'a Config,
// }

pub trait DeciderMinimal {
    /// Returns the result of this decider for one single machine. \
    /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
    fn decide_machine_minimal(&mut self, machine: &Machine) -> MachineStatus;

    /// Returns the name of this decider
    fn name_minimal(&self) -> &str;
}

pub struct DeciderId {
    pub no: u16,
    pub name: &'static str,
}

pub trait Decider {
    // TODO into id, name struct
    /// Returns the name of this decider
    fn id(&self) -> usize;

    /// Returns the name of this decider
    fn name(&self) -> &str;

    // fn new_from_config(config: &Config) -> Self;
    // fn new_from_self(&self) -> Self;
    // fn new_from_self(&self, config: &Config) -> Self;

    // /// clears any data for a fresh batch (recycle)
    // fn clear(&mut self);

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

    fn decider_run_batch_v2(batch_data: &mut BatchData) -> ResultUnitEndReason;
}

// impl From<&Config> for Decider {
//     fn from(value: &Config) -> Self {
//         Self::new_from_config(config)
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
        PreDeciderRun::RunNormal => {
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
        PreDeciderRun::RunStartBRightOnly => {
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

#[inline]
pub fn decider_generic_run_batch_v2(
    mut decider: impl Decider,
    batch_data: &mut BatchData,
) -> ResultUnitEndReason {
    if batch_data.machines.is_empty() {
        return Err(EndReason::NoBatchData);
    }
    // let mut machines_undecided: Vec<MachineUndecided> = Vec::with_capacity(machines.len());
    // TODO optimize undecided. Possible collect only ids, if count is same then to_vec for machines.
    // let cap = if batch_data.run_predecider != PreDeciderRun::DoNotRun {
    //     // loop decider should run first, which eliminates most machines
    //     batch_data.machines.len() / 100
    // } else {
    //     batch_data.machines.len()
    // };
    // let mut machines_undecided = MachinesUndecided::new(cap);
    // let mut result_decided = DeciderResultStats::new(config);
    match batch_data.run_predecider {
        PreDeciderRun::DoNotRun => {
            for machine in batch_data.machines.iter() {
                // TODO self_ref
                let status = decider.decide_machine(machine);
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        batch_data.result_decided.add(machine, &status);
                    }
                }
            }
        }
        PreDeciderRun::RunNormal => {
            for machine in batch_data.machines.iter() {
                let mut status = run_pre_decider_simple(machine.transition_table());
                if status == MachineStatus::NoDecision {
                    status = decider.decide_machine(machine);
                }
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        batch_data.result_decided.add(machine, &status);
                    }
                }
            }
        }
        PreDeciderRun::RunStartBRightOnly => {
            for machine in batch_data.machines.iter() {
                let mut status = run_pre_decider_strict(machine.transition_table());
                if status == MachineStatus::NoDecision {
                    status = decider.decide_machine(machine);
                }
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        batch_data.result_decided.add(machine, &status);
                    }
                }
            }
        }
    }
    batch_data
        .result_decided
        .add_total(batch_data.machines.len() as u64);

    Ok(())
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
// This is just a convenience function to avoid creating a vector of functions to call.
pub fn run_decider_data_provider_single_thread<F, W>(
    f_decider_run_batch: &F,
    data_provider: impl DataProvider,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let total = data_provider.num_machines_total();
    run_decider_chain_data_provider_single_thread_reporting(
        &[f_decider_run_batch],
        data_provider,
        Some(Reporter::new_default(total)),
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
    let total = data_provider.num_machines_total();
    run_decider_chain_data_provider_single_thread_reporting(
        fs_decider_run_batch,
        data_provider,
        Some(Reporter::new_default(total)),
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
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    let n_states = config.n_states();
    data_provider.set_batch_size_for_num_threads(1);
    let mut result = DeciderResultStats::new(config);
    let requires_pre_decider_check = data_provider.requires_pre_decider_check();
    result.set_record_machines_max_steps(config.limit_machines_max_steps());
    result.set_limit_machines_undecided(config.limit_machines_undecided());
    // let mut batch_no = 0;
    // copy config so init steps can be updated
    // TODO maybe handle init_steps differently?
    // required to have individual update of init steps (really?)
    let mut config = config.clone();
    loop {
        let start_gen = Instant::now();
        // if batch_no >= 409 {
        //     println!()
        // }
        let data = data_provider.machine_batch_next();
        // batch_no += 1;
        // if batch_no % 100 == 0 {
        // println!("Batch no. {batch_no} / {}", data_provider.num_batches());
        // }
        if let Some(pre) = data.pre_decider_count {
            result.add_pre_decider_count(&pre);
            result.add_total(pre.num_total());
        }
        duration_data_provider += start_gen.elapsed();
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
                result.end_reason = e;
                // TODO remove stop_run, check on error
                stop_run = true;
                // eprintln!("{}", e);
            }
            config.increase_steps_max_init(br.result_decided.steps_max());
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
        match data.end_reason {
            EndReason::AllMachinesChecked => todo!(),
            EndReason::Error(_, _) => todo!(),
            EndReason::IsLastBatch => {
                result.end_reason = EndReason::AllMachinesChecked;
                break;
            }
            EndReason::MachineLimitReached(_) => todo!(),
            EndReason::NoBatchData => todo!(),
            EndReason::NoMoreData => {
                result.end_reason = data.end_reason;
                break;
            }
            EndReason::StopRequested(_) => todo!(),
            EndReason::UndecidedLimitReached(_) => todo!(),
            EndReason::Undefined => {}
            EndReason::Working => {}
        }

        // Output info on progress
        if let Some(reporter) = reporter.as_mut() {
            if reporter.is_due_progress() {
                let s = reporter.report_stats(result.num_processed_total(), &result);
                println!("{s}");
            }
        }
    }
    result.duration = DurationDataProvider {
        duration_data_provider,
        duration_decider,
        duration_total: start.elapsed(),
    };

    // Add the name at the end or it will result in a little performance loss. Reason unknown.
    // TODO name
    result.set_name(format!("BB{}: '{}'", n_states, "decider.name()"));

    result
}

// pub fn run_decider_chain_data_provider_single_thread_reporting_v2<F, W>(
//     mut data_provider: impl DataProvider,
//     decider_config: &[DeciderConfig],
//     mut reporter: Option<Reporter>,
// ) -> DeciderResultStats
// where
//     F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>, // + Send + Copy + 'static,
//     W: Fn(&BatchResult, &Config) -> ResultWorker,
// {
//     // TODO check filled
//     let start = Instant::now();
//     let mut duration_data_provider = Duration::default();
//     let mut duration_decider = Duration::default();
//     let first_config = decider_config.first().expect("No decider given").config;
//     let n_states = first_config.n_states();
//     data_provider.set_batch_size_for_num_threads(1);
//     let mut result = DeciderResultStats::new(first_config);
//     let requires_pre_decider_check = data_provider.requires_pre_decider_check();
//     result.set_record_machines_max_steps(first_config.limit_machines_max_steps());
//     result.set_limit_machines_undecided(first_config.limit_machines_undecided());
//     // copy config so init steps can be updated
//     // TODO maybe handle init_steps differently?
//     // required to have individual update of init steps (really?)
//     // let mut config = first_config.clone();
//     // loop over batch_next()
//     loop {
//         // Generator or Data Provider: get next batch
//         let start_gen = Instant::now();
//         let data = data_provider.machine_batch_next();
//         // if batch_no % 100 == 0 {
//         // println!("Batch no. {batch_no} / {}", data_provider.num_batches());
//         // }
//         if let Some(pre) = data.pre_decider_count {
//             result.add_pre_decider_count(&pre);
//             result.add_total(pre.num_total());
//         }
//         duration_data_provider += start_gen.elapsed();
//
//         // Run deciders on batch
//         let start_decider = Instant::now();
//         // run first decider which includes pre-decider elimination
//         let mut undecided_available = true;
//         let mut stop_run = false;
//         for dc in decider_config {
//             if let Some(br) =
//                 dc.f_decider()(&data.machines, requires_pre_decider_check, dc.config())
//             {
//                 result.add_result(&br.result_decided);
//                 // call user analyzer/worker so result can be dealt with individually (e.g. save)
//                 if let Err(e) = dc.f_result_worker()(&br, dc.config()) {
//                     result.end_reason = EndReason::Error(e.to_string());
//                     stop_run = true;
//                     // eprintln!("{}", e);
//                 }
//                 config.increase_steps_max_init(br.result_decided.steps_max());
//                 let mut m_undecided = br.machines_undecided;
//
//                 if !stop_run {
//                     // run other deciders
//                     for f in fs_decider_run_batch.iter().skip(1) {
//                         if !m_undecided.machines.is_empty() {
//                             if let Some(br) =
//                                 f(&m_undecided.machines, PreDeciderRun::DoNotRun, &config)
//                             {
//                                 result.add_result(&br.result_decided);
//                                 m_undecided = br.machines_undecided;
//                             }
//                         }
//                     }
//                 }
//
//                 // add remaining undecided to final result
//                 for (i, m) in m_undecided.machines.iter().enumerate() {
//                     if !result.add(m, &m_undecided.states[i]) {
//                         result.end_reason =
//                             EndReason::UndecidedLimitReached(result.limit_machines_undecided());
//                         undecided_available = false;
//                         break;
//                     }
//                 }
//             }
//         }
//         // println!(
//         //     "batch {batch_no}: total {}, should be {}",
//         //     result.num_checked_total(),
//         //     batch_no * data_provider.batch_size()
//         // );
//
//         // let undecided_available = result.add_result(&br.result_decided);
//         duration_decider += start_decider.elapsed();
//
//         // end if undecided limit has been reached
//         if stop_run || !undecided_available {
//             break;
//         }
//         match data.end_reason {
//             EndReason::AllMachinesChecked => todo!(),
//             EndReason::Error(_) => todo!(),
//             EndReason::IsLastBatch => {
//                 result.end_reason = EndReason::AllMachinesChecked;
//                 break;
//             }
//             EndReason::MachineLimitReached(_) => todo!(),
//             EndReason::NoMoreData => {
//                 result.end_reason = data.end_reason;
//                 break;
//             }
//             EndReason::StopRequested(_) => todo!(),
//             EndReason::UndecidedLimitReached(_) => todo!(),
//             EndReason::Undefined => {}
//             EndReason::Working => {}
//         }
//
//         // Output info on progress
//         if let Some(reporter) = reporter.as_mut() {
//             if reporter.is_due_progress() {
//                 let s = reporter.report_stats(result.num_processed_total(), &result);
//                 println!("{s}");
//             }
//         }
//     }
//     result.duration = DurationDataProvider {
//         duration_data_provider,
//         duration_decider,
//         duration_total: start.elapsed(),
//     };
//
//     // Add the name at the end or it will result in a little performance loss. Reason unknown.
//     // TODO name
//     result.set_name(format!("BB{}: '{}'", n_states, "decider.name()"));
//
//     result
// }

/// Runs the given decider in a single thread.
// This is just a convenience function to avoid creating a vector.
pub fn run_decider_threaded_data_provider_single_thread<F, G, W>(
    f_decider_run_batch: F,
    data_provider: G,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProvider,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let total = data_provider.num_machines_total();
    run_decider_chain_threaded_data_provider_single_thread_reporting(
        &[f_decider_run_batch],
        data_provider,
        Some(Reporter::new_default(total)),
        config,
        f_result_worker,
    )
}

/// Runs the check in separate threads using the standard reporter.
/// The generation of the permutations is not threaded.
pub fn run_decider_chain_threaded_data_provider_single_thread<F, G, W>(
    fs_decider_run_batch: &[F],
    data_provider: G,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProvider,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let total = data_provider.num_machines_total();
    run_decider_chain_threaded_data_provider_single_thread_reporting(
        fs_decider_run_batch,
        data_provider,
        Some(Reporter::new_default(total)),
        config,
        f_result_worker,
    )
}

/// TODO Doc
/// Runs the data provider and the deciders in separate threads (only the deciders can have multiple threads)
/// using a custom reporter (or None to omit reporting).
/// The data provider is running on the main thread and the deciders are spawned.
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
pub fn run_decider_chain_threaded_data_provider_single_thread_reporting<F, G, W>(
    fs_decider_run_batch: &[F],
    mut data_provider: G,
    mut reporter: Option<Reporter>,
    config: &Config,
    f_result_worker: &W,
) -> DeciderResultStats
where
    F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult> + Send + Copy + 'static,
    G: DataProvider,
    W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let max_threads = num_cpus_percentage(config.cpu_utilization_percent());
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
    // TODO some fine tuning. Now the decider uses all threads, which leads to more load than CPUs are available.
    // If we leave one open, then CPU is not used in case of quick data provider.
    let max_threads_decider = max_threads;

    let start = Instant::now();
    let mut result = DeciderResultStats::new_init_steps_max(
        config,
        DeciderResultStats::init_steps_max(config.n_states()),
    );
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    result.set_record_machines_max_steps(config.limit_machines_max_steps());
    result.set_limit_machines_undecided(config.limit_machines_undecided());
    // let mut max_threads_gen = (max_threads / 2 + 1).max(1);
    let mut batch_no = 0;
    let (send_finished_thread_decider, receive_finished_thread_decider) =
        std::sync::mpsc::channel::<ThreadResultDecider>();
    let mut num_threads_decider_running = 0;
    let mut buffer_gen_result = Vec::new();
    let max_buffer_gen = (max_threads - 1).max(max_threads + 4);
    let mut is_gen_finished = false;
    // required to have individual update of init steps (really?)
    // let mut config = config.clone();

    loop {
        // triggers a thread sleep if none have finished
        let mut do_sleep = true;
        if !is_gen_finished && buffer_gen_result.len() < max_buffer_gen
        // && batch_no < data_provider.num_batches() // not required, checked within
        {
            do_sleep = false;
            // let mut data_provider_batch = data_provider.new_from_data_provider();
            // count_gen_spawn += 1;
            // println!(
            //     "Generator batch {}/{} created",
            //     batch_no + 1,
            //     data_provider.num_batches(),
            // );
            let start = Instant::now();
            let data = data_provider.machine_batch_next();
            match data.end_reason {
                EndReason::AllMachinesChecked => todo!(),
                EndReason::Error(_, _) => todo!(),
                EndReason::IsLastBatch => is_gen_finished = true,
                EndReason::MachineLimitReached(_) => todo!(),
                EndReason::NoBatchData => todo!(),
                EndReason::NoMoreData => todo!(),
                EndReason::StopRequested(_) => todo!(),
                EndReason::UndecidedLimitReached(_) => todo!(),
                EndReason::Undefined => todo!(),
                EndReason::Working => {}
            }
            batch_no = data.batch_no;
            buffer_gen_result.push(data);
            // is_gen_finished = batch_no == data_provider.num_batches();
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
                result.add_pre_decider_count(pre_decider_count);
                result.add_total(pre_decider_count.num_total());
            }
            // let batch_info = result.batch_info();
            let mut config_thread = config.clone();
            config_thread.increase_steps_max_init(result.steps_max());
            config_thread.set_limit_machines_undecided(result.num_undecided_free());
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
                // start first decider with pre-decider if required
                if let Some(br) = fs_run_thread[0](
                    &gen_result.machines,
                    requires_pre_decider_check,
                    &config_thread,
                ) {
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
                                //     let s = format!(
                                //         "2nd decider with {} machines.",
                                //         m_undecided.machines.len()
                                //     );
                                //     println!("{s} {decided} decided. ***");
                                //     // } else {
                                //     //     print!("{s} None decided.");
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
                        } else {
                            break;
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
        }

        // Output info on progress
        if let Some(reporter) = reporter.as_mut() {
            if reporter.is_due_progress() {
                let s = reporter.report_stats(result.num_processed_total(), &result);
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
            if batch_no + 1 < data_provider.num_batches() {
                panic!(
                    "All empty! Batch {batch_no}/{}, buffer {}",
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
    result.set_name(format!(
        "BB{}: '{}' threaded",
        config.n_states(),
        "decider.name()"
    ));

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
    let total = data_provider.num_machines_total();
    run_decider_chain_data_provider_threaded_reporting(
        &[f_decider_run_batch],
        data_provider,
        Some(Reporter::new_default(total)),
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
    let total = data_provider.num_machines_total();
    run_decider_chain_data_provider_threaded_reporting(
        fs_decider_run_batch,
        data_provider,
        Some(Reporter::new_default(total)),
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
    let max_threads = num_cpus_percentage(config.cpu_utilization_percent());
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
    let mut result = DeciderResultStats::new_init_steps_max(
        config,
        DeciderResultStats::init_steps_max(config.n_states()),
    );
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    result.set_record_machines_max_steps(config.limit_machines_max_steps());
    result.set_limit_machines_undecided(config.limit_machines_undecided());
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
    // let mut count_gen_spawn = 0;
    // let mut config = config.clone();

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
                result.add_pre_decider_count(pre_decider_count);
                result.add_total(pre_decider_count.num_total());
            }
            // let batch_info = result.batch_info();
            let mut config_thread = config.clone();
            config_thread.increase_steps_max_init(result.steps_max());
            config_thread.set_limit_machines_undecided(result.num_undecided_free());
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
                    let s = reporter.report_stats(result.num_processed_total(), &result);
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
    result.set_name(format!(
        "BB{}: '{}' threaded",
        config.n_states(),
        "decider.name()"
    ));

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
