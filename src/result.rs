use num_format::{Buffer, ToFormattedString};
use std::{fmt::Display, time::Duration};

#[cfg(feature = "bb_counter_stats")]
use crate::status::COUNTER_ARRAY_SIZE;
use crate::{
    generator,
    machine::Machine,
    machine_info::MachineInfo,
    status::{EndlessReason, MachineStatus, PreDeciderReason},
    utils::{duration_as_ms_rounded, user_locale},
    StepType,
};

// #[cfg(feature = "bb_use_result_large")]
// pub type ResultType = ResultLarge;
// #[cfg(not(feature = "bb_use_result_large"))]
// pub type ResultType = ResultSmall;

// TODO Decide if the fields should not be pub
#[derive(Debug, Default)]
pub struct ResultDecider {
    // Result Small
    /// Number of machines which have been tested by the deciders (not pre-deciders) or have been eliminated during
    /// generation. This needs to be the num_turing_machines if not limited.
    pub num_evaluated: u64,
    /// Tested machines which come to a hold.
    /// This does not include machines which have not been tested, e.g. because they cannot produce the maximal steps.
    pub num_hold: u64,
    /// Eliminated machines which cannot reach the maximum steps (may or may not hold).
    pub num_not_max: u64,
    /// Tested machines which did not come to a result.
    pub num_undecided: u64,
    pub pre_decider_count: PreDeciderCount,
    pub endless_count: EndlessCount,

    /// Number of states used for the Turing machines.
    pub n_states: usize,
    /// Number of possible Turing machines (max seven states for u64 size limit).
    pub num_turing_machines: u64,

    pub num_not_max_too_many_hold_transitions: u64,
    /// Eliminated machines which cannot reach the maximum steps because not all states were used.
    pub num_not_max_not_all_states_used: u64,

    // steps
    pub steps_max: StepMaxResult,
    // pub steps_max: StepType,
    // pub num_machines_for_steps_max: u16,
    // machine_max_steps: Option<MachineInfo>,
    /// Store all machines with max steps up to this limit.
    // record_machines_max_steps: u16,
    // machines_max_steps: Option<Vec<MachineInfo>>,
    /// Store all machines Undecided up to this limit.
    record_machines_undecided: u32,
    // machine_undecided: Option<MachineInfo>,
    machines_undecided: Option<Vec<MachineInfo>>,

    // for statistical purposes and performance tests
    pub duration: DurationGenerator,
    /// Optional name of the test or any other info.
    pub name: String,

    // Additional statistics, possibly make this a struct in an Option to turn on at runtime
    // TODO HashMaps for larger
    #[cfg(feature = "bb_counter_stats")]
    /// Array for the first 100 steps, [0] holds all which are greater
    pub hold_steps_stats: [StepType; COUNTER_ARRAY_SIZE],
    #[cfg(feature = "bb_counter_stats")]
    pub loop_size_stats: [StepType; COUNTER_ARRAY_SIZE],
    #[cfg(feature = "bb_counter_stats")]
    pub loop_steps_stats: [StepType; COUNTER_ARRAY_SIZE],
    // HashMap for larger
    // pub hold_steps_long: HashMap<StepType, StepType>,
}

impl ResultDecider {
    /// Result with starting steps_max to avoid unnecessary updates on machine with max steps. \
    /// Use init_steps_max(n_states).
    pub fn new(n_states: usize, init_steps_max: StepType) -> Self {
        ResultDecider {
            n_states,
            num_turing_machines: generator::num_turing_machine_permutations_u64(n_states),
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            steps_max: StepMaxResult::new(init_steps_max, 0),
            ..Default::default()
        }
    }

    // providing known steps_max avoids some updates
    pub fn new_batch(batch_info: &ResultBatchInfo) -> Self {
        ResultDecider {
            n_states: batch_info.n_states,
            num_turing_machines: generator::num_turing_machine_permutations_u64(
                batch_info.n_states,
            ),
            steps_max: StepMaxResult::new(
                batch_info.steps_max,
                batch_info.limit_machines_max_steps,
            ),
            record_machines_undecided: batch_info.limit_machines_undecided,
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            ..Default::default()
        }
    }

