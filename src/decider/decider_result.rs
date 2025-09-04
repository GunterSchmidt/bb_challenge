use num_format::{Buffer, ToFormattedString};
use std::{fmt::Display, time::Duration};

use crate::{
    config::{user_locale, Config, IdNormalized, StepTypeBig, StepTypeSmall},
    data_provider::enumerator::num_turing_machine_permutations,
    decider::{pre_decider::PreDeciderRun, DeciderId},
    machine_binary::MachineBinary,
    machine_info::MachineInfo,
    reporter::format_duration_hhmmss_ms,
    status::{MachineStatus, NonHaltReason, PreDeciderReason},
};

const NUM_LONG_LEN: usize = 18;
const NUM_SHORT_LEN: usize = 14;
const LEVEL_1_CHAR: char = '\u{2022}';
const NUM_MAX_MACHINES_TO_DISPLAY_IN_RESULT: usize = 10;
const NUM_UNDECIDED_MACHINES_TO_DISPLAY_IN_RESULT: usize = 10;

pub type ResultDeciderStats = std::result::Result<DeciderResultStats, String>;
pub type ResultUnitEndReason = Result<(), EndReason>;

// TODO result print undecided

#[non_exhaustive]
#[derive(Debug, Default, Clone, PartialEq)]
// TODO allow error?
pub enum EndReason {
    /// Final end reason of the decider(s).
    AllMachinesChecked,
    /// Error Machine Id, msg
    // TODO Option<MachineInfo>
    Error(u64, String),
    /// The data provider needs to mark the last batch so the caller knows it can end requesting batches.
    IsLastBatch,
    MachineLimitReached(u64),
    /// A legit result of the data provider, e.g. when all machines have been pre-decided and none are to decide.
    NoBatchData,
    /// Usually an unexpected end when the total is not reached.
    NoMoreData,
    /// Machine Id, msg. ResultWorker can use this to end processing without marking it as an error.
    StopRequested(u64, String),
    /// When the maximum number of recorded undecided machines is reached. For analyzing undecided.
    RecordLimitDecidedReached(usize),
    /// When the maximum number of recorded undecided machines is reached. For analyzing undecided.
    RecordLimitUndecidedReached(usize),
    /// Default state indicating no action has been taken yet.
    #[default]
    None,
    // / The data provider needs to have this state when working.
    // Working,
}

// Implement std::convert::From for AppError; from io::Error
impl From<std::io::Error> for EndReason {
    fn from(error: std::io::Error) -> Self {
        Self::Error(0, error.to_string())
    }
}

impl Display for EndReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndReason::AllMachinesChecked => write!(f, "All machines checked"),
            EndReason::Error(m_id, message) => {
                let ms = if *m_id != 0 {
                    format!("Machine Id: {m_id}, ")
                } else {
                    String::new()
                };
                write!(f, "{ms}Error: {message}")
            }
            EndReason::IsLastBatch => {
                write!(f, "Last batch indication. Should be internal only")
            }
            EndReason::MachineLimitReached(limit) => {
                write!(f, "Limit of {limit} machines reached")
            }
            EndReason::NoBatchData => write!(f, "No data in this batch"),
            EndReason::NoMoreData => write!(f, "No more data found"),
            EndReason::StopRequested(m_id, message) => {
                let ms = if *m_id != 0 {
                    format!("Machine Id: {m_id}, ")
                } else {
                    String::new()
                };
                write!(f, "{ms}Stop requested: {message}")
            }
            EndReason::RecordLimitDecidedReached(limit) => {
                write!(f, "Limit ({limit}) for recording decided machines reached")
            }
            EndReason::RecordLimitUndecidedReached(limit) => {
                write!(
                    f,
                    "Limit ({limit}) for recording undecided machines reached"
                )
            }
            EndReason::None => write!(f, "No end reason"),
        }
        // write(f, "{s}")
    }
}

/// The result of the decider. It holds a number of counters for each result type and may carry the
/// max steps and undecided machines.
/// This is always returned. end_reason should give error information if any.
// TODO list of deciders with id, name, config, runtime, evaluated and decided (= and undecided)
#[derive(Debug, Default)]
pub struct DeciderResultStats {
    /// Number of machines which have been tested by the deciders (not pre-deciders) or have been eliminated during
    /// enumeration. This needs to be the num_turing_machines if not limited.
    num_processed_total: u64,
    /// Number of machines which have run through deciders. This included the Pre-Decider if not eliminated in the enumerator.
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
    /// Number of Turing machines to decide.
    // num_total_turing_machines: IdBig,
    num_not_max_too_many_hold_transitions: u64,
    /// Eliminated machines which cannot reach the maximum steps because not all states were used.
    num_not_max_not_all_states_used: u64,

