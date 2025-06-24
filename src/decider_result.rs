use num_format::{Buffer, ToFormattedString};
use std::{fmt::Display, time::Duration};

#[cfg(feature = "bb_counter_stats")]
use crate::status::COUNTER_ARRAY_SIZE;
use crate::{
    config::Config,
    generator,
    machine::Machine,
    machine_info::MachineInfo,
    status::{EndlessReason, MachineStatus, PreDeciderReason},
    utils::{duration_as_ms_rounded, user_locale},
    StepType,
};

const NUM_LONG_LEN: usize = 18;
const NUM_SHORT_LEN: usize = 14;
const LEVEL_1_CHAR: char = '\u{2022}';
const NUM_MAX_MACHINES_TO_DISPLAY_IN_RESULT: usize = 10;
const NUM_UNDECIDED_MACHINES_TO_DISPLAY_IN_RESULT: usize = 10;

// TODO result print undecided

#[non_exhaustive]
#[derive(Debug, Default, Clone, PartialEq)]
// TODO allow error?
pub enum EndReason {
    AllMachinesChecked,
    Error(String),
    // This is a temporary state indicating the last batch needs to be evaluated, but gives a stop indication.
    IsLastBatch,
    MachineLimitReached(u64),
    // This is usually an unexpected end when the total is not reached.
    NoMoreData,
    StopRequested(String),
    UndecidedLimitReached(usize),
    #[default]
    Undefined,
    // not ended yet
    Working,
}

impl Display for EndReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndReason::AllMachinesChecked => write!(f, "All machines checked"),
            EndReason::Error(message) => write!(f, "Error: {message}"),
            EndReason::IsLastBatch => write!(f, "Last batch indication. Should be internal only"),
            EndReason::MachineLimitReached(limit) => {
                write!(f, "Limit of {} machines reached", limit)
            }
            EndReason::NoMoreData => write!(f, "No more data found"),
            EndReason::StopRequested(message) => write!(f, "Stop requested: {message}"),
            EndReason::UndecidedLimitReached(limit) => {
                write!(f, "Limit of {} undecided machines reached", limit)
            }
            EndReason::Undefined => write!(f, "No end reason given"),
            EndReason::Working => write!(f, "working..."),
        }
        // write(f, "{s}")
    }
}

/// The result of the decider. It holds a number of counters for each result type and may carry the
/// max steps and undecided machines.
#[derive(Debug, Default)]
pub struct DeciderResultStats {
    /// Number of machines which have been tested by the deciders (not pre-deciders) or have been eliminated during
    /// generation. This needs to be the num_turing_machines if not limited.
    num_checked_total: u64,
    /// Number of machines which have run through deciders. This included the Pre-Decider if not eliminated in the generator.
    num_evaluated: u64,
    /// Tested machines which come to a hold.
    /// This does not include machines which have not been tested, e.g. because they cannot produce the maximal steps.
    num_hold: u64,
    /// Eliminated machines which cannot reach the maximum steps (may or may not hold).
    num_not_max: u64,
    /// Tested machines which did not come to a result.
    num_undecided: u64,
    /// Breakdown of eliminated machines
    pre_decider_count: PreDeciderCount,
    /// Breakdown of endless running machines
    endless_count: EndlessCount,

    /// Number of states used for the Turing machines.
    n_states: usize,
    /// Number of possible Turing machines (max seven states for u64 size limit).
    num_turing_machines: u64,

    num_not_max_too_many_hold_transitions: u64,
    /// Eliminated machines which cannot reach the maximum steps because not all states were used.
    num_not_max_not_all_states_used: u64,

    // steps
    steps_max: StepMaxResult,
    // pub steps_max: StepType,
    // pub num_machines_for_steps_max: u16,
    // machine_max_steps: Option<MachineInfo>,
    /// Store all machines with max steps up to this limit.
    // record_machines_max_steps: u16,
    // machines_max_steps: Option<Vec<MachineInfo>>,
    /// Store all machines Undecided up to this limit.
    limit_machines_undecided: usize,
    // machine_undecided: Option<MachineInfo>,
    machines_undecided: Option<Vec<MachineInfo>>,
    pub end_reason: EndReason,