    pub fn batch_info(&self) -> ResultBatchInfo {
        ResultBatchInfo {
            n_states: self.n_states,
            steps_max: self.steps_max(),
            limit_machines_max_steps: self.steps_max.record_machines_max_steps,
            limit_machines_undecided: self.record_machines_undecided,
        }
    }

    /// Set steps_max a bit higher to avoid saving a lot of machines with low steps
    pub fn init_steps_max(n_states: usize) -> u32 {
        match n_states {
            1 => 0,
            2 | 3 => 4,
            _ => 25,
        }
    }

    pub fn set_record_machines_max_steps(&mut self, limit: u16) {
        self.steps_max.set_record_machines_max_steps(limit);
    }

    pub fn set_record_machines_undecided(&mut self, limit: u32) {
        self.record_machines_undecided = limit;
        if limit == 0 {
            self.machines_undecided = None;
        }
    }

    /// Add one single result to these totals
    pub fn add(&mut self, machine: &Machine, status: &MachineStatus) {
        self.num_evaluated += 1;
        match status {
            MachineStatus::DecidedHolds(steps) => {
                self.num_hold += 1;
                #[cfg(feature = "bb_counter_stats")]
                {
                    if steps < COUNTER_ARRAY_SIZE as StepType {
                        self.hold_steps_stats[steps as usize] += 1;
                    } else {
                        self.hold_steps_stats[0] += 1;
                    }
                }
                // if *steps >= self.steps_max.steps_max {
                self.steps_max.add_steps(*steps, machine, status);
                // }
                // println!("{}, {}", machine, status)
            }
            MachineStatus::EliminatedPreDecider(reason) => match reason {
                PreDeciderReason::None => panic!("None must not happen."),
                PreDeciderReason::NotAllStatesUsed => {
                    self.pre_decider_count.num_not_all_states_used += 1
                }
                PreDeciderReason::NotExactlyOneHoldCondition => {
                    self.pre_decider_count.num_not_exactly_one_hold_condition += 1
                }
                PreDeciderReason::OnlyOneDirection => {
                    self.pre_decider_count.num_only_one_direction += 1
                }
                PreDeciderReason::SimpleStartLoop => {
                    self.pre_decider_count.num_simple_start_loop += 1
                }
                PreDeciderReason::StartRecursive => self.pre_decider_count.num_start_recursive += 1,
                PreDeciderReason::WritesOnlyZero => {
                    self.pre_decider_count.num_writes_only_zero += 1
                }
            },
            MachineStatus::DecidedHoldsDetail(_, _, _) => todo!(),
            MachineStatus::UndecidedFastTapeBoundReached => todo!(),
            // MachineStatus::DecidedHoldsOld(steps, _) => {
            //     self.num_hold += 1;
            //     #[cfg(feature = "bb_counter_stats")]
            //     {
            //         if steps < COUNTER_ARRAY_SIZE as StepType {
            //             self.hold_steps_stats[steps as usize] += 1;
            //         } else {
            //             self.hold_steps_stats[0] += 1;
            //         }
            //     }
            //     self.add_steps(*steps, machine, status);
            // }
            MachineStatus::DecidedEndless(endless_reason) => {
                // self.num_endless += 1;
                self.endless_count.add_endless_reason(endless_reason);
            }
            MachineStatus::Undecided(_, _, _) => {
                if self.record_machines_undecided > 0
                    && self.num_undecided < self.record_machines_undecided as u64
                {
                    if let Some(machines) = self.machines_undecided.as_mut() {
                        machines.push(MachineInfo::from_machine(machine, status));
                    } else {
                        self.machines_undecided =
                            Some(vec![MachineInfo::from_machine(machine, status)]);
                    }
                }
                self.num_undecided += 1;
            }
            MachineStatus::DecidedNotMaxTooManyHoldTransitions => {
                self.num_not_max_too_many_hold_transitions += 1
            }
            MachineStatus::DecidedNotMaxNotAllStatesUsed => {
                self.num_not_max_not_all_states_used += 1
            }
            MachineStatus::NoDecision => {
                panic!("State NoDecision must not be the final state. Change it to Undecided.")
            }
        }
    }