    // steps
    steps_max: StepMaxResult,
    // pub steps_max: StepTypeBig,
    // pub num_machines_for_steps_max: u16,
    // machine_max_steps: Option<MachineInfo>,
    /// Store all machines with max steps up to this limit.
    // record_machines_max_steps: u16,
    // machines_max_steps: Option<Vec<MachineInfo>>,
    /// Store all machines Undecided up to this limit.
    limit_machines_decided: usize,
    limit_machines_undecided: usize,
    // machine_undecided: Option<MachineInfo>,
    machines_decided: Option<Vec<MachineInfo>>,
    machines_undecided: Option<Vec<MachineInfo>>,
    pub end_reason: EndReason,

    // for statistical purposes and performance tests
    pub duration: DurationDataProvider,
    // pub name: String,
    /// Optional name of the tests or any other info.
    names: Vec<String>,

    // Additional statistics, possibly make this a struct in an Option to turn on at runtime
    // TODO HashMaps for larger
    #[cfg(feature = "bb_counter_stats")]
    pub counter_stats: CounterStats,
}

impl DeciderResultStats {
    /// Result with starting steps_max to avoid unnecessary updates on machine with max steps. \
    /// Use init_steps_max(n_states).
    pub fn new(config: &Config) -> Self {
        let init_steps_max = if config.n_states() == 1 { 0 } else { 2 };
        Self::new_init_steps_max(config, init_steps_max)
    }

    /// Creates a new result stat with higher init_steps_max which avoids storing irrelevant machines
    /// with less than max steps. Used in decider engine.
    pub fn new_init_steps_max(config: &Config, init_steps_max: StepTypeBig) -> Self {
        // limit_machines_decided is handled differently because there is no counter like num_undecided
        let limit_machines_decided = config.limit_machines_decided();
        DeciderResultStats {
            n_states: config.n_states(),
            steps_max: StepMaxResult::new(init_steps_max),
            limit_machines_decided,
            machines_decided: if limit_machines_decided > 0 {
                Some(Vec::new())
            } else {
                None
            },
            limit_machines_undecided: config.limit_machines_undecided(),
            ..Default::default()
        }
    }

    /// Set limit to highest of all configs
    pub fn enhance_machines_un_decided(&mut self, config: &Config) {
        if self.limit_machines_decided < config.limit_machines_decided() {
            self.limit_machines_decided = config.limit_machines_decided();
            if self.machines_decided.is_none() {
                self.machines_decided = Some(Vec::new());
            }
        }
        if self.limit_machines_undecided < config.limit_machines_undecided() {
            self.limit_machines_undecided = config.limit_machines_undecided();
        }
    }

    // /// Set steps_max a bit higher to avoid saving a lot of machines with low steps
    // pub fn init_steps_max(n_states: usize) -> StepTypeBig {
    //     match n_states {
    //         1 => 0,
    //         2 | 3 => 4,
    //         _ => 25,
    //     }
    // }

    pub fn limit_machines_undecided(&self) -> usize {
        self.limit_machines_undecided
    }

    // pub fn set_limit_machines_undecided(&mut self, limit: usize) {
    //     self.limit_machines_undecided = limit;
    //     if limit == 0 {
    //         self.machines_undecided = None;
    //     }
    // }

