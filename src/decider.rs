use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    generator::Generator,
    machine::Machine,
    reporter::Reporter,
    result::{DurationGenerator, PreDeciderCount, ResultBatchInfo, ResultDecider},
    status::MachineStatus,
    utils::num_cpus_percentage,
};

pub trait Decider {
    /// Creates a new decider. Allows individual initialization code for each decider and is called for each permutation batch.
    fn new_decider(&self) -> Self;

    /// Returns the result of this decider for one single machine. \
    /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus;

    /// Returns the name of this decider
    fn name(&self) -> String;
}

pub struct DeciderDummy;

impl Decider for DeciderDummy {
    fn new_decider(&self) -> Self {
        DeciderDummy
    }

    fn decide_machine(&mut self, _machine: &Machine) -> MachineStatus {
        MachineStatus::NoDecision
    }

    fn name(&self) -> String {
        "Dummy".to_string()
    }
}

pub fn run_decider_generator_single_thread(
    decider: impl Decider,
    generator: impl Generator,
) -> ResultDecider {
    run_decider_generator_single_thread_reporting(decider, generator, Some(Reporter::default()))
}

pub fn run_decider_generator_single_thread_reporting(
    decider: impl Decider,
    mut generator: impl Generator,
    mut reporter: Option<Reporter>,
) -> ResultDecider {
    let start = Instant::now();
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    let n_states = generator.n_states();
    generator.check_generator_batch_size_request_single_thread();
    let mut result = ResultDecider::new(n_states, 0);
    let requires_pre_decider_check = generator.requires_pre_decider_check();
    result.set_record_machines_max_steps(generator.config().record_machines_max_steps);
    result.set_record_machines_undecided(generator.config().record_machines_undecided);
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
        let r = if requires_pre_decider_check {
            decider_batch_run_with_pre_deciders(decider, &machines, &result.batch_info())
        } else {
            decider_batch_run_without_pre_deciders(decider, &machines, &result.batch_info())
        };
        result.add_result(&r);
        duration_decider += start_decider.elapsed();

        if is_last_batch {
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
    result.duration = DurationGenerator {
        duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };

    // Add the name at the end or it will result in a little performance loss. Reason unknown.
    result.name = format!("BB{}: ", n_states) + decider.name().as_str();

    result
}

// #[inline]
pub fn decider_batch_run_with_pre_deciders(
    mut decider: impl Decider,
    machines: &[Machine],
    batch_info: &ResultBatchInfo,
) -> ResultDecider {
    if machines.is_empty() {
        return ResultDecider::new(batch_info.n_states, 0);
    }
    let mut result = ResultDecider::new_batch(batch_info);
    for machine in machines.iter() {
        let mut status = crate::pre_deciders::run_pre_deciders(machine.transition_table());
        if status == MachineStatus::NoDecision {
            // TODO self_ref
            status = decider.decide_machine(machine);
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
    mut decider: impl Decider,
    machines: &[Machine],
    batch_info: &ResultBatchInfo,
) -> ResultDecider {
    if machines.is_empty() {
        return ResultDecider::new(batch_info.n_states, 0);
    }
    let mut result = ResultDecider::new_batch(batch_info);
    for machine in machines.iter() {
        let status = decider.decide_machine(machine);
        result.add(machine, &status);
    }
    result.add_total(machines.len() as u64);

    result
}

#[derive(Default)]
struct ThreadResultGenerator {
    // #[allow(dead_code)]
    batch_no: usize,
    permutations: Vec<Machine>,
    pre_decider_count: PreDeciderCount,
    num_eliminated: u64,
    duration: Duration,
}

struct ThreadResultDecider {
    #[allow(dead_code)]
    batch_no: usize,
    result: ResultDecider,
    duration: Duration,
}

/// Runs the check in separate threads using the standard reporter.  
/// The generation of the permutations is not threaded.
pub fn run_decider_generator_threaded<D, G>(decider: D, generator: G) -> ResultDecider
where
    D: Decider + Send + 'static,
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
) -> ResultDecider
where
    D: Decider + Send + 'static,
    G: Generator + Send + 'static,
{
    let max_threads = num_cpus_percentage(generator.config().cpu_utilization);
    // if single thread run single
    if max_threads == 1 {
        return run_decider_generator_single_thread_reporting(decider, generator, reporter);
    }

    let start = Instant::now();
    let n_states = generator.n_states();
    let mut result = ResultDecider::new(n_states, ResultDecider::init_steps_max(n_states));
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();
    result.set_record_machines_max_steps(generator.config().record_machines_max_steps);
    result.set_record_machines_undecided(generator.config().record_machines_undecided);
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
            let mut generator_batch = generator.new_from_generator();
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
                let result = Box::new(ThreadResultGenerator {
                    batch_no,
                    permutations,
                    pre_decider_count: generator_batch.pre_decider_count(),
                    num_eliminated: generator_batch.num_eliminated(),
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
                let mut dr = if requires_pre_decider_run {
                    decider_batch_run_with_pre_deciders(
                        decider,
                        &gen_result.permutations,
                        &batch_info,
                    )
                } else {
                    decider_batch_run_without_pre_deciders(
                        decider,
                        &gen_result.permutations,
                        &batch_info,
                    )
                };
                dr.add_pre_decider_count(&gen_result.pre_decider_count);
                dr.add_total(gen_result.num_eliminated);
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
    result.duration = DurationGenerator {
        duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };
    result.name = format!("BB{}: ", n_states) + decider.name().as_str() + " threaded";

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

pub fn run_decider_generator_threaded_reporting_v1<D, G>(
    decider: D,
    generator: G,
    mut reporter: Option<Reporter>,
    cpu_utilization_percent: usize,
) -> ResultDecider
where
    D: Decider + Send + 'static,
    G: Generator + Send + 'static,
{
    let start = Instant::now();
    // let generator = GeneratorFull::new(n_states, batch_size_request, limit);
    // println!("batch_size {}", generator.batch_size);
    let n_states = generator.n_states();
    let mut result = ResultDecider::new(n_states, ResultDecider::init_steps_max(n_states));
    let mut duration_generator = Duration::default();
    let mut duration_decider = Duration::default();

    result.set_record_machines_max_steps(100);
    result.set_record_machines_undecided(25);
    // let mut reporter = Reporter::new(2000, 30);
    let max_threads = num_cpus_percentage(cpu_utilization_percent);
    // TODO allow single thread
    assert!(max_threads >= 2);
    let mut threads_generator = (max_threads / 2).max(1);
    let mut batch_no = 0;
    let (send_finished_thread_generator, receive_finished_thread_generator) =
        std::sync::mpsc::channel();
    let (send_finished_thread_decider, receive_finished_thread_decider) =
        std::sync::mpsc::channel::<ThreadResultDecider>();
    let mut num_threads_generator_running = 0;
    let mut num_threads_decider_running = 0;
    let mut buffer_gen_result = Vec::new();
    let mut last_gen_thread_change_batch_no = 0;
    let mut last_buf_len = 0;

    loop {
        // triggers a thread sleep if none have finished
        let mut has_finished = false;
        if num_threads_generator_running < threads_generator
        // && batch_no < generator.get_num_batches() // not required, checked within
        {
            let send_finished_thread_gen = send_finished_thread_generator.clone();
            let mut generator_batch = generator.new_from_generator();
            // Handle not used as it would create a very large vector to maintain
            thread::spawn(move || {
                let start = Instant::now();
                let (permutations, _is_last_batch) =
                    generator_batch.generate_permutation_batch_no(batch_no);

                // Send finished signal to allow decider to run.
                let result = Box::new(ThreadResultGenerator {
                    batch_no,
                    permutations,
                    pre_decider_count: generator_batch.pre_decider_count(),
                    num_eliminated: 0,
                    duration: start.elapsed(),
                });
                // TODO sending result doubles the time as the data is copied within memory
                // or not? single thread is twice as fast, maybe cache issue
                // Arc?
                // https://doc.rust-lang.org/rust-by-example/std/arc.html
                send_finished_thread_gen.send(result).unwrap();
            });
            batch_no += 1;
            if batch_no == generator.num_batches() {
                // turn off generator threads, they are not needed any more
                threads_generator = 0
            }
            num_threads_generator_running += 1;

            // Spawn parallel generators for all CPUs to build a buffer
            if num_threads_generator_running < threads_generator
                && num_threads_decider_running + num_threads_generator_running < max_threads
            {
                continue;
            }
        }

        // #[cfg(all(debug_assertions, feature = "debug"))]
        // println!(
        //     "Threads {} ({}, {}) - batch {batch_no}/{}, buffer {}",
        //     num_threads_generator_running + num_threads_decider_running,
        //     num_threads_generator_running,
        //     num_threads_decider_running,
        //     generator.num_batches(),
        //     buffer_gen_result.len(),
        // );

        // adjust threads between generator and decider to optimize usage
        if batch_no - last_gen_thread_change_batch_no > threads_generator * 2 {
            if buffer_gen_result.len() > max_threads
                && buffer_gen_result.len() >= last_buf_len
                && threads_generator > 1
            {
                // generator is too fast, decider cannot keep up
                threads_generator -= 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads reduced to   {threads_generator} (batch {batch_no})");
            } else if buffer_gen_result.len() < max_threads - threads_generator
                && buffer_gen_result.len() <= last_buf_len
            // && batch_no > max_threads * 2
            && threads_generator < max_threads - 1
            && threads_generator > 0
            {
                threads_generator += 1;
                last_gen_thread_change_batch_no = batch_no;
                last_buf_len = buffer_gen_result.len();
                // println!("  *** Gen Threads increased to {threads_generator} (batch {batch_no})");
            }
        }

        // Wait until a permutation pack is available, then run decider. This also frees one CPU.
        if num_threads_generator_running > 0 {
            if buffer_gen_result.is_empty() {
                // no data to work for decider, must wait
                let thread_result_gen = receive_finished_thread_generator.recv().unwrap();
                duration_generator += thread_result_gen.duration;
                buffer_gen_result.push(*thread_result_gen);
                num_threads_generator_running -= 1;
                has_finished = true;
            } else {
                // collect all finished permutation batches
                while let Ok(thread_result_gen) = receive_finished_thread_generator.try_recv() {
                    duration_generator += thread_result_gen.duration;
                    buffer_gen_result.push(*thread_result_gen);
                    num_threads_generator_running -= 1;
                    has_finished = true;
                }
            }
        }

        // Check if new decider thread can be started
        // check available threads, keep one open for next generator
        if !buffer_gen_result.is_empty()
            && max_threads - threads_generator.max(num_threads_generator_running)
                > num_threads_decider_running
        {
            // Thread is available, start decider
            let send_finished_thread_dec = send_finished_thread_decider.clone();
            // move result out of vector to move into thread
            let gen_result = buffer_gen_result.remove(0);
            let batch_info = result.batch_info();
            let requires_pre_decider_run = generator.requires_pre_decider_check();
            let decider = decider.new_decider();
            // println!(
            //     "Decider batch {}/{} spawned, max steps; {}",
            //     gen_result.batch_no,
            //     generator.get_num_batches(),
            //     batch_info.steps_max,
            // );
            thread::spawn(move || {
                let start = Instant::now();
                // let decider = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);

                let mut dr = if requires_pre_decider_run {
                    decider_batch_run_with_pre_deciders(
                        decider,
                        &gen_result.permutations,
                        &batch_info,
                    )
                } else {
                    decider_batch_run_without_pre_deciders(
                        decider,
                        &gen_result.permutations,
                        &batch_info,
                    )
                };
                dr.add_pre_decider_count(&gen_result.pre_decider_count);
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
            num_threads_decider_running += 1;
        }

        // Check if deciders have finished
        loop {
            let thread_result_dec = receive_finished_thread_decider.try_recv();
            match thread_result_dec {
                Ok(dr) => {
                    result.add_result(&dr.result);
                    duration_decider += dr.duration;
                    num_threads_decider_running -= 1;
                    has_finished = true;
                    // println!(
                    //     "Decider batch {}/{} finished",
                    //     dr.batch_no,
                    //     generator.get_num_batches()
                    // );

                    // Output info on progress
                    if let Some(reporter) = reporter.as_mut() {
                        if reporter.is_due_progress() {
                            reporter.report(result.num_checked_total(), generator.limit(), &result);
                        }
                    }
                    // if reporter.is_due_progress() {
                    //     let mio = (result.num_checked as f64 / 100_000.0).round() / 10.0;
                    //     let p = (result.num_checked as f64 / generator.limit() as f64 * 1000.0)
                    //         .round()
                    //         / 10.0;
                    //     println!("Working: {} = {} million, {p}%", result.num_checked, mio);
                    //     reporter.reset_last_report_progress_time();
                    //     if reporter.is_due_detail() {
                    //         println!("\nCurrent result\n{}", result);
                    //         reporter.reset_last_report_detail_time();
                    //     }
                    // }
                }
                Err(_) => {
                    // no more finished deciders, just exit this inner loop
                    break;
                }
            }
        }

        if num_threads_generator_running + num_threads_decider_running == 0
            && buffer_gen_result.is_empty()
        {
            break;
        }

        if !has_finished {
            thread::sleep(Duration::from_millis(1));
        }
    }
    result.duration = DurationGenerator {
        duration_generator,
        duration_decider,
        duration_total: start.elapsed(),
    };
    result.name = format!("BB{}: ", n_states) + decider.name().as_str() + " threaded";

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