    pub fn add_result(&mut self, result: &ResultDecider) {
        self.num_evaluated += result.num_evaluated;
        self.num_hold += result.num_hold;
        // self.num_endless += result.num_endless;
        self.num_not_max += result.num_not_max;
        self.num_undecided += result.num_undecided;

        self.steps_max.add_self(&result.steps_max);

        self.pre_decider_count.add_self(&result.pre_decider_count);
        self.pre_decider_count.num_checked = self.pre_decider_count.total() + self.num_evaluated;
        self.endless_count.add_self(&result.endless_count);

        self.num_not_max_not_all_states_used += result.num_not_max_not_all_states_used;
        self.num_not_max_too_many_hold_transitions += result.num_not_max_too_many_hold_transitions;

        // add undecided machines
        if self.num_undecided < self.record_machines_undecided as u64 {
            if let Some(new_machines) = result.machines_undecided.as_ref() {
                if let Some(machines) = self.machines_undecided.as_mut() {
                    let max = new_machines
                        .len()
                        .min(self.record_machines_undecided as usize - machines.len());
                    machines.extend_from_slice(&new_machines[0..max]);
                } else {
                    self.machines_undecided = result.machines_undecided.clone();
                }
            }
        }
        // update array stats
        #[cfg(feature = "bb_counter_stats")]
        for i in 0..COUNTER_ARRAY_SIZE {
            self.hold_steps_stats[i] += result.hold_steps_stats[i];
            self.loop_size_stats[i] += result.loop_size_stats[i];
            self.loop_steps_stats[i] += result.loop_steps_stats[i];
        }
    }

    pub fn add_pre_decider_count(&mut self, count: &PreDeciderCount) {
        self.pre_decider_count.add_self(count);
    }

    pub fn steps_max(&self) -> StepType {
        self.steps_max.steps_max
    }

    /// Returns the first machine with max steps.
    pub fn machine_max_steps(&self) -> Option<MachineInfo> {
        self.steps_max.machine_max_steps()
    }

    /// Returns all recorded machines with max steps.
    pub fn machines_max_steps(&self) -> Option<&Vec<MachineInfo>> {
        self.steps_max.machines_max_steps()
    }

    // pub fn machines_max_steps_to_string(&self, max_machines: usize) -> String {
    //     if let Some(machines) = &self.machines_max_steps {
    //         let last = machines.len().min(max_machines);
    //         let mut s = String::new();
    //         for m in machines.iter().take(last) {
    //             s.push_str(format!("Hold {}\n", m).as_str());
    //         }
    //         s
    //     } else if self.num_machines_for_steps_max == 0 {
    //         "No machines found that holds!".to_string()
    //     } else {
    //         format!(
    //             "No max step machines recorded, but {} machines found!",
    //             self.num_machines_for_steps_max
    //         )
    //     }
    // }

    pub fn machines_max_steps_to_string(&self, return_max_machines: usize) -> String {
        self.steps_max
            .machines_max_steps_to_string(return_max_machines)
    }

    pub fn machines_undecided(&self) -> Option<&Vec<MachineInfo>> {
        self.machines_undecided.as_ref()
    }

    pub fn machines_undecided_to_string(&self, max_machines: usize) -> String {
        if let Some(machines) = &self.machines_undecided {
            let last = machines.len().min(max_machines);
            let mut s = String::new();
            for m in machines.iter().take(last) {
                s.push_str(
                    format!(
                        "Undecided M No. {:5} {}, Steps: {}\n",
                        m.id(),
                        m.to_standard_tm_text_format(),
                        m.steps()
                    )
                    .as_str(),
                );
            }
            s
        } else if self.num_undecided == 0 {
            "No undecided machines found!".to_string()
        } else {
            format!(
                "No undecided machines recorded, but {} machines found!",
                self.num_undecided
            )
        }
    }