    /// Add one single result to these totals
    /// # Returns
    /// False if <limit_machines_(un)decided> (Un)decided Machines have been stored
    /// which allows the caller to stop further processing. \
    /// In this case the end_reason is set also.  
    pub fn add(&mut self, machine: &MachineBinary, status: &MachineStatus) -> bool {
        // self.num_checked_total += 1;
        let mut is_decided = true;
        self.num_evaluated += 1;
        match status {
            MachineStatus::DecidedHalts(steps) => {
                self.num_hold += 1;
                self.steps_max.add_steps(*steps, machine, status);

                #[cfg(feature = "bb_counter_stats")]
                {
                    self.counter_stats.add_steps(*steps);

                    // if *steps == 3 && self.counter_stats.hold_steps_stats[3] < 20 {
                    //     println!("Holds in 3: {}, {}", machine, status);
                    // }
                    // if *steps == 4 && self.counter_stats.hold_steps_stats[4] < 20 {
                    //     println!("Holds in 4: {}, {}", machine, status);
                    // }
                    // if *steps == 5 && self.counter_stats.hold_steps_stats[5] < 20 {
                    //     println!("Holds in 5: {}, {}", machine, status);
                    // }
                    // if *steps == 6 && self.counter_stats.hold_steps_stats[6] < 20 {
                    //     println!("Holds in 6: {}, {}", machine, status);
                    // }
                }
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
                PreDeciderReason::SimpleStartCycle => {
                    self.pre_decider_count.num_simple_start_cycle += 1
                }
                PreDeciderReason::StartRecursive => self.pre_decider_count.num_start_recursive += 1,
                PreDeciderReason::NotStartStateBRight => {
                    self.pre_decider_count.num_not_start_state_b_right += 1
                }
                PreDeciderReason::WritesOnlyZero => {
                    self.pre_decider_count.num_writes_only_zero += 1
                }
            },
            MachineStatus::DecidedHaltsDetail(_, _, _) => todo!(),
            // MachineStatus::DecidedHoldsOld(steps, _) => {
            //     self.num_hold += 1;
            //     #[cfg(feature = "bb_counter_stats")]
            //     {
            //         if steps < COUNTER_ARRAY_SIZE as StepTypeBig {
            //             self.hold_steps_stats[steps as usize] += 1;
            //         } else {
            //             self.hold_steps_stats[0] += 1;
            //         }
            //     }
            //     self.add_steps(*steps, machine, status);
            // }
            MachineStatus::DecidedNonHalt(endless_reason) => {
                self.endless_count.add_endless_reason(endless_reason);
                #[cfg(feature = "bb_counter_stats")]
                self.counter_stats.add_endless_cycle(endless_reason);
            }
            MachineStatus::Undecided(_, _, _) => {
                is_decided = false;
                if self.limit_machines_undecided > 0 {
                    if self.num_undecided < self.limit_machines_undecided as u64 {
                        if let Some(machines) = self.machines_undecided.as_mut() {
                            machines.push(MachineInfo::from_machine(machine, status));
                        } else {
                            self.machines_undecided =
                                Some(vec![MachineInfo::from_machine(machine, status)]);
                        }
                    } else {
                        self.end_reason =
                            EndReason::RecordLimitUndecidedReached(self.limit_machines_undecided);
                        return false;
                    }
                }
                self.num_undecided += 1;
            }
            MachineStatus::DecidedNotMaxTooManyHaltTransitions => {
                self.num_not_max_too_many_hold_transitions += 1;
            }
            MachineStatus::DecidedNotMaxNotAllStatesUsed => {
                self.num_not_max_not_all_states_used += 1;
            }
            MachineStatus::NoDecision => {
                panic!("State NoDecision must not be the final state. Change it to Undecided.");
            }
        }

        if is_decided && self.limit_machines_decided > 0 {
            if let Some(m_decided) = self.machines_decided.as_mut() {
                if m_decided.len() < self.limit_machines_decided {
                    m_decided.push(MachineInfo::from_machine(machine, status));
                } else {
                    self.end_reason =
                        EndReason::RecordLimitDecidedReached(self.limit_machines_decided);
                    return false;
                }
            }
        }
        true
    }

    pub fn set_name(&mut self, name: String) {
        self.add_name(&name);
    }

    /// adds or sets this name if it does not exist already
    pub fn add_name(&mut self, name: &String) {
        if !self.names.contains(name) {
            // for name in self.names.iter() {
            //     if name.as_str() == name {
            //         return;
            //     }
            // }
            self.names.push(name.to_string());
        }
    }