    // for statistical purposes and performance tests
    pub duration: DurationDataProvider,
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

impl DeciderResultStats {
    /// Result with starting steps_max to avoid unnecessary updates on machine with max steps. \
    /// Use init_steps_max(n_states).
    pub fn new(config: &Config) -> Self {
        DeciderResultStats {
            n_states: config.n_states(),
            num_turing_machines: generator::num_turing_machine_permutations_u64(config.n_states()),
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            steps_max: StepMaxResult::new(
                config.init_steps_max(),
                config.limit_machines_max_steps(),
            ),
            limit_machines_undecided: config.limit_machines_undecided(),
            ..Default::default()
        }
    }

    pub fn new_init_steps_max(init_steps_max: StepType, config: &Config) -> Self {
        DeciderResultStats {
            n_states: config.n_states(),
            num_turing_machines: generator::num_turing_machine_permutations_u64(config.n_states()),
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            steps_max: StepMaxResult::new(init_steps_max, config.limit_machines_max_steps()),
            limit_machines_undecided: config.limit_machines_undecided(),
            ..Default::default()
        }
    }

    /// Result with starting steps_max to avoid unnecessary updates on machine with max steps. \
    /// Use init_steps_max(n_states).
    pub fn new_deprecated(n_states: usize, init_steps_max: StepType) -> Self {
        DeciderResultStats {
            n_states,
            num_turing_machines: generator::num_turing_machine_permutations_u64(n_states),
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            steps_max: StepMaxResult::new(init_steps_max, 0),
            ..Default::default()
        }
    }

    #[deprecated(note = "Functions should rely on Config")]
    // providing known steps_max avoids some updates
    pub(crate) fn new_batch_deprecated(batch_info: &ResultBatchInfo) -> Self {
        DeciderResultStats {
            n_states: batch_info.n_states,
            num_turing_machines: generator::num_turing_machine_permutations_u64(
                batch_info.n_states,
            ),
            steps_max: StepMaxResult::new(
                batch_info.steps_max_init,
                batch_info.limit_machines_max_steps,
            ),
            limit_machines_undecided: batch_info.limit_machines_undecided,
            // #[cfg(feature = "bb_use_result_large")]
            // is_result_large: true,
            ..Default::default()
        }
    }

    #[deprecated(note = "Functions should rely on Config")]
    pub(crate) fn batch_info(&self) -> ResultBatchInfo {
        ResultBatchInfo {
            n_states: self.n_states,
            steps_max_init: self.steps_max(),
            limit_machines_max_steps: self.steps_max.limit_machines_max_steps,
            limit_machines_undecided: self.limit_machines_undecided,
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

    pub fn set_record_machines_max_steps(&mut self, limit: usize) {
        self.steps_max.set_record_machines_max_steps(limit);
    }

    pub fn limit_machines_undecided(&self) -> usize {
        self.limit_machines_undecided
    }

    pub fn set_limit_machines_undecided(&mut self, limit: usize) {
        self.limit_machines_undecided = limit;
        if limit == 0 {
            self.machines_undecided = None;
        }
    }

    /// Add one single result to these totals
    /// Returns false if <limit_machines_undecided> Undecided Machines have been stored
    /// which allows the caller to stop further processing.  
    pub fn add(&mut self, machine: &Machine, status: &MachineStatus) -> bool {
        // self.num_checked_total += 1;
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
                PreDeciderReason::StartStateBandRight => {
                    self.pre_decider_count.num_start_state_b_and_right += 1
                }
                PreDeciderReason::WritesOnlyZero => {
                    self.pre_decider_count.num_writes_only_zero += 1
                }
            },
            MachineStatus::DecidedHoldsDetail(_, _, _) => todo!(),
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
                if self.limit_machines_undecided > 0 {
                    if self.num_undecided < self.limit_machines_undecided as u64 {
                        if let Some(machines) = self.machines_undecided.as_mut() {
                            machines.push(MachineInfo::from_machine(machine, status));
                        } else {
                            self.machines_undecided =
                                Some(vec![MachineInfo::from_machine(machine, status)]);
                        }
                    } else {
                        return false;
                    }
                }
                self.num_undecided += 1;
            }
            MachineStatus::DecidedNotMaxTooManyHoldTransitions => {
                self.num_not_max_too_many_hold_transitions += 1;
            }
            MachineStatus::DecidedNotMaxNotAllStatesUsed => {
                self.num_not_max_not_all_states_used += 1;
            }
            MachineStatus::NoDecision => {
                panic!("State NoDecision must not be the final state. Change it to Undecided.");
            }
        }

        true
    }