    pub fn to_string_extended(&self) -> String {
        let mut s = String::new();
        // list some undecided machines
        if let Some(undecided) = self.machines_undecided.as_ref() {
            for u in undecided.iter().take(10) {
                s.push_str(format!("{}\n", u).as_str());
            }
        }

        // add normal result
        s.push_str(format!("\n{}", self).as_str());

        // if self.steps_max > 0 {
        //     s.push_str(format!("Most Steps:\n{}", self.machine_max_steps().unwrap()).as_str());
        // }

        let locale = user_locale();
        // let mut buf = Buffer::default();
        // buf.write_formatted(&self.num_turing_machines, &locale);

        s.push_str(format!(
            "\nBB{}: '{}' time elapsed for run with {} machines: generator {:?} ms, decider {:?} ms, total time {:?} ms.",
            self.n_states,
            self.name,
            self.num_evaluated.to_formatted_string(&locale),
            duration_as_ms_rounded(self.duration.duration_generator),
            duration_as_ms_rounded(self.duration.duration_decider),
            duration_as_ms_rounded(self.duration.duration_total),
        ).as_str());

        s
    }

    pub fn num_checked_total(&self) -> u64 {
        self.pre_decider_count.total() + self.num_evaluated
    }

    pub fn num_endless(&self) -> u64 {
        self.endless_count.num_endless_total()
    }
}