    /// Add another result to this result. \
    /// Returns false if <limit_machines_(un)decided> (Un)decided Machines have been stored
    /// which allows the caller to stop further processing.  
    pub fn add_result(&mut self, result: &DeciderResultStats) -> bool {
        self.num_processed_total += result.num_processed_total;
        self.num_evaluated += result.num_evaluated;
        self.num_hold += result.num_hold;
        // self.num_endless += result.num_endless;
        self.num_not_max += result.num_not_max;

        self.steps_max.add_self(&result.steps_max);

        self.pre_decider_count.add_self(&result.pre_decider_count);
        // self.pre_decider_count.num_checked = self.pre_decider_count.total() + self.num_evaluated;
        self.endless_count.add_self(&result.endless_count);

        self.num_not_max_not_all_states_used += result.num_not_max_not_all_states_used;
        self.num_not_max_too_many_hold_transitions += result.num_not_max_too_many_hold_transitions;

        let mut is_ok = true;

        if !result.names.is_empty() {
            if self.names.is_empty() {
                self.names = result.names.to_vec();
            } else {
                for name in result.names.iter() {
                    if !self.names.contains(name) {
                        self.names.push(name.to_owned());
                    }
                }
            }
        }

        // update array stats
        #[cfg(feature = "bb_counter_stats")]
        self.counter_stats.add_result(result);

        // add decided machines
        if self.limit_machines_decided > 0 {
            if let Some(d_machines) = self.machines_decided.as_mut() {
                if d_machines.len() < self.limit_machines_decided {
                    if let Some(new_machines) = result.machines_decided.as_ref() {
                        let max = new_machines
                            .len()
                            .min(self.limit_machines_decided - d_machines.len());
                        d_machines.extend_from_slice(&new_machines[0..max]);
                    }
                    if d_machines.len() >= self.limit_machines_decided {
                        self.end_reason =
                            EndReason::RecordLimitDecidedReached(self.limit_machines_decided);
                        is_ok = false;
                    }
                } else {
                    self.end_reason =
                        EndReason::RecordLimitDecidedReached(self.limit_machines_decided);
                    is_ok = false;
                }
            }
        }

        // add undecided machines
        if self.limit_machines_undecided > 0 {
            if self.num_undecided < self.limit_machines_undecided as u64 {
                if let Some(new_machines) = result.machines_undecided.as_ref() {
                    if let Some(machines) = self.machines_undecided.as_mut() {
                        let max = new_machines
                            .len()
                            .min(self.limit_machines_undecided - machines.len());
                        machines.extend_from_slice(&new_machines[0..max]);
                    } else {
                        self.machines_undecided = result.machines_undecided.clone();
                    }
                    if self.machines_undecided.as_ref().unwrap().len()
                        >= self.limit_machines_undecided
                    {
                        self.end_reason =
                            EndReason::RecordLimitUndecidedReached(self.limit_machines_undecided);
                        is_ok = false;
                    }
                }
            } else {
                self.end_reason =
                    EndReason::RecordLimitUndecidedReached(self.limit_machines_undecided);
                is_ok = false;
            }
        }
        self.num_undecided += result.num_undecided;

        // add end_reason
        if result.end_reason != EndReason::None {
            match self.end_reason {
                // only if not already an error was reported
                EndReason::AllMachinesChecked | EndReason::None => match result.end_reason {
                    EndReason::IsLastBatch => {}
                    EndReason::NoBatchData => {}
                    _ => self.end_reason = result.end_reason.clone(),
                },
                _ => {}
            }
        }

        is_ok
    }

    pub fn add_pre_decider_count(&mut self, count: &PreDeciderCount) {
        self.pre_decider_count.add_self(count);
    }

    pub fn add_total(&mut self, value: u64) {
        self.num_processed_total += value;
    }

    /// Clears the total which is required if multiple deciders run as this would result in a double count.
    pub fn clear_total(&mut self) {
        self.num_processed_total = 0;
    }

    /// Returns the first machine with max steps.
    pub fn endless_count(&self) -> &EndlessCount {
        &self.endless_count
    }

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

    pub fn machines_decided(&self) -> Option<&Vec<MachineInfo>> {
        self.machines_decided.as_ref()
    }

