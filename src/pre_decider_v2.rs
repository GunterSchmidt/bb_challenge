//! This pre-decider is designed to check the conditions and mark the machine as decided.
//! This should allow to use the same batch data for multiple deciders.
//! The idea is to separate data provider from deciders.
//! 1. Run Generator, get package (need other data?)
//! 2. Run first decider for batch and include Pre-Decider in run
//! (typically pre-decider and cycler eliminate most machines, so no need to pass the undecided from pre-decider around).
//! 3. Run result_decider_worker for first decider.
//! 3. Run second decider for same machine
#![allow(unused)]
use std::time::{Duration, Instant};

use crate::{
    config::Config,
    data_provider::DataProvider,
    decider::{self, DeciderConfig, ResultDecider},
    decider_result::{
        BatchData, BatchResult, DeciderResultStats, DurationDataProvider, EndReason,
        MachinesUndecided,
    },
    decider_result_worker::{self, ResultWorker},
    generator_full::GeneratorFull,
    generator_reduced::GeneratorReduced,
    machine::Machine,
    pre_decider::PreDeciderRun,
    reporter::{self, Reporter},
    status::{MachineStatus, PreDeciderReason},
    transition_symbol2::TRANSITIONS_FOR_A0,
    ResultUnitEndReason,
};

// pub type ResultBatchV2 = ResultUnitEndReason;

pub fn test_run_multiple_decider_test_pre(
    decider_config: &[DeciderConfig],
    multi_core: bool,
) -> DeciderResultStats {
    // let f_result_worker = decider_result_worker::no_worker;
    let first_config = decider_config.first().expect("No decider given").config();

    // let generator = GeneratorFull::new(first_config);
    let generator = GeneratorReduced::new(first_config);
    let result = if multi_core {
        // decider::run_decider_chain_data_provider_threaded(
        //     fs_decider,
        //     generator,
        //     config,
        //     &f_result_worker,
        // )
        todo!()
    } else {
        let reporter = Reporter::new_default(generator.num_machines_total());
        run_decider_chain_data_provider_single_thread_reporting(
            decider_config,
            generator,
            Some(reporter),
        )
    };

    result
}

fn decider_run_batch<'a>(
    decider_config: &DeciderConfig,
    machines: &'a [crate::machine::Machine],
) -> ResultUnitEndReason {
    todo!()
}

pub fn run_decider_chain_data_provider_single_thread_reporting(
    // <F, W>
    decider_config: &[DeciderConfig],
    mut data_provider: impl DataProvider,
    mut reporter: Option<Reporter>,
) -> DeciderResultStats
// where
//     F: Fn(&[Machine], PreDeciderRun, &Config) -> Option<BatchResult>, // + Send + Copy + 'static,
//     W: Fn(&BatchResult, &Config) -> ResultWorker,
{
    let first_config = decider_config.first().expect("No decider given").config();

    let start = Instant::now();
    let mut duration_data_provider = Duration::default();
    let mut duration_decider = Duration::default();
    // let n_states = config.n_states();
    data_provider.set_batch_size_for_num_threads(1);
    let mut result_main = DeciderResultStats::new(first_config);
    let requires_pre_decider_check = data_provider.requires_pre_decider_check();
    // result_main.set_record_machines_max_steps(config.limit_machines_max_steps());
    // result_main.set_limit_machines_undecided(config.limit_machines_undecided());
    // let mut batch_no = 0;
    // copy config so init steps can be updated
    // TODO maybe handle init_steps differently? -> atomic
    // required to have individual update of init steps (really?)
    // let mut config = config.clone();
    loop {
        // generate or get one batch of machines
        let start_gen = Instant::now();
        let data = data_provider.machine_batch_next();
        // batch_no += 1;
        // if batch_no % 100 == 0 {
        // println!("Batch no. {batch_no} / {}", data_provider.num_batches());
        // }
        if let Some(pre) = data.pre_decider_count {
            result_main.add_pre_decider_count(&pre);
            result_main.add_total(pre.num_total());
        }
        duration_data_provider += start_gen.elapsed();

        // TODO check on end_reason

        // run deciders
        let start_decider = Instant::now();
        // run first decider which includes pre-decider elimination
        let mut undecided_available = true;
        let mut stop_run = false;

        let first_decider_config = decider_config.first().expect("No decider!");
        let mut batch_data = BatchData {
            machines: &data.machines,
            result_decided: DeciderResultStats::new_init_steps_max(
                &first_config,
                result_main.steps_max(),
            ),
            machines_undecided: Default::default(),
            batch_no: data.batch_no,
            num_batches: data_provider.num_batches(),
            decider_id: 0,
            config: first_config,
            run_predecider: data_provider.requires_pre_decider_check(),
        };
        if first_decider_config.f_decider()(&mut batch_data).is_ok() {
            result_main.add_result(&batch_data.result_decided);
            // call user analyzer/worker so result can be dealt with individually (e.g. save)
            if let Err(e) = first_decider_config.f_result_worker()(&batch_data) {
                result_main.end_reason = e;
                stop_run = true;
                // eprintln!("{}", e);
            }

            let mut m_undecided;
            // run other deciders
            // batch_data.run_predecider = PreDeciderRun::DoNotRun;
            for d in decider_config.iter().skip(1) {
                if !batch_data.machines_undecided.machines.is_empty() && !stop_run {
                    m_undecided = batch_data.machines_undecided.machines;
                    // borrow checker requires new object instead of just updating ref to machines
                    batch_data = BatchData {
                        machines: &m_undecided,
                        result_decided: DeciderResultStats::new_init_steps_max(
                            &first_config,
                            result_main.steps_max(),
                        ),
                        machines_undecided: Default::default(),
                        batch_no: data.batch_no,
                        num_batches: data_provider.num_batches(),
                        decider_id: 0,
                        config: d.config(),
                        run_predecider: PreDeciderRun::DoNotRun,
                    };

                    match d.f_decider()(&mut batch_data) {
                        Ok(()) => {
                            result_main.add_result(&batch_data.result_decided);
                            // call user analyzer/worker so result can be dealt with individually (e.g. save)
                            if let Err(e) = d.f_result_worker()(&batch_data) {
                                result_main.end_reason = e;
                                stop_run = true;
                                // eprintln!("{}", e);
                            }
                        }
                        Err(e) => {
                            result_main.end_reason = e;
                            stop_run = true;
                        }
                    }
                }
            }

            // add remaining undecided to final result
            for (i, m) in batch_data.machines_undecided.machines.iter().enumerate() {
                if !result_main.add(m, &batch_data.machines_undecided.states[i]) {
                    result_main.end_reason =
                        EndReason::UndecidedLimitReached(result_main.limit_machines_undecided());
                    undecided_available = false;
                    break;
                }
            }
        }
        // TODO remove check
        #[cfg(feature = "bb_debug")]
        if result_main.num_processed_total()
            != (data.batch_no + 1) as u64 * data_provider.batch_size() as u64
        {
            println!(
                "batch {}: total {}, should be {}",
                data.batch_no,
                result_main.num_processed_total(),
                data.batch_no * data_provider.batch_size()
            );
        }

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
                result_main.end_reason = EndReason::AllMachinesChecked;
                break;
            }
            EndReason::MachineLimitReached(_) => todo!(),
            EndReason::NoBatchData => todo!(),
            EndReason::NoMoreData => {
                result_main.end_reason = data.end_reason;
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
                let s = reporter.report_stats(result_main.num_processed_total(), &result_main);
                println!("{s}");
            }
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