impl Display for ResultDecider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let locale = SystemLocale::default().unwrap();
        let locale = user_locale();
        let mut buf = Buffer::default();

        buf.write_formatted(&self.num_turing_machines, &locale);
        let mut s = format!("Turing machines:    {:>15}\n", buf.as_str());
        buf.write_formatted(&self.num_evaluated, &locale);
        s.push_str(format!("Total checked:      {:>15}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_undecided, &locale);
        s.push_str(format!("  Undecided:        {:>15}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_hold, &locale);
        s.push_str(format!("  Decided Holds:    {:>15}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_not_max_too_many_hold_transitions, &locale);
        // s.push_str(format!("  Two+ Hold Trans.: {:>15}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_not_max_not_all_states_used, &locale);
        // s.push_str(format!("  Not All States used:{:>13}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_endless, &locale);
        // s.push_str(format!("  Decided Endless:  {:>15}\n", buf.as_str()).as_str());
        s.push_str(format!("{}", self.endless_count).as_str());
        s.push_str(format!("{}", self.pre_decider_count).as_str());
        s.push_str(format!("{}", self.steps_max).as_str());
        #[cfg(feature = "bb_counter_stats")]
        {
            let steps: StepType = self.hold_steps_stats.iter().sum();
            s.push_str(
                format!(
                    "Hold: Steps till {}: {} {:?}",
                    COUNTER_ARRAY_SIZE, steps, self.hold_steps_stats
                )
                .as_str(),
            );
            s.push('\n');
            s.push_str(
                format!(
                    "Loop: Size till {}: {:?}",
                    COUNTER_ARRAY_SIZE, self.loop_size_stats
                )
                .as_str(),
            );
            s.push('\n');
            s.push_str(
                format!(
                    "Loop: Step detected {}: {:?}",
                    COUNTER_ARRAY_SIZE, self.loop_steps_stats
                )
                .as_str(),
            );
        }
        write!(f, "{}", s)
    }
}

pub struct ResultBatchInfo {
    pub n_states: usize,
    pub steps_max: StepType,
    pub limit_machines_max_steps: u16,
    pub limit_machines_undecided: u32,
}

#[derive(Debug, Default)]
pub struct EndlessCount {
    pub num_expanding_loop: u64,
    pub num_expanding_sinus: u64,
    pub num_only_one_direction: u64,
    /// Eliminated machines which cannot hold because they have no hold conditions.
    pub num_no_hold_transition: u64,
    /// Eliminated machines which cannot reach the maximum steps because they have two or more hold conditions.
    pub num_simple_start_loop: u64,
    pub num_start_recursive: u64,
    pub num_writes_only_zeros: u64,
    pub num_loop: u64,
    pub longest_loop: StepType,
    pub loop_detect_step_max: StepType,
}

impl EndlessCount {
    pub fn add_endless_reason(&mut self, endless_reason: &EndlessReason) {
        match endless_reason {
            EndlessReason::ExpandingLoop => self.num_expanding_loop += 1,
            EndlessReason::ExpandingSinus(_) => self.num_expanding_sinus += 1,
            EndlessReason::OnlyOneDirection => self.num_only_one_direction += 1,
            EndlessReason::NoHoldTransition => self.num_no_hold_transition += 1,
            EndlessReason::SimpleStartLoop => self.num_simple_start_loop += 1,
            EndlessReason::StartRecursive => self.num_start_recursive += 1,
            EndlessReason::WritesOnlyZero => self.num_writes_only_zeros += 1,
            EndlessReason::Loop(steps, loop_size) => {
                self.num_loop += 1;
                if *loop_size > self.longest_loop {
                    self.longest_loop = *loop_size;
                    // #[cfg(debug_assertions)]
                    // if *loop_size > COUNTER_ARRAY_SIZE as StepType {
                    //     println!("Loop Size {loop_size} Machine {}", machine)
                    // }
                }
                if *steps > self.loop_detect_step_max {
                    self.loop_detect_step_max = *steps;
                    // #[cfg(debug_assertions)]
                    // if *loop_size > COUNTER_ARRAY_SIZE as StepType {
                    //     println!("Loop detected steps {steps} Machine {}", machine)
                    // }
                }
                #[cfg(feature = "bb_counter_stats")]
                {
                    if loop_size < COUNTER_ARRAY_SIZE as StepType {
                        self.loop_size_stats[loop_size as usize] += 1;
                    } else {
                        self.loop_size_stats[0] += 1;
                    }
                    if steps < COUNTER_ARRAY_SIZE as StepType {
                        self.loop_steps_stats[steps as usize] += 1;
                    } else {
                        self.loop_steps_stats[0] += 1;
                    }
                }
            }
        }
    }

    fn add_self(&mut self, other: &Self) {
        self.num_expanding_loop += other.num_expanding_loop;
        self.num_expanding_sinus += other.num_expanding_sinus;
        self.num_only_one_direction += other.num_only_one_direction;
        self.num_no_hold_transition += other.num_no_hold_transition;
        self.num_simple_start_loop += other.num_simple_start_loop;
        self.num_start_recursive += other.num_start_recursive;
        self.num_writes_only_zeros += other.num_writes_only_zeros;
        self.num_loop += other.num_loop;
        self.longest_loop = other.longest_loop.max(self.longest_loop);
        self.loop_detect_step_max = other.loop_detect_step_max.max(self.loop_detect_step_max);
    }

    fn num_endless_total(&self) -> u64 {
        self.num_expanding_loop
            + self.num_expanding_sinus
            + self.num_only_one_direction
            + self.num_no_hold_transition
            + self.num_simple_start_loop
            + self.num_start_recursive
            + self.num_writes_only_zeros
            + self.num_loop
    }
}

impl Display for EndlessCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let locale = SystemLocale::default().unwrap();
        let locale = user_locale();
        let mut buf = Buffer::default();
        let mut s = String::new();

        buf.write_formatted(&self.num_endless_total(), &locale);
        s.push_str(format!("  Decided Endless:  {:>15}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_no_hold_transition, &locale);
        // s.push_str(format!("   No Hold Transition: {:>14}\n", buf.as_str()).as_str());
        // // s.push_str(
        // //     format!(
        // //         "   2+ Hold Transitions:{:10}\n",
        // //         self.num_not_max_too_many_hold_transitions
        // //     )
        // //     .as_str(),
        // // );
        // buf.write_formatted(&self.num_start_recursive, &locale);
        // s.push_str(format!("   Start Recursive:    {:>14}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_only_one_direction, &locale);
        // s.push_str(format!("   Only One Direction: {:>14}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_writes_only_zeros, &locale);
        // s.push_str(format!("   Writes Only Zero:   {:>14}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_expanding_sinus, &locale);
        // s.push_str(format!("   Expanding Sinus:    {:>14}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_expanding_loop, &locale);
        // s.push_str(format!("   Expanding Loop:     {:>14}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_simple_start_loop, &locale);
        // s.push_str(format!("   Simple Start Loop:  {:>14}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_loop, &locale);
        s.push_str(format!("   Loop:               {:>14}\n", buf.as_str()).as_str());
        s.push_str(format!("   - Longest Loop:     {:>14}\n", self.longest_loop).as_str());
        s.push_str(format!("   - Detect Step Max:  {:>14}\n", self.loop_detect_step_max).as_str());

        write!(f, "{}", s)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PreDeciderCount {
    // reference for percent calculation
    pub num_checked: u64,
    pub num_not_all_states_used: u64,
    pub num_not_exactly_one_hold_condition: u64,
    pub num_not_generated: u64,
    pub num_only_one_direction: u64,
    pub num_simple_start_loop: u64,
    pub num_start_recursive: u64,
    pub num_writes_only_zero: u64,
}

impl PreDeciderCount {
    pub fn add_self(&mut self, other: &Self) {
        self.num_not_all_states_used += other.num_not_all_states_used;
        self.num_not_exactly_one_hold_condition += other.num_not_exactly_one_hold_condition;
        self.num_not_generated += other.num_not_generated;
        self.num_only_one_direction += other.num_only_one_direction;
        self.num_simple_start_loop += other.num_simple_start_loop;
        self.num_start_recursive += other.num_start_recursive;
        self.num_writes_only_zero += other.num_writes_only_zero;
    }

    pub fn total(&self) -> u64 {
        self.num_not_all_states_used
            + self.num_not_exactly_one_hold_condition
            + self.num_not_generated
            + self.num_only_one_direction
            + self.num_simple_start_loop
            + self.num_start_recursive
            + self.num_writes_only_zero
    }
}

impl Display for PreDeciderCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let locale = SystemLocale::default().unwrap();
        let locale = user_locale();
        let mut buf = Buffer::default();
        let mut s = String::new();

        let total = self.total();
        buf.write_formatted(&total, &locale);
        s.push_str(format!("  Eliminated Pre-Decider: {:>15}", buf.as_str()).as_str());
        if total > 0 {
            if self.num_checked != 0 {
                let p = (total * 10000 / self.num_checked) as f64 / 100.0;
                s.push_str(format!(" ({p}%)").as_str());
            }
            buf.write_formatted(&self.num_not_generated, &locale);
            s.push_str(format!("\n   Not Generated:           {:>15}", buf.as_str()).as_str());
            if self.num_checked != 0 && self.num_not_generated > 0 {
                let p = (self.num_not_generated * 10000 / self.num_checked) as f64 / 100.0;
                s.push_str(format!(" ({p}%)").as_str());
            }
            buf.write_formatted(&self.num_not_exactly_one_hold_condition, &locale);
            s.push_str(format!("\n   Not One Hold Condition:   {:>14}\n", buf.as_str()).as_str());
            buf.write_formatted(&self.num_only_one_direction, &locale);
            s.push_str(format!("   Only One Direction:       {:>14}\n", buf.as_str()).as_str());
            buf.write_formatted(&self.num_writes_only_zero, &locale);
            s.push_str(format!("   Writes Only Zero:         {:>14}\n", buf.as_str()).as_str());
            buf.write_formatted(&self.num_not_all_states_used, &locale);
            s.push_str(format!("   Not All States Used:      {:>14}\n", buf.as_str()).as_str());
            buf.write_formatted(&self.num_simple_start_loop, &locale);
            s.push_str(format!("   Simple Start Loop:        {:>14}\n", buf.as_str()).as_str());
            buf.write_formatted(&self.num_start_recursive, &locale);
            s.push_str(format!("   Start Recursive:          {:>14}\n", buf.as_str()).as_str());
        } else {
            s.push('\n');
        }
        write!(f, "{}", s)
    }
}

/// Duration of the generator tasks.
#[derive(Debug, Default)]
pub struct DurationGenerator {
    pub duration_generator: Duration,
    /// Duration of the decider tasks.
    pub duration_decider: Duration,
    /// Duration total which includes the task creation and waiting time.
    pub duration_total: Duration,
}

#[derive(Debug, Default)]
pub struct StepMaxResult {
    pub steps_max: StepType,
    pub num_machines_steps_max: u16,
    machine_max_steps: Option<MachineInfo>,
    machines_max_steps: Option<Vec<MachineInfo>>,
    /// Store all machines with max steps up to this limit.
    record_machines_max_steps: u16,
}

impl StepMaxResult {
    pub fn new(steps_max_init: StepType, record_machines_max_steps: u16) -> Self {
        Self {
            steps_max: steps_max_init,
            record_machines_max_steps,
            ..Default::default()
        }
    }

    pub fn add_self(&mut self, other: &Self) {
        if other.steps_max >= self.steps_max {
            if other.steps_max == self.steps_max {
                self.num_machines_steps_max += other.num_machines_steps_max;
                if self.record_machines_max_steps > 0
                    && self.record_machines_max_steps as usize > self.len_machines_max_steps()
                {
                    let max_len =
                        self.record_machines_max_steps as usize - self.len_machines_max_steps();
                    if max_len > 0 {
                        if let Some(machines) = other.machines_max_steps.as_ref() {
                            if self.machines_max_steps.is_none() {
                                self.machines_max_steps = Some(machines.clone());
                            } else {
                                let end = machines.len().min(max_len);
                                self.machines_max_steps
                                    .as_mut()
                                    .unwrap()
                                    .extend_from_slice(&machines[0..end]);
                            }
                        }
                    }
                }
            } else {
                // new max
                self.steps_max = other.steps_max;
                self.num_machines_steps_max = other.num_machines_steps_max;
                if self.record_machines_max_steps == 0 {
                    if other.machine_max_steps.is_some() {
                        self.machine_max_steps = other.machine_max_steps;
                    }
                } else {
                    self.machines_max_steps = other.machines_max_steps.clone();
                }
            }
        }
    }

    fn add_steps(&mut self, steps: StepType, machine: &Machine, status: &MachineStatus) {
        // Check biggerThan to avoid two ifs on every check as it occurs rarely
        if steps >= self.steps_max {
            if steps == self.steps_max {
                // store additional max step machine only if requested
                if self.record_machines_max_steps > 0
                    && self.num_machines_steps_max < self.record_machines_max_steps
                {
                    if self.machines_max_steps.is_none() {
                        self.machines_max_steps = Some(Vec::with_capacity(4));
                    }
                    self.machines_max_steps
                        .as_mut()
                        .unwrap()
                        .push(MachineInfo::from_machine(machine, status));
                    // println!("  Added machine for max step {steps}");
                }
                self.num_machines_steps_max += 1;
            } else {
                // new max, clear the list of machines and add new first
                // if steps > 107 {
                //     println!("{}", machine);
                //     println!("{status}");
                //     let s = DeciderU128Long::<SubDeciderDummy>::run_decider(machine);
                //     let rf = machine.has_self_referencing_transition();
                //     println!("{s}");
                //     println!();
                // }
                self.steps_max = steps;
                self.num_machines_steps_max = 1;
                if self.record_machines_max_steps > 0 {
                    if self.machines_max_steps.is_none() {
                        self.machines_max_steps = Some(Vec::with_capacity(8));
                    } else {
                        self.machines_max_steps.as_mut().unwrap().clear();
                    }
                    self.machines_max_steps
                        .as_mut()
                        .unwrap()
                        .push(MachineInfo::from_machine(machine, status));
                    // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    // {
                    // println!("  New max steps {}", self.steps_max);
                    //     let p = Permutation::new(machine.id, machine.transitions);
                    //     println!("Transitions\n{}", &p);
                    // }
                } else {
                    self.machine_max_steps = Some(MachineInfo::from_machine(machine, status))
                }
            }
        }
    }

    /// Returns the first machine with max steps.
    pub fn machine_max_steps(&self) -> Option<MachineInfo> {
        if self.record_machines_max_steps == 0 {
            if let Some(m) = self.machine_max_steps.as_ref() {
                return Some(*m);
            }
        } else if let Some(machines) = self.machines_max_steps.as_ref() {
            return machines.first().cloned();
        };
        None
    }

    /// Returns all recorded machines with max steps.
    pub fn machines_max_steps(&self) -> Option<&Vec<MachineInfo>> {
        self.machines_max_steps.as_ref()
    }

    pub fn machines_max_steps_to_string(&self, return_max_machines: usize) -> String {
        if let Some(machines) = &self.machines_max_steps {
            let end = machines.len().min(return_max_machines);
            let mut s = String::new();
            for m in machines.iter().take(end) {
                s.push_str(
                    format!(
                        "Hold M No. {:5} {}, Steps: {}\n",
                        m.id(),
                        m.to_standard_tm_text_format(),
                        m.steps()
                    )
                    .as_str(),
                );
            }
            s
        } else if self.num_machines_steps_max == 0 {
            "No machines found that holds!".to_string()
        } else {
            format!(
                "No max step machines recorded, but {} machines found!",
                self.num_machines_steps_max
            )
        }
    }

    fn len_machines_max_steps(&self) -> usize {
        match self.machines_max_steps.as_ref() {
            Some(m) => m.len(),
            None => 0,
        }
    }

    pub fn set_record_machines_max_steps(&mut self, limit: u16) {
        if limit <= 1 {
            self.machines_max_steps = None;
            self.record_machines_max_steps = 0;
        } else {
            self.record_machines_max_steps = limit;
        }
    }

    pub fn sort_machines(&mut self) {
        if let Some(v) = self.machines_max_steps.as_mut() {
            // v.sort_by(|a, b| a.id().cmp(&b.id()));
            v.sort();
        }
    }
}

impl Display for StepMaxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let locale = SystemLocale::default().unwrap();
        let locale = user_locale();
        // let mut buf = Buffer::default();
        // let mut s = String::new();

        // buf.write_formatted(&self.steps_max, &locale);
        // s.push_str(
        //     format!(
        //         "  Max Steps:           {:>14} (Number of machines: {})\n",
        //         buf.as_str(),
        //         self.num_machines_steps_max
        //     )
        //     .as_str(),
        // );
        // if let Some(machine) = self.machine_max_steps() {
        //     s.push_str(format!("Most Steps: {}\n", machine).as_str());
        // }

        write!(
            f,
            "  Max Steps:           {:>14} (Number of machines: {})\n",
            self.steps_max.to_formatted_string(&locale),
            self.num_machines_steps_max,
        )?;
        // print max 4 max step machines
        if self.num_machines_steps_max > 1 && self.machines_max_steps.is_some() {
            // get first 4 machines, but need to sort first for batches may come in other order
            let mut v = self.machines_max_steps.as_ref().unwrap().to_vec();
            v.sort();
            // format right aligned
            let len = v.last().unwrap().id().to_formatted_string(&locale).len();
            dbg!(len);
            // let v = self.machines_max_steps.as_ref().unwrap();
            for m in v.iter().take(4) {
                write!(
                    f,
                    "   Machine No. {:>len$}: {}\n",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format()
                )?;
            }
        } else {
            if let Some(m) = self.machine_max_steps() {
                write!(
                    f,
                    "   Machine No. {}: {}\n",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format()
                )?;
            };
        }
        Ok(())
    }
}

pub fn result_max_steps_known(n_states: usize) -> StepType {
    match n_states {
        1 => 1,
        2 => 6,
        3 => 21,
        4 => 107,
        5 => 47_176_870,
        _ => panic!("result_max_steps: Not build for this."),
    }
}