    /// Add another result to this result. \
    /// Returns false if <limit_machines_undecided> Undecided Machines have been stored
    /// which allows the caller to stop further processing.  
    pub fn add_result(&mut self, result: &DeciderResultStats) -> bool {
        self.num_checked_total += result.num_checked_total;
        self.num_evaluated += result.num_evaluated;
        self.num_hold += result.num_hold;
        // self.num_endless += result.num_endless;
        self.num_not_max += result.num_not_max;
        self.num_undecided += result.num_undecided;

        self.steps_max.add_self(&result.steps_max);

        self.pre_decider_count.add_self(&result.pre_decider_count);
        // self.pre_decider_count.num_checked = self.pre_decider_count.total() + self.num_evaluated;
        self.endless_count.add_self(&result.endless_count);

        self.num_not_max_not_all_states_used += result.num_not_max_not_all_states_used;
        self.num_not_max_too_many_hold_transitions += result.num_not_max_too_many_hold_transitions;

        // update array stats
        #[cfg(feature = "bb_counter_stats")]
        for i in 0..COUNTER_ARRAY_SIZE {
            self.hold_steps_stats[i] += result.hold_steps_stats[i];
            self.loop_size_stats[i] += result.loop_size_stats[i];
            self.loop_steps_stats[i] += result.loop_steps_stats[i];
        }

        // add undecided machines
        if self.limit_machines_undecided > 0 {
            if self.num_undecided < self.limit_machines_undecided as u64 {
                if let Some(new_machines) = result.machines_undecided.as_ref() {
                    if let Some(machines) = self.machines_undecided.as_mut() {
                        let max = new_machines
                            .len()
                            .min(self.limit_machines_undecided as usize - machines.len());
                        machines.extend_from_slice(&new_machines[0..max]);
                    } else {
                        self.machines_undecided = result.machines_undecided.clone();
                    }
                    if self.num_undecided >= self.limit_machines_undecided as u64 {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        true
    }

    pub fn add_pre_decider_count(&mut self, count: &PreDeciderCount) {
        self.pre_decider_count.add_self(count);
    }

    pub fn add_total(&mut self, value: u64) {
        self.num_checked_total += value;
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

    /// Returns all recorded machines with max steps, sorted by id.
    pub fn machines_max_steps_sorted(&self) -> Option<Vec<MachineInfo>> {
        self.steps_max.machines_max_steps_sorted()
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

    /// Returns all recorded machines with max steps, sorted by id.
    pub fn machines_undecided_sorted(&self) -> Option<Vec<MachineInfo>> {
        if let Some(machines) = self.machines_undecided.as_ref() {
            let mut v = machines.to_vec();
            v.sort();
            Some(v)
        } else {
            None
        }
    }

    // TODO move undecided in own struct and replace this with Display. Merge from result Display.
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

    pub fn to_string_with_duration(&self) -> String {
        // normal result
        format!(
            "{}\nBB{}: '{}' time elapsed for run with {} machines: generator {:?} ms, decider {:?} ms, total time {:?} ms.",
            self,
            self.n_states,
            self.name,
            self.num_evaluated.to_formatted_string(&user_locale()),
            duration_as_ms_rounded(self.duration.duration_data_provider),
            duration_as_ms_rounded(self.duration.duration_decider),
            duration_as_ms_rounded(self.duration.duration_total),
        )
    }

    pub fn num_checked_total(&self) -> u64 {
        if self.num_checked_total != 0 {
            self.num_checked_total
        } else {
            self.num_evaluated
        }
        // self.pre_decider_count.total() + self.num_evaluated
    }

    pub fn num_endless(&self) -> u64 {
        self.endless_count.num_endless_total()
    }

    pub fn num_evaluated(&self) -> u64 {
        self.num_evaluated
    }

    pub fn num_hold(&self) -> u64 {
        self.num_hold
    }

    pub fn num_not_max(&self) -> u64 {
        self.num_not_max
    }

    pub fn num_undecided(&self) -> u64 {
        self.num_undecided
    }

    pub fn num_undecided_free(&self) -> usize {
        if self.limit_machines_undecided == 0
            || self.num_undecided >= self.limit_machines_undecided as u64
        {
            0
        } else {
            self.limit_machines_undecided - self.num_undecided as usize
        }
    }

    pub fn pre_decider_count(&self) -> PreDeciderCount {
        self.pre_decider_count
    }

    pub fn endless_count(&self) -> &EndlessCount {
        &self.endless_count
    }

    pub fn n_states(&self) -> usize {
        self.n_states
    }

    pub fn num_turing_machines(&self) -> u64 {
        self.num_turing_machines
    }

    pub fn num_not_max_too_many_hold_transitions(&self) -> u64 {
        self.num_not_max_too_many_hold_transitions
    }
}

impl Display for DeciderResultStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // update predecider
        // self.pre_decider_count.num_checked = self.pre_decider_count.total() + self.num_evaluated;

        let locale = user_locale();
        let mut buf = Buffer::default();
        // TODO Could be replaced with write()?
        let mut s = String::new();

        writeln!(f, "Result: {}", self.end_reason)?;
        buf.write_formatted(&self.num_turing_machines, &locale);
        s.push_str(format!("Turing machines:    {:>NUM_LONG_LEN$}\n", buf.as_str()).as_str());
        if self.num_checked_total() != self.num_evaluated {
            buf.write_formatted(&self.num_checked_total, &locale);
            s.push_str(format!("Total checked:      {:>NUM_LONG_LEN$}\n", buf.as_str()).as_str());
        }
        buf.write_formatted(&self.num_evaluated, &locale);
        s.push_str(format!("  Total evaluated:    {:>NUM_LONG_LEN$}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_undecided, &locale);
        s.push_str(
            format!(
                "  {LEVEL_1_CHAR} Undecided:        {:>NUM_LONG_LEN$}\n",
                buf.as_str()
            )
            .as_str(),
        );
        buf.write_formatted(&self.num_hold, &locale);
        s.push_str(
            format!(
                "  {LEVEL_1_CHAR} Decided Holds:    {:>NUM_LONG_LEN$}\n",
                buf.as_str()
            )
            .as_str(),
        );
        // buf.write_formatted(&self.num_not_max_too_many_hold_transitions, &locale);
        // s.push_str(format!("  Two+ Hold Trans.: {:>NUM_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_not_max_not_all_states_used, &locale);
        // s.push_str(format!("  Not All States used:{:>13}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_endless, &locale);
        // s.push_str(format!("  Decided Endless:  {:>NUM_LEN$}\n", buf.as_str()).as_str());
        s.push_str(format!("{}", self.endless_count).as_str());
        s.push_str(format!("{}", self.pre_decider_count).as_str());
        s.push_str(format!("{}", self.steps_max).as_str());
        write!(f, "{}", s)?;

        if let Some(machines) = self.machines_undecided.as_ref() {
            writeln!(
                f,
                "  Undecided:             (Number of machines: {})",
                self.num_undecided,
            )?;
            // print first undecided machines
            // get first machines, but need to sort first for batches may come in other order
            let mut v = machines.to_vec();
            v.sort();
            // format right aligned
            let len = v.last().unwrap().id().to_formatted_string(&locale).len();
            for m in v.iter().take(NUM_UNDECIDED_MACHINES_TO_DISPLAY_IN_RESULT) {
                writeln!(
                    f,
                    "   Machine No. {:>len$}: {}, {}",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format(),
                    m.status()
                )?;
            }
        };

        // counter stats if enabled
        // TODO test enabled
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
        Ok(())
    }
}

pub struct ResultBatchInfo {
    pub n_states: usize,
    pub steps_max_init: StepType,
    pub limit_machines_max_steps: usize,
    pub limit_machines_undecided: usize,
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
        s.push_str(
            format!(
                "  {LEVEL_1_CHAR} Decided Endless:  {:>NUM_LONG_LEN$}\n",
                buf.as_str()
            )
            .as_str(),
        );
        // buf.write_formatted(&self.num_no_hold_transition, &locale);
        // s.push_str(format!("   No Hold Transition: {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // // s.push_str(
        // //     format!(
        // //         "   2+ Hold Transitions:{:10}\n",
        // //         self.num_not_max_too_many_hold_transitions
        // //     )
        // //     .as_str(),
        // // );
        // buf.write_formatted(&self.num_start_recursive, &locale);
        // s.push_str(format!("   Start Recursive:    {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_only_one_direction, &locale);
        // s.push_str(format!("   Only One Direction: {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_writes_only_zeros, &locale);
        // s.push_str(format!("   Writes Only Zero:   {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_expanding_sinus, &locale);
        // s.push_str(format!("   Expanding Sinus:    {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_expanding_loop, &locale);
        // s.push_str(format!("   Expanding Loop:     {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        // buf.write_formatted(&self.num_simple_start_loop, &locale);
        // s.push_str(format!("   Simple Start Loop:  {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_loop, &locale);
        s.push_str(
            format!(
                "     Loop:                  {:>NUM_SHORT_LEN$}\n",
                buf.as_str()
            )
            .as_str(),
        );
        s.push_str(
            format!(
                "     - Longest Loop:        {:>NUM_SHORT_LEN$}\n",
                self.longest_loop
            )
            .as_str(),
        );
        s.push_str(
            format!(
                "     - Detect Step Max:     {:>NUM_SHORT_LEN$}\n",
                self.loop_detect_step_max
            )
            .as_str(),
        );

        write!(f, "{}", s)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PreDeciderCount {
    // reference for percent calculation, holds the total number checked (not only pre-decider)
    pub num_checked_total_for_display: u64,
    pub num_not_all_states_used: u64,
    pub num_not_exactly_one_hold_condition: u64,
    pub num_not_generated: u64,
    pub num_only_one_direction: u64,
    pub num_simple_start_loop: u64,
    pub num_start_state_b_and_right: u64,
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
        self.num_start_state_b_and_right += other.num_start_state_b_and_right;
        self.num_writes_only_zero += other.num_writes_only_zero;
    }

    pub fn num_total(&self) -> u64 {
        self.num_not_all_states_used
            + self.num_not_exactly_one_hold_condition
            + self.num_not_generated
            + self.num_only_one_direction
            + self.num_simple_start_loop
            + self.num_start_state_b_and_right
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

        let total = self.num_total();
        buf.write_formatted(&total, &locale);
        s.push_str(
            format!(
                "  {LEVEL_1_CHAR} Eliminated Pre-Decider: {:>NUM_LONG_LEN$}",
                buf.as_str()
            )
            .as_str(),
        );
        if total > 0 {
            if self.num_checked_total_for_display != 0 {
                let p =
                    ((total * 10000 / self.num_checked_total_for_display) as f64).round() / 100.0;
                s.push_str(format!("   ({p:.1}%)").as_str());
            }
            s.push('\n');
            if self.num_not_generated != 0 {
                buf.write_formatted(&self.num_not_generated, &locale);
                s.push_str(
                    format!(
                        "    - Not Generated:           {:>NUM_LONG_LEN$}",
                        buf.as_str()
                    )
                    .as_str(),
                );
                if self.num_checked_total_for_display != 0 {
                    let p = ((self.num_not_generated * 10000 / self.num_checked_total_for_display)
                        as f64)
                        .round()
                        / 100.0;
                    s.push_str(format!(" ({p:.1}%)").as_str());
                }
                s.push('\n');
            }
            if self.num_start_state_b_and_right > 0 {
                buf.write_formatted(&self.num_start_state_b_and_right, &locale);
                s.push_str(
                    format!(
                        "    - Start must be 0RB or 1RB:    {:>NUM_SHORT_LEN$}\n",
                        buf.as_str()
                    )
                    .as_str(),
                );
            }
            buf.write_formatted(&self.num_not_exactly_one_hold_condition, &locale);
            s.push_str(
                format!(
                    "    - Not One Hold Condition:      {:>NUM_SHORT_LEN$}\n",
                    buf.as_str()
                )
                .as_str(),
            );
            buf.write_formatted(&self.num_only_one_direction, &locale);
            s.push_str(
                format!(
                    "    - Only One Direction:          {:>NUM_SHORT_LEN$}\n",
                    buf.as_str()
                )
                .as_str(),
            );
            buf.write_formatted(&self.num_writes_only_zero, &locale);
            s.push_str(
                format!(
                    "    - Writes Only Zero:            {:>NUM_SHORT_LEN$}\n",
                    buf.as_str()
                )
                .as_str(),
            );
            buf.write_formatted(&self.num_not_all_states_used, &locale);
            s.push_str(
                format!(
                    "    - Not All States Used:         {:>NUM_SHORT_LEN$}\n",
                    buf.as_str()
                )
                .as_str(),
            );
            buf.write_formatted(&self.num_simple_start_loop, &locale);
            s.push_str(
                format!(
                    "    - Simple Start Loop:           {:>NUM_SHORT_LEN$}\n",
                    buf.as_str()
                )
                .as_str(),
            );
            if self.num_start_recursive != 0 {
                buf.write_formatted(&self.num_start_recursive, &locale);
                s.push_str(
                    format!(
                        "    - Start Recursive:             {:>NUM_SHORT_LEN$}\n",
                        buf.as_str()
                    )
                    .as_str(),
                );
            }
        } else {
            s.push('\n');
        }
        write!(f, "{}", s)
    }
}

/// Duration of the generator tasks.
#[derive(Debug, Default)]
pub struct DurationDataProvider {
    pub duration_data_provider: Duration,
    /// Duration of the decider tasks.
    pub duration_decider: Duration,
    /// Duration total which includes the task creation and waiting time.
    pub duration_total: Duration,
}

#[derive(Debug, Default)]
pub struct StepMaxResult {
    pub steps_max: StepType,
    pub num_machines_steps_max: usize,
    machine_max_steps: Option<MachineInfo>,
    machines_max_steps: Option<Vec<MachineInfo>>,
    /// Store all machines with max steps up to this limit.
    limit_machines_max_steps: usize,
}

impl StepMaxResult {
    pub fn new(steps_max_init: StepType, limit_machines_max_steps: usize) -> Self {
        Self {
            // // Machines with only one step are not recorded, there are too many
            // steps_max: if steps_max_init > 2 {
            //     steps_max_init
            // } else {
            //     2
            // },
            steps_max: steps_max_init,
            limit_machines_max_steps,
            ..Default::default()
        }
    }

    pub fn add_self(&mut self, other: &Self) {
        if other.steps_max >= self.steps_max {
            if other.steps_max == self.steps_max {
                self.num_machines_steps_max += other.num_machines_steps_max;
                if self.limit_machines_max_steps > 0
                    && self.limit_machines_max_steps as usize > self.len_machines_max_steps()
                {
                    let max_len =
                        self.limit_machines_max_steps as usize - self.len_machines_max_steps();
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
                if self.limit_machines_max_steps == 0 {
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
                if self.limit_machines_max_steps > 0
                    && self.num_machines_steps_max < self.limit_machines_max_steps
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
                if self.limit_machines_max_steps > 0 {
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
        if self.limit_machines_max_steps == 0 {
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

    /// Returns all recorded machines with max steps, sorted by id.
    pub fn machines_max_steps_sorted(&self) -> Option<Vec<MachineInfo>> {
        if let Some(machines) = self.machines_max_steps.as_ref() {
            let mut v = machines.to_vec();
            v.sort();
            Some(v)
        } else {
            None
        }
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

    pub fn set_record_machines_max_steps(&mut self, limit: usize) {
        if limit <= 1 {
            self.machines_max_steps = None;
            self.limit_machines_max_steps = 0;
        } else {
            self.limit_machines_max_steps = limit;
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
        let locale = user_locale();
        write!(
            f,
            "  Max Steps:      {:>10} (Number of machines: {})\n",
            self.steps_max.to_formatted_string(&locale),
            self.num_machines_steps_max,
        )?;
        // print first max step machines
        if self.num_machines_steps_max == 1 {
            if let Some(m) = self.machine_max_steps() {
                write!(
                    f,
                    "   Machine No. {}: {}\n",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format()
                )?;
            };
            return Ok(());
        }
        if let Some(machines) = self.machines_max_steps_sorted() {
            // format right aligned
            let len = machines
                .last()
                .unwrap()
                .id()
                .to_formatted_string(&locale)
                .len();
            for m in machines.iter().take(NUM_MAX_MACHINES_TO_DISPLAY_IN_RESULT) {
                write!(
                    f,
                    "   Machine No. {:>len$}: {}\n",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format()
                )?;
            }
        }
        Ok(())
    }
}

pub struct MachinesUndecided {
    /// All undecided machines of one batch run. \
    /// Machines can be used directly in next batch run with undecided only.
    pub machines: Vec<Machine>,
    /// The detailed MachineStatus which holds the UndecidedReason. State corresponds with the machine with the same index.
    pub states: Vec<MachineStatus>,
}

impl MachinesUndecided {
    pub fn new(capacity: usize) -> Self {
        Self {
            machines: Vec::with_capacity(capacity),
            states: Vec::with_capacity(capacity),
        }
    }

    /// Converts the data to a vector of MachineInfo, which contains the machine data in a single struct.
    pub fn to_machine_info(&self) -> Vec<MachineInfo> {
        let mut infos = Vec::new();
        for (i, m) in self.machines.iter().enumerate() {
            infos.push(MachineInfo::new(
                m.id(),
                *m.transition_table(),
                self.states[i],
            ));
        }

        infos
    }
}

/// Result of a batch run with results for all machines in the batch.
/// All undecided Turing machines are recorded in detail.
pub struct BatchResult {
    pub batch_no: usize,
    pub num_batches: usize,
    pub result_decided: DeciderResultStats,
    pub machines_undecided: MachinesUndecided,
    pub decider_name: String,
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