    /// Returns all recorded machines with max steps, sorted by id.
    pub fn machines_decided_sorted(&self) -> Option<Vec<MachineInfo>> {
        if let Some(machines) = self.machines_decided.as_ref() {
            let mut v = machines.to_vec();
            v.sort();
            Some(v)
        } else {
            None
        }
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

    pub fn n_states(&self) -> usize {
        self.n_states
    }

    pub fn num_processed_total(&self) -> u64 {
        if self.num_processed_total != 0 {
            self.num_processed_total
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

    pub fn num_total_turing_machines(&self) -> IdNormalized {
        num_turing_machine_permutations(self.n_states) as IdNormalized
    }

    pub fn num_not_max_too_many_hold_transitions(&self) -> u64 {
        self.num_not_max_too_many_hold_transitions
    }

    pub fn pre_decider_count(&self) -> PreDeciderCount {
        self.pre_decider_count
    }

    pub fn steps_max(&self) -> StepTypeBig {
        self.steps_max.steps_max()
    }

    pub fn to_string_with_duration(&self) -> String {
        let names;
        let name = if self.names.len() == 1 {
            names = String::new();
            // single name
            self.names.first().unwrap().to_string()
        } else {
            names = "\n".to_string() + self.names.join(", ").as_str();
            String::new()
        };
        format!(
            "{}{names}\n{name} time elapsed for {} machines:\n Get machines {:?} ms, decider {}, total time {}.",
            self,
            self.num_evaluated.to_formatted_string(&user_locale()),
            // duration_as_ms_rounded(self.duration.duration_data_provider),
            // duration_as_ms_rounded(self.duration.duration_decider),
            // duration_as_ms_rounded(self.duration.duration_total),
            format_duration_hhmmss_ms(self.duration.duration_data_provider, true),
            format_duration_hhmmss_ms(self.duration.duration_decider, true),
            format_duration_hhmmss_ms(self.duration.duration_total, true),
        )
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

        writeln!(f, "Result BB{}: {}", self.n_states, self.end_reason)?;
        buf.write_formatted(&self.num_total_turing_machines(), &locale);
        s.push_str(format!("Turing machines:    {:>NUM_LONG_LEN$}\n", buf.as_str()).as_str());
        if self.num_processed_total() != self.num_evaluated {
            buf.write_formatted(&self.num_processed_total, &locale);
            s.push_str(format!("Total processed:    {:>NUM_LONG_LEN$}\n", buf.as_str()).as_str());
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
        write!(f, "{s}")?;

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

        #[cfg(feature = "bb_counter_stats")]
        write!(f, "{}", self.counter_stats)?;

        Ok(())
    }
}

pub struct ResultBatchInfo {
    pub n_states: usize,
    pub steps_min: StepTypeBig,
    pub limit_machines_max_steps: usize,
    pub limit_machines_undecided: usize,
}

#[derive(Debug, Default)]
pub struct EndlessCount {
    pub num_expanding_cycler: u64,
    pub num_expanding_bouncer: u64,
    pub num_only_one_direction: u64,
    /// Eliminated machines which cannot hold because they have no hold conditions.
    pub num_no_hold_transition: u64,
    /// Eliminated machines which cannot reach the maximum steps because they have two or more hold conditions.
    pub num_simple_start_cycle: u64,
    pub num_start_recursive: u64,
    pub num_writes_only_zeros: u64,
    pub num_cycle: u64,
    pub longest_cycle: StepTypeSmall,
    pub detect_cycle_step_max: StepTypeSmall,
}

impl EndlessCount {
    pub fn add_endless_reason(&mut self, endless_reason: &NonHaltReason) {
        match endless_reason {
            // TODO check if all are needed
            // EndlessReason::ExpandingCycler => self.num_expanding_cycler += 1,
            NonHaltReason::ExpandingCycler => todo!(),
            // EndlessReason::OnlyOneDirection => self.num_only_one_direction += 1,
            NonHaltReason::OnlyOneDirection => todo!(),
            // EndlessReason::NoHoldTransition => self.num_no_hold_transition += 1,
            NonHaltReason::NoHoldTransition => todo!(),
            // EndlessReason::SimpleStartCycle => self.num_simple_start_cycle += 1,
            NonHaltReason::SimpleStartCycle => todo!(),
            // EndlessReason::StartRecursive => self.num_start_recursive += 1,
            NonHaltReason::StartRecursive => todo!(),
            // EndlessReason::WritesOnlyZero => self.num_writes_only_zeros += 1,
            NonHaltReason::WritesOnlyZero => todo!(),
            NonHaltReason::ExpandingBouncer(_) => self.num_expanding_bouncer += 1,
            // TODO steps? differentiate to expanding bouncer
            NonHaltReason::Bouncer(_) => self.num_expanding_bouncer += 1,
            NonHaltReason::Cycler(steps, cycle_size) => {
                self.num_cycle += 1;
                if *cycle_size > self.longest_cycle {
                    self.longest_cycle = *cycle_size;
                    // #[cfg(debug_assertions)]
                    // if *cycle_size > COUNTER_ARRAY_SIZE as StepTypeBig {
                    //     println!("Cycle Size {cycle_size} Machine {}", machine)
                    // }
                }
                if *steps > self.detect_cycle_step_max {
                    self.detect_cycle_step_max = *steps;
                    // #[cfg(debug_assertions)]
                    // if *cycle_size > COUNTER_ARRAY_SIZE as StepTypeBig {
                    //     println!("Cycle detected steps {steps} Machine {}", machine)
                    // }
                }
            }
        }
    }

    fn add_self(&mut self, other: &Self) {
        self.num_expanding_cycler += other.num_expanding_cycler;
        self.num_expanding_bouncer += other.num_expanding_bouncer;
        self.num_only_one_direction += other.num_only_one_direction;
        self.num_no_hold_transition += other.num_no_hold_transition;
        self.num_simple_start_cycle += other.num_simple_start_cycle;
        self.num_start_recursive += other.num_start_recursive;
        self.num_writes_only_zeros += other.num_writes_only_zeros;
        self.num_cycle += other.num_cycle;
        self.longest_cycle = other.longest_cycle.max(self.longest_cycle);
        self.detect_cycle_step_max = other.detect_cycle_step_max.max(self.detect_cycle_step_max);
    }

    fn num_endless_total(&self) -> u64 {
        self.num_expanding_cycler
            + self.num_expanding_bouncer
            + self.num_only_one_direction
            + self.num_no_hold_transition
            + self.num_simple_start_cycle
            + self.num_start_recursive
            + self.num_writes_only_zeros
            + self.num_cycle
    }
}

impl Display for EndlessCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = user_locale();
        let mut buf = Buffer::default();

        buf.write_formatted(&self.num_endless_total(), &locale);
        writeln!(
            f,
            "  {LEVEL_1_CHAR} Decided Endless:  {:>NUM_LONG_LEN$}",
            buf.as_str()
        )?;
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
        buf.write_formatted(&self.num_expanding_bouncer, &locale);
        writeln!(
            f,
            "     Expanding Bouncer:     {:>NUM_SHORT_LEN$}",
            buf.as_str()
        )?;
        buf.write_formatted(&self.num_expanding_cycler, &locale);
        writeln!(
            f,
            "     Expanding Cycle:       {:>NUM_SHORT_LEN$}",
            buf.as_str()
        )?;
        // buf.write_formatted(&self.num_simple_start_cycle, &locale);
        // s.push_str(format!("   Simple Start Cycle:  {:>NUM_SHORT_LEN$}\n", buf.as_str()).as_str());
        buf.write_formatted(&self.num_cycle, &locale);
        writeln!(
            f,
            "     Cycle:                 {:>NUM_SHORT_LEN$}",
            buf.as_str()
        )?;
        writeln!(
            f,
            "     - Longest Cycle:       {:>NUM_SHORT_LEN$}",
            self.longest_cycle
        )?;
        writeln!(
            f,
            "     - Detect Step Max:     {:>NUM_SHORT_LEN$}",
            self.detect_cycle_step_max
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PreDeciderCount {
    // reference for percent calculation, holds the total number checked (not only pre-decider)
    pub num_checked_total_for_display: u64,
    pub num_not_all_states_used: u64,
    pub num_not_exactly_one_hold_condition: u64,
    pub num_not_enumerated: u64,
    pub num_not_start_state_b_right: u64,
    pub num_only_one_direction: u64,
    pub num_simple_start_cycle: u64,
    pub num_start_recursive: u64,
    pub num_writes_only_zero: u64,
    // TODO num_hold or DeciderStats
}

impl PreDeciderCount {
    pub fn add_self(&mut self, other: &Self) {
        self.num_not_all_states_used += other.num_not_all_states_used;
        self.num_not_exactly_one_hold_condition += other.num_not_exactly_one_hold_condition;
        self.num_not_enumerated += other.num_not_enumerated;
        self.num_only_one_direction += other.num_only_one_direction;
        self.num_simple_start_cycle += other.num_simple_start_cycle;
        self.num_start_recursive += other.num_start_recursive;
        self.num_not_start_state_b_right += other.num_not_start_state_b_right;
        self.num_writes_only_zero += other.num_writes_only_zero;
    }

    pub fn num_total(&self) -> u64 {
        self.num_not_all_states_used
            + self.num_not_exactly_one_hold_condition
            + self.num_not_enumerated
            + self.num_only_one_direction
            + self.num_simple_start_cycle
            + self.num_not_start_state_b_right
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
            if self.num_not_enumerated != 0 {
                buf.write_formatted(&self.num_not_enumerated, &locale);
                s.push_str(
                    format!(
                        "    - Not Generated:           {:>NUM_LONG_LEN$}",
                        buf.as_str()
                    )
                    .as_str(),
                );
                if self.num_checked_total_for_display != 0 {
                    let p = ((self.num_not_enumerated * 10000 / self.num_checked_total_for_display)
                        as f64)
                        .round()
                        / 100.0;
                    s.push_str(format!(" ({p:.1}%)").as_str());
                }
                s.push('\n');
            }
            if self.num_not_start_state_b_right > 0 {
                buf.write_formatted(&self.num_not_start_state_b_right, &locale);
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
            buf.write_formatted(&self.num_simple_start_cycle, &locale);
            s.push_str(
                format!(
                    "    - Simple Start Cycle:          {:>NUM_SHORT_LEN$}\n",
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
        write!(f, "{s}")
    }
}

/// Duration of the enumerator/data provider tasks.
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
    steps_max: StepTypeBig,
    // steps_min: StepTypeBig,
    num_machines_steps_max: usize,
    machines_max_steps: Option<Vec<MachineInfo>>,
}

impl StepMaxResult {
    pub fn new(steps_min: StepTypeBig) -> Self {
        Self {
            // steps_min,
            steps_max: steps_min,
            ..Default::default()
        }
    }

    pub fn add_self(&mut self, other: &Self) {
        if other.steps_max >= self.steps_max {
            if other.steps_max == self.steps_max {
                self.num_machines_steps_max += other.num_machines_steps_max;
                if let Some(machines) = other.machines_max_steps.as_ref() {
                    if self.machines_max_steps.is_none() {
                        self.machines_max_steps = Some(machines.clone());
                    } else {
                        self.machines_max_steps.as_mut().unwrap().extend(machines);
                    }
                }
            } else {
                // new max
                self.steps_max = other.steps_max;
                self.num_machines_steps_max = other.num_machines_steps_max;
                self.machines_max_steps = other.machines_max_steps.clone();
            }
        }
    }

    fn add_steps(&mut self, steps: StepTypeBig, machine: &MachineBinary, status: &MachineStatus) {
        // Check biggerThan to avoid two ifs on every check as it occurs rarely
        if steps >= self.steps_max {
            if steps == self.steps_max {
                // store additional max step machine
                if self.machines_max_steps.is_none() {
                    self.machines_max_steps = Some(Vec::with_capacity(4));
                }
                self.machines_max_steps
                    .as_mut()
                    .unwrap()
                    .push(MachineInfo::from_machine(machine, status));
                // println!("  Added machine for max step {steps}");
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
            }
        }
    }

    /// Returns the first machine with max steps.
    pub fn machine_max_steps(&self) -> Option<MachineInfo> {
        if let Some(machines) = self.machines_max_steps.as_ref() {
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

    // fn len_machines_max_steps(&self) -> usize {
    //     match self.machines_max_steps.as_ref() {
    //         Some(m) => m.len(),
    //         None => 0,
    //     }
    // }

    pub fn sort_machines(&mut self) {
        if let Some(v) = self.machines_max_steps.as_mut() {
            // v.sort_by(|a, b| a.id().cmp(&b.id()));
            v.sort();
        }
    }

    /// Returns the recorded steps max. If steps_min is given, steps_max may not hold the correct value.
    pub fn steps_max(&self) -> StepTypeBig {
        if self.num_machines_steps_max == 0 {
            0
        } else {
            self.steps_max
        }
    }
}

impl Display for StepMaxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let locale = user_locale();
        writeln!(
            f,
            "  Max Steps:      {:>10} (Number of machines: {})",
            self.steps_max().to_formatted_string(&locale),
            self.num_machines_steps_max,
        )?;
        // print first max step machines
        if self.num_machines_steps_max == 1 {
            if let Some(m) = self.machine_max_steps() {
                writeln!(
                    f,
                    "   Machine No. {}: {}",
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
                writeln!(
                    f,
                    "   Machine No. {:>len$}: {}",
                    m.id().to_formatted_string(&locale),
                    m.to_standard_tm_text_format()
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct MachinesStates {
    /// All undecided machines of one batch run. \
    /// Machines can be used directly in next batch run with undecided only.
    pub machines: Vec<MachineBinary>,
    /// The detailed MachineStatus which holds the UndecidedReason. State corresponds with the machine with the same index.
    pub states: Vec<MachineStatus>,
}

impl MachinesStates {
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
            infos.push(MachineInfo::new(*m, self.states[i]));
        }

        infos
    }
}

// impl Default for MachinesUndecided {
//     fn default() -> Self {
//         Self {
//             machines: Default::default(),
//             states: Default::default(),
//         }
//     }
// }

/// Result of a batch run with results for all machines in the batch.
/// All undecided Turing machines are recorded in detail.
pub struct BatchResult {
    pub result_decided: DeciderResultStats,
    pub machines_undecided: MachinesStates,
    pub batch_no: usize,
    pub num_batches: usize,
    pub decider_name: String,
}

/// Result of a batch run with results for all machines in the batch.
/// All undecided Turing machines are recorded in detail.
// TODO reduce pub fields
#[derive(Debug)]
pub struct BatchData<'a> {
    pub machines: &'a [MachineBinary],
    pub result_decided: DeciderResultStats,
    pub machines_decided: MachinesStates,
    pub machines_undecided: MachinesStates,
    /// Current batch no, first batch is 0.
    pub batch_no: usize,
    pub num_batches: usize,
    pub decider_id: &'a DeciderId,
    pub run_predecider: PreDeciderRun,
    pub config: &'a Config,
}

/// Result of a batch run with results for all machines in the batch.
/// All undecided Turing machines are recorded in detail.
// TODO reduce pub fields
#[derive(Debug)]
pub struct BatchDataThread<'a> {
    pub machines: Vec<MachineBinary>,
    pub result_decided: DeciderResultStats,
    pub machines_undecided: MachinesStates,
    /// Current batch no, first batch is 0.
    pub batch_no: usize,
    pub num_batches: usize,
    pub decider_id: usize,
    pub run_predecider: PreDeciderRun,
    pub config: &'a Config,
}

pub fn result_max_steps_known(n_states: usize) -> StepTypeBig {
    match n_states {
        1 => 1,
        2 => 6,
        3 => 21,
        4 => 107,
        5 => 47_176_870,
        _ => panic!("result_max_steps: Not build for this."),
    }
}

#[cfg(feature = "bb_counter_stats")]
pub const COUNTER_ARRAY_SIZE: usize = 110;

#[cfg(feature = "bb_counter_stats")]
#[derive(Debug)]
pub struct CounterStats {
    /// Array for the first 100 steps, [0] holds all which are greater
    pub hold_steps_stats: [StepTypeBig; COUNTER_ARRAY_SIZE],
    pub cycle_size_stats: [StepTypeBig; COUNTER_ARRAY_SIZE],
    pub cycle_steps_stats: [StepTypeBig; COUNTER_ARRAY_SIZE],
    // HashMap for larger
    // pub hold_steps_long: HashMap<StepTypeBig, StepTypeBig>,
}

#[cfg(feature = "bb_counter_stats")]
impl CounterStats {
    pub fn add_steps(&mut self, steps: StepTypeBig) {
        if steps < COUNTER_ARRAY_SIZE as StepTypeBig {
            self.hold_steps_stats[steps as usize] += 1;
        } else {
            self.hold_steps_stats[0] += 1;
        }
    }

    pub fn add_endless_cycle(&mut self, endless_reason: &NonHaltReason) {
        match endless_reason {
            NonHaltReason::Cycler(steps, cycle_size) => {
                if *cycle_size < COUNTER_ARRAY_SIZE as StepTypeBig {
                    self.cycle_size_stats[*cycle_size as usize] += 1;
                } else {
                    self.cycle_size_stats[0] += 1;
                }
                if *steps < COUNTER_ARRAY_SIZE as StepTypeBig {
                    self.cycle_steps_stats[*steps as usize] += 1;
                } else {
                    self.cycle_steps_stats[0] += 1;
                }
            }
            _ => {}
        }
    }

    pub fn add_result(&mut self, result: &DeciderResultStats) {
        for i in 0..COUNTER_ARRAY_SIZE {
            self.hold_steps_stats[i] += result.counter_stats.hold_steps_stats[i];
            self.cycle_size_stats[i] += result.counter_stats.cycle_size_stats[i];
            self.cycle_steps_stats[i] += result.counter_stats.cycle_steps_stats[i];
        }
    }
}

#[cfg(feature = "bb_counter_stats")]
impl Default for CounterStats {
    fn default() -> Self {
        Self {
            hold_steps_stats: [0; COUNTER_ARRAY_SIZE],
            cycle_size_stats: [0; COUNTER_ARRAY_SIZE],
            cycle_steps_stats: [0; COUNTER_ARRAY_SIZE],
        }
    }
}

#[cfg(feature = "bb_counter_stats")]
impl Display for CounterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\nCounter Statistic:")?;
        let steps: StepTypeBig = self.hold_steps_stats.iter().sum();
        writeln!(
            f,
            "Hold: Steps till {}: total {}\n{}",
            COUNTER_ARRAY_SIZE,
            steps,
            fmt_array(&self.hold_steps_stats)
        )?;

        writeln!(
            f,
            "Cycle: Size till {}:\n{}",
            COUNTER_ARRAY_SIZE,
            fmt_array(&self.cycle_size_stats)
        )?;
        writeln!(
            f,
            "Cycle: Step detected {}:\n{}",
            COUNTER_ARRAY_SIZE,
            fmt_array(&self.cycle_steps_stats)
        )
    }
}

#[cfg(feature = "bb_counter_stats")]
fn fmt_array(arr: &[StepTypeBig]) -> String {
    let mut v = Vec::new();
    let mut start = 0;
    while start < arr.len() {
        let a = arr[start..]
            .iter()
            .take(25)
            .copied()
            .collect::<Vec<StepTypeBig>>();
        v.push(format!("{start:>3}: {:?}", a));
        start += 25;
    }
    let locale = user_locale();
    let first_10 = arr.iter().skip(1).take(10).sum::<StepTypeBig>();
    let first_25 = arr.iter().skip(1).take(25).sum::<StepTypeBig>();
    let first_50 = arr.iter().skip(1).take(50).sum::<StepTypeBig>();
    let total = arr.iter().sum::<StepTypeBig>();
    let first_10_p = (first_10 as f64 * 1000.0 / total as f64).round() / 10.0;
    let first_25_p = (first_25 as f64 * 1000.0 / total as f64).round() / 10.0;
    let first_50_p = (first_50 as f64 * 1000.0 / total as f64).round() / 10.0;
    v.push(format!(
        "  First 10: {} ({first_10_p}%), First 25: {} ({first_25_p}%), First 50: {} ({first_50_p}%) Total: {}",
        first_10.to_formatted_string(&locale),
        first_25.to_formatted_string(&locale),
        first_50.to_formatted_string(&locale), total.to_formatted_string(&locale),
    ));
    v.join("\n")
}
