#[cfg(feature = "bb_enable_html_reports")]
use std::fs::File;
use std::{fmt::Display, time::Duration};

#[cfg(feature = "bb_enable_html_reports")]
use crate::html;
use crate::{
    config::{Config, StepTypeBig, StepTypeSmall, MAX_TAPE_GROWTH, TAPE_SIZE_INIT_CELL_BLOCKS},
    decider_bouncer::DeciderBouncer,
    decider_result::{BatchData, DeciderResultStats, EndReason, PreDeciderCount},
    decider_result_worker::FnResultWorker,
    machine::Machine,
    machine_info::MachineInfo,
    pre_decider::{run_pre_decider_simple, run_pre_decider_strict, PreDeciderRun},
    status::{MachineStatus, UndecidedReason},
    tape_utils::{
        CLEAR_HIGH95_64BITS_U128, CLEAR_LOW63_00BITS_U128, CLEAR_LOW63_32BITS_U128,
        HIGH32_SWITCH_U128, LOW32_SWITCH_U128, MIDDLE_BIT_U128, POS_HALF_U128,
        TAPE_SIZE_FOURTH_UPPER_128, TAPE_SIZE_HALF_128, TL_POS_START_128,
    },
    transition_symbol2::{TransitionSymbol2, TransitionTableSymbol2, TRANSITION_SYM2_START},
    ResultUnitEndReason,
};

// This result gives a clear indication if an error occurred. It returns the data which has been processed so far.
pub type ResultDecider = Result<DeciderResultStats, Box<DeciderError>>;

// pub type ResultDecider = std::result::Result<(), DeciderError>;
pub type FnDeciderRunBatchV2 = fn(&mut BatchData) -> ResultUnitEndReason;

// /// The deciders need to return Self to be able to make a new decider for each thread.
// /// This makes them not object save and thus cannot be passed in a Vec.
// pub(crate) enum DeciderEnum {
//     PreDecider(Box<crate::pre_decider::PreDecider>),
//     LoopV4(Box<crate::decider_cycler_v4::DeciderCyclerV4>),
//     HoldLong(Box<crate::decider_hold_u128_long::DeciderHoldU128Long>),
// }
//
// pub(crate) enum DeciderEnumV2<'a> {
//     PreDecider(&'a mut PreDecider),
//     LoopV4(&'a mut DeciderCyclerV4),
//     // LoopV4(Box<crate::decider_loop_v4::DeciderLoopV4>),
//     // HoldLong(Box<crate::decider_hold_u128_long::DeciderHoldU128Long>),
// }

pub enum DeciderStandard {
    Bouncer,
    Cycler,
    Hold,
}

impl DeciderStandard {
    pub fn decider_caller(&self) -> DeciderCaller<'_> {
        match self {
            DeciderStandard::Bouncer => {
                DeciderCaller::new(&DECIDER_BOUNCER_ID, DeciderBouncer::decider_run_batch)
            }
            DeciderStandard::Cycler => DeciderCaller::new(
                &DECIDER_CYCLER_ID,
                crate::decider_cycler::DeciderCycler::decider_run_batch,
            ),
            DeciderStandard::Hold => DeciderCaller::new(
                &DECIDER_HOLD_ID,
                crate::decider_hold_u128_long_v3::DeciderHoldU128Long::decider_run_batch,
            ),
        }
    }

    pub fn decider_config<'a>(&self, config: &'a Config) -> DeciderConfig<'a> {
        match self {
            DeciderStandard::Bouncer => DeciderConfig::new(
                &DECIDER_BOUNCER_ID,
                DeciderBouncer::decider_run_batch,
                config,
            ),
            DeciderStandard::Cycler => DeciderConfig::new(
                &DECIDER_CYCLER_ID,
                crate::decider_cycler::DeciderCycler::decider_run_batch,
                config,
            ),
            DeciderStandard::Hold => DeciderConfig::new(
                &DECIDER_HOLD_ID,
                crate::decider_hold_u128_long_v3::DeciderHoldU128Long::decider_run_batch,
                config,
            ),
        }
    }
}

// Deciders in this library
pub const DECIDER_HOLD_ID: DeciderId = DeciderId {
    id: 10,
    name: "Decider Hold",
};
pub const DECIDER_CYCLER_ID: DeciderId = DeciderId {
    id: 20,
    name: "Decider Cycler",
};
pub const DECIDER_CYCLER_LONG_ID: DeciderId = DeciderId {
    id: 21,
    name: "Decider Cycler Long",
};
pub const DECIDER_BOUNCER_ID: DeciderId = DeciderId {
    id: 20,
    name: "Decider Bouncer",
};

/// This struct defines the call to the decider and its name.
#[derive(Debug, Clone, Copy)]
pub struct DeciderCaller<'a> {
    decider_id: &'a DeciderId,
    f_decider: FnDeciderRunBatchV2,
}

impl<'a> DeciderCaller<'a> {
    pub fn new(decider_id: &'a DeciderId, f_decider: FnDeciderRunBatchV2) -> Self {
        Self {
            decider_id,
            f_decider,
        }
    }

    pub fn decider_id(&self) -> &'a DeciderId {
        self.decider_id
    }

    pub fn f_decider(&self) -> fn(&mut BatchData<'_>) -> Result<(), EndReason> {
        self.f_decider
    }
}

/// This struct is used to chain the deciders.
#[derive(Debug, Clone, Copy)]
pub struct DeciderConfig<'a> {
    decider_id: &'a DeciderId,
    f_decider_run_batch: FnDeciderRunBatchV2,
    pub fo_result_worker: Option<FnResultWorker>,
    config: &'a Config,
}

impl<'a> DeciderConfig<'a> {
    pub fn new(
        decider_id: &'a DeciderId,
        f_decider: FnDeciderRunBatchV2,
        config: &'a Config,
    ) -> Self {
        Self {
            decider_id,
            f_decider_run_batch: f_decider,
            fo_result_worker: None,
            config,
        }
    }

    pub fn new_caller(decider_caller: &'a DeciderCaller, config: &'a Config) -> Self {
        Self {
            decider_id: decider_caller.decider_id,
            f_decider_run_batch: decider_caller.f_decider,
            fo_result_worker: None,
            config,
        }
    }

    pub fn new_with_worker(
        decider_id: &'a DeciderId,
        f_decider: FnDeciderRunBatchV2,
        f_result_worker: FnResultWorker,
        config: &'a Config,
    ) -> Self {
        Self {
            decider_id,
            f_decider_run_batch: f_decider,
            fo_result_worker: Some(f_result_worker),
            config,
        }
    }

    pub fn f_decider(&self) -> FnDeciderRunBatchV2 {
        self.f_decider_run_batch
    }

    // pub fn f_result_worker(&self) -> FnResultWorker {
    //     self.f_result_worker
    // }

    pub fn fo_result_worker(&self) -> Option<FnResultWorker> {
        self.fo_result_worker
    }

    pub fn config(&self) -> &'a Config {
        self.config
    }

    pub fn decider_id(&self) -> &DeciderId {
        self.decider_id
    }
}

// pub trait DeciderMinimalTest {
//     /// Returns the result of this decider for one single machine. \
//     /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
//     fn decide_machine_minimal(&mut self, machine: &Machine) -> MachineStatus;
//
//     /// Returns the name of this decider
//     fn name_minimal(&self) -> &str;
// }

/// Decider identification. As only the function to run the decider is passed, the id can not be requested
/// and needs to be part of the DeciderConfig.

#[derive(Debug, Default, Clone, Copy)]
pub struct DeciderId {
    pub id: usize,
    pub name: &'static str,
}

// impl DeciderId {
//     pub fn new(id: usize, name: &'static str) -> Self {
//         Self { id, name }
//     }
//
//     pub fn id(&self) -> usize {
//         self.id
//     }
//
//     pub fn name(&self) -> &'static str {
//         self.name
//     }
// }

pub trait Decider {
    // TODO into id, name struct
    fn decider_id() -> &'static DeciderId;

    //     /// Returns the name of this decider
    //     fn id(&self) -> usize;
    //
    //     /// Returns the name of this decider
    //     fn name(&self) -> &str;

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

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason;
}

// impl From<&Config> for Decider {
//     fn from(value: &Config) -> Self {
//         Self::new_from_config(config)
//     }
// }

#[inline]
pub fn decider_generic_run_batch_v2(
    mut decider: impl Decider,
    batch_data: &mut BatchData,
) -> ResultUnitEndReason {
    if batch_data.machines.is_empty() {
        return Err(EndReason::NoBatchData);
    }

    let limit_decided = batch_data.config.limit_machines_decided();
    match batch_data.run_predecider {
        PreDeciderRun::DoNotRun => {
            for machine in batch_data.machines.iter() {
                let status = decider.decide_machine(machine);
                // This part is identical for all branches
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        if limit_decided > 0
                            && batch_data.machines_decided.machines.len() < limit_decided
                        {
                            batch_data.machines_decided.machines.push(*machine);
                            batch_data.machines_decided.states.push(status);
                        }
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
                // This part is identical for all branches
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        if limit_decided > 0
                            && batch_data.machines_decided.machines.len() < limit_decided
                        {
                            batch_data.machines_decided.machines.push(*machine);
                            batch_data.machines_decided.states.push(status);
                        }
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
                // This part is identical for all branches
                match status {
                    MachineStatus::Undecided(_, _, _) => {
                        batch_data.machines_undecided.machines.push(*machine);
                        batch_data.machines_undecided.states.push(status);
                    }
                    _ => {
                        if limit_decided > 0
                            && batch_data.machines_decided.machines.len() < limit_decided
                        {
                            batch_data.machines_decided.machines.push(*machine);
                            batch_data.machines_decided.states.push(status);
                        }
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

#[derive(Debug, Default)]
pub struct DeciderError {
    pub decider_id: DeciderId,
    pub machine: Option<MachineInfo>,
    pub decider_result: Option<DeciderResultStats>,
    pub msg: String,
}

impl std::error::Error for DeciderError {}

// impl From<std::io::Error> for DeciderError {
//     fn from(error: std::io::Error) -> Self {
//         ResultWorkerError::FileError(error.to_string())
//     }
// }

impl Display for DeciderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.decider_id.name, self.msg)?;
        if let Some(machine) = &self.machine {
            write!(f, "\n{machine}")?;
        }
        if let Some(decider_result) = &self.decider_result {
            write!(f, "\n{decider_result}")?;
        }
        Ok(())
    }
}

/// This contains the functionality for a hold decider.
/// It can be used to build a specific decider.
#[derive(Debug)]
pub struct DeciderData128 {
    /// Partial fast Turing tape which shifts in every step, so that the head is always at the MIDDLE_BIT.
    tape_shifted: u128,
    // Indication where the original pos_middle has moved within tape_shifted. Used to load data from long_tape.
    pos_middle: usize,
    // The tape_long is a Vec<u64> which allows to copy half of u128 tape_shifted to
    // be copied into the long tape when a bound is reached.
    // TODO The tape has an initial size of e.g. 128 u64 which is 1024 Byte or 8192 tape cells.
    // The size will double every time its limit is reached. E.g it doubles 1x times to get a size of 256 or 16284 cells,
    // which is the size for BB5 Max tape length.
    // Once 131072 u64 is reached (1 MB), it will grow by 1 MB each time.
    // Generally speaking here the head is moving within the tape; it does not shift at all.
    tape_long: Vec<u32>,
    /// tl_pos represents the start of the 128 tape in the long tape (covering 4 u32)
    tl_pos: usize,
    /// High bound in tape_long, this is the rightmost value.
    tl_high_bound: usize,
    /// TODO low bound in bit, this is the rightmost doubleword (16-bit) in tape_shifted (bit 0), min value is 0, but will be negative when testing.
    /// Low bound in tape_long, this is the leftmost value.
    tl_low_bound: usize,

    // machine id, just for debugging
    // machine_id: IdBig,
    pub transition_table: TransitionTableSymbol2,

    /// Current transition
    pub tr: TransitionSymbol2,
    /// Field Id of the current transition. This is the table field, e.g. B1 converted to a 1D-map (A0=2, B1=5).
    pub tr_field_id: usize,

    /// TODO Number of steps, where first step is TODO 0
    pub num_steps: StepTypeBig,
    /// Maximum number of steps, after that Undecided will be returned.
    pub step_limit: StepTypeBig,
    /// Tape size limit in number of cells
    tape_size_limit_u32_blocks: u32,
    /// Final status, only valid once machine has ended, but intended to be used internally.
    pub status: MachineStatus,
    /// HTML step limit limits output to file. Set to 0 if write_html_file is false.
    #[cfg(feature = "bb_enable_html_reports")]
    pub write_html_line_limit: u32,
    #[cfg(feature = "bb_enable_html_reports")]
    pub write_html_line_count: u32,
    #[cfg(feature = "bb_enable_html_reports")]
    pub write_html_step_limit: u32,
    #[cfg(feature = "bb_enable_html_reports")]
    pub path: Option<String>,
    #[cfg(feature = "bb_enable_html_reports")]
    pub file_name: Option<String>,
    #[cfg(feature = "bb_enable_html_reports")]
    pub file: Option<File>,
}

impl DeciderData128 {
    // Sets the defaults and start transition A0.
    pub fn new(config: &Config) -> Self {
        Self {
            tape_shifted: 0,
            pos_middle: MIDDLE_BIT_U128,

            tape_long: vec![0; TAPE_SIZE_INIT_CELL_BLOCKS],
            tl_pos: TL_POS_START_128,
            tl_low_bound: TL_POS_START_128,
            tl_high_bound: TL_POS_START_128 + 3,

            num_steps: 0,
            transition_table: TransitionTableSymbol2::default(),
            // Initialize transition with A0 as start
            tr: TRANSITION_SYM2_START,
            tr_field_id: 2,
            // copy the transition table as this runs faster
            // machine_id: 0,
            // transition_table: TransitionTableSymbol2::default(),
            status: MachineStatus::NoDecision,
            step_limit: config.step_limit_hold(),
            tape_size_limit_u32_blocks: config.tape_size_limit_u32_blocks(),
            #[cfg(feature = "bb_enable_html_reports")]
            write_html_line_limit: if config.write_html_file() {
                config.write_html_line_limit()
            } else {
                0
            },
            write_html_line_count: 0,
            #[cfg(feature = "bb_enable_html_reports")]
            write_html_step_limit: if config.write_html_file() {
                config.write_html_step_limit()
            } else {
                0
            },
            #[cfg(feature = "bb_enable_html_reports")]
            path: None,
            #[cfg(feature = "bb_enable_html_reports")]
            file_name: None,
            #[cfg(feature = "bb_enable_html_reports")]
            file: None,
        }
    }

    // resets the decider for a different machine
    pub fn clear(&mut self) {
        self.tape_shifted = 0;
        self.pos_middle = MIDDLE_BIT_U128;

        self.tape_long.clear();
        self.tape_long.resize(TAPE_SIZE_INIT_CELL_BLOCKS, 0);
        self.tl_pos = TL_POS_START_128;
        self.tl_low_bound = TL_POS_START_128;
        self.tl_high_bound = TL_POS_START_128 + 3;

        self.num_steps = 0;
        self.tr = TRANSITION_SYM2_START;
        self.tr_field_id = 2;
        self.status = MachineStatus::NoDecision;
        // keep step_limit and other config data
    }

    /// Counts Ones for self referencing speed-up
    #[inline(always)]
    fn count_left(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = (self.tape_shifted >> 64) as u64;
        if symbol == 1 {
            t.trailing_ones() + 1
        } else {
            t.trailing_zeros() + 1
        }
    }

    /// Counts Ones for self referencing speed-up
    #[inline(always)]
    fn count_right(&self, symbol: usize) -> u32 {
        // count 1s starting from middle; get lower part
        let t = self.tape_shifted as u64;
        if symbol == 1 {
            t.leading_ones()
        } else {
            t.leading_zeros()
        }
    }

    // TODO correct data
    pub fn count_ones(&self) -> StepTypeSmall {
        println!("WARNING: This count is incorrect");
        let mut ones = self.tape_shifted.count_ones();
        if self.tl_high_bound - self.tl_low_bound > 3 {
            for n in self.tape_long[self.tl_low_bound..self.tl_pos].iter() {
                ones += n.count_ones();
            }
            for n in self.tape_long[self.tl_pos + 4..self.tl_high_bound + 1].iter() {
                ones += n.count_ones();
            }
        }
        ones as StepTypeSmall
    }

    #[inline(always)]
    pub fn get_current_symbol(&self) -> usize {
        // resolves to one if bit is set
        ((self.tape_shifted & POS_HALF_U128) != 0) as usize
    }

    #[inline(always)]
    pub fn next_transition(&mut self) {
        self.num_steps += 1;
        self.tr_field_id = self.tr.state_x2() + self.get_current_symbol();
        self.tr = self.transition_table.transition(self.tr_field_id);
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        println!("{}", self.data.step_to_string());
    }

    /// Checks if the decider is done, either because of hold or step limit.
    /// Sets the status field.
    #[inline(always)]
    pub fn is_done(&mut self) -> bool {
        if self.tr.is_hold() {
            // write last symbol
            if !self.tr.is_symbol_undefined() {
                self.set_current_symbol();
            }
            self.status = MachineStatus::DecidedHolds(self.num_steps);
            // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
            #[cfg(feature = "bb_enable_html_reports")]
            self.write_step_html();
            return true;
        } else if self.num_steps >= self.step_limit {
            self.status = self.status_undecided_step_limit();
            #[cfg(feature = "bb_enable_html_reports")]
            self.write_step_html();
            return true;
        }
        false
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn is_write_html_in_limit(&self) -> bool {
        self.write_html_step_limit != 0
            && self.num_steps <= self.write_html_step_limit
            && self.write_html_line_count < self.write_html_line_limit
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn is_write_html_file(&self) -> bool {
        self.write_html_step_limit != 0
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn rename_html_file_to_status(&self) {
        if let Some(file_name) = self.file_name.as_ref() {
            let path = self.path.as_ref().unwrap();
            // self.file = None;
            html::rename_file_to_status(path, file_name, &self.status);
        }
    }

    /// Update tape: write symbol at head position into cell
    #[inline(always)]
    fn set_current_symbol(&mut self) {
        if self.tr.is_symbol_one() {
            self.tape_shifted |= POS_HALF_U128
        } else {
            self.tape_shifted &= !POS_HALF_U128
        };
    }

    /// Shifts the pos in the long tape one to left and checks Vec dimensions. \
    /// Here the vector needs to be expanded at the beginning and the data must be shifted.
    #[inline(always)]
    fn shift_pos_to_left_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos < self.tl_low_bound + 1 {
            if self.tl_pos == 0 {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 0 -> pos 40
                let mut grow_by = MAX_TAPE_GROWTH.min(self.tape_long.len());
                let old_len = self.tape_long.len();
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by
                    > self.tape_size_limit_u32_blocks as usize
                {
                    // TODO this is untested
                    grow_by = self.tape_size_limit_u32_blocks as usize
                        - self.tape_long.len()
                        - self.tl_low_bound;
                    if grow_by == 0 {
                        self.status = MachineStatus::Undecided(
                            UndecidedReason::TapeSizeLimit,
                            self.num_steps,
                            self.tape_size(),
                        );
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  Tape Resize at start: {} -> {}",
                    self.tape_long.len(),
                    self.tape_long.len() + grow_by
                );
                // Make room in beginning. Grow vector first, then move elements.
                self.tape_long.resize(self.tape_long.len() + grow_by, 0);
                self.tape_long.copy_within(0..old_len, grow_by);
                self.tape_long[0..grow_by].fill(0);
                self.tl_pos += grow_by;
                self.tl_low_bound += grow_by;
                self.tl_high_bound += grow_by;
            }
            self.tl_low_bound -= 1;
        }
        self.tl_pos -= 1;

        true
    }

    /// Shifts the pos in the long tape one to right and checks Vec dimensions. \
    /// Here the vector can easily be expanded at the end.
    #[inline(always)]
    pub fn shift_pos_to_right_checked(&mut self) -> bool {
        // check if tape is long enough
        if self.tl_pos + 4 > self.tl_high_bound {
            self.tl_high_bound += 1;
            if self.tl_high_bound == self.tape_long.len() {
                // Example: len = 100, grow_by = 40 -> new len = 140, pos 96 -> pos 96
                let mut grow_by = MAX_TAPE_GROWTH.min(self.tape_long.len());
                // check tape size limit
                if self.tape_long.len() + self.tl_low_bound + grow_by
                    > self.tape_size_limit_u32_blocks as usize
                {
                    // TODO this is untested
                    grow_by = self.tape_size_limit_u32_blocks as usize
                        - self.tape_long.len()
                        - self.tl_low_bound;
                    if grow_by == 0 {
                        self.status = MachineStatus::Undecided(
                            UndecidedReason::TapeSizeLimit,
                            self.num_steps,
                            self.tape_size(),
                        );
                        return false;
                    }
                }
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!(
                    "  Tape Resize at end: {} -> {}",
                    self.tape_long.len(),
                    self.tape_long.len() + grow_by
                );
                self.tape_long.resize(self.tape_long.len() + grow_by, 0);
            }
        }
        self.tl_pos += 1;

        true
    }

    fn status_undecided_step_limit(&self) -> MachineStatus {
        MachineStatus::Undecided(
            UndecidedReason::StepLimit,
            self.num_steps as StepTypeBig,
            self.tape_size(),
        )
    }

    /// Returns the status of the decider
    pub fn status(&self) -> MachineStatus {
        self.status
    }

    /// Returns the status of the decider and additionally written Ones on tape and Tape Size
    /// TODO NOTE: This is incorrect! see count_ones
    pub fn status_full(&self) -> MachineStatus {
        match self.status {
            MachineStatus::DecidedHolds(steps) => {
                MachineStatus::DecidedHoldsDetail(steps, self.tape_size(), self.count_ones())
            }
            _ => self.status,
        }
    }

    // TODO implement
    // pub fn status_hold_details(&self) -> MachineStatus {
    //     MachineStatus::DecidedHoldsDetail(
    //         self.num_steps as StepType,
    //         self.get_tape_size(),
    //         self.tape_shifted.count_ones() as usize,
    //     )
    // }

    pub fn step_limit(&self) -> StepTypeBig {
        self.step_limit
    }

    pub fn tape_shifted(&self) -> u128 {
        self.tape_shifted
    }

    /// Returns the approximate tape size, which grows by 32 steps
    pub fn tape_size(&self) -> u32 {
        ((self.tl_high_bound - self.tl_low_bound + 1) * 32) as u32
    }

    /// Updates tape_shifted and tape_long.
    /// Also prints and writes step to html if feature is set.
    #[inline(always)]
    pub fn update_tape_single_step(&mut self) -> bool {
        self.set_current_symbol();
        let shift_ok = if self.tr.is_dir_right() {
            self.tape_shifted <<= 1;
            self.pos_middle += 1;
            self.shift_tape_long_head_dir_right()
        } else {
            self.tape_shifted >>= 1;
            self.pos_middle -= 1;
            self.shift_tape_long_head_dir_left()
        };
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            if self.num_steps % 100 == 0 {
                println!();
            }
            println!("{}", self.step_to_string());
        }
        #[cfg(feature = "bb_enable_html_reports")]
        self.write_step_html();

        shift_ok
    }

    /// Updates tape_shifted and tape_long.
    #[inline(always)]
    pub fn update_tape_self_ref_speed_up(&mut self) -> bool {
        let shift_ok = if self.tr.is_dir_right() {
            // normal shift RIGHT -> tape moves left

            // Check if self referencing, which speeds up the shift greatly.
            // Self referencing means also that the symbol does not change, ergo no need to update the fields
            if self.tr.array_id() == self.tr_field_id {
                // get jump within tape_shifted, which is only the lower part and thus a maximum of 63 bits
                let mut jump = self.count_right(self.tr_field_id & 1) as usize;
                // if self.num_steps > 50_000 {
                //     // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                //     println!("  jump R {jump}, {}", self.step_to_string());
                // }
                // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                // Interestingly, the version with the long_jump logic runs faster.
                let mut long_jump = false;
                if jump == 32 && self.pos_middle + jump == HIGH32_SWITCH_U128 {
                    // check further for larger jump
                    // compare depending on symbol
                    let v32 = if self.tr_field_id & 1 == 0 {
                        0
                    } else {
                        u32::MAX
                    };
                    // head goes right, tape shifts left
                    // tl_pos + 2 is now a known required value v32, because that is what count_right just tested
                    let mut p = self.tl_pos + 3;
                    let mut j = 1;
                    while p <= self.tl_high_bound && self.tape_long[p] == v32 {
                        p += 1;
                        j += 1;
                    }
                    // j is one more as the first one is already checked with count_right
                    if j >= 2 {
                        // if tape_shifted_left_0 != v32 {
                        //     println!("Not v32 {v32} but {tape_shifted_left_0}");
                        // }
                        // println!(
                        //     "Step {}: Long jump = {j} u32 = {} bits",
                        //     self.num_steps,
                        //     j * 32
                        // );
                        // shift out high bit after moving 32 bit
                        let tape_shifted_left_1 = (self.tape_shifted >> 64) as u32;
                        self.tape_long[self.tl_pos + 1] = tape_shifted_left_1;
                        self.tl_pos = p - 3;
                        // println!("before {}", self.tape_shifted.to_binary_split_string());
                        self.tape_shifted = if self.tr_field_id & 1 == 0 {
                            0
                        } else {
                            CLEAR_LOW63_00BITS_U128
                        };
                        // println!("filled {}", self.tape_shifted.to_binary_split_string());
                        self.pos_middle = HIGH32_SWITCH_U128;
                        self.num_steps += j * 32 - 1;
                        // shift in low bits (low part is already cleared)
                        self.tape_shifted |= (self.tape_long[self.tl_pos + 3] as u128) << 32;
                        // println!("fill 2 {}", self.tape_shifted.to_binary_split_string());
                        long_jump = true;
                    }
                    //                         else {
                    //                             self.pos_middle += jump;
                    //
                    //                             // shift tape
                    //                             // self.set_current_symbol();
                    //                             self.tape_shifted <<= jump;
                    //                             self.num_steps += jump as StepTypeBig - 1;
                    //                         }
                }
                if !long_jump {
                    if self.pos_middle + jump > HIGH32_SWITCH_U128 {
                        jump = HIGH32_SWITCH_U128 - self.pos_middle;
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  jump right adjusted {jump}");
                    }
                    self.pos_middle += jump;

                    // shift tape
                    // self.set_current_symbol();
                    self.tape_shifted <<= jump;
                    self.num_steps += jump as StepTypeBig - 1;
                }
                // #[cfg(feature = "bb_enable_html_reports")]
                // if self.write_html_step_limit > 0 {
                //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                //     let s = format!(
                //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                //         self.num_steps,
                //         self.tl_pos,
                //         self.tape_long[tl_pos_min_1],
                //         self.tape_long[self.tl_pos],
                //         self.tape_long[self.tl_pos + 1],
                //         self.tape_long[self.tl_pos + 2],
                //         self.tape_long[self.tl_pos + 3],
                //         self.tape_long[self.tl_pos + 4],
                //     );
                //     self.write_html_p(&s);
                // }
            } else {
                self.pos_middle += 1;

                // shift tape
                self.set_current_symbol();
                self.tape_shifted <<= 1;
            }

            self.shift_tape_long_head_dir_right()
        } else {
            // normal shift LEFT -> tape moves left

            // Check if self referencing, which speeds up the shift greatly.
            if self.tr.array_id() == self.tr_field_id {
                let mut jump = self.count_left(self.tr_field_id & 1) as usize;
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                println!("  jump left {jump}");
                // The content is either always 0 or always 1, which makes looping over multiple u32 fields easy
                // Interestingly, the version with the long_jump logic runs faster.
                let mut long_jump = false;
                if jump == 33 && LOW32_SWITCH_U128 - 1 + jump == self.pos_middle {
                    // check further for larger jump
                    // compare depending on symbol
                    let v32 = if self.tr_field_id & 1 == 0 {
                        0
                    } else {
                        u32::MAX
                    };
                    // head goes left, tape shifts right
                    // tl_pos + 1 is known required value v32, because that is what count_left just tested
                    let mut p = self.tl_pos;
                    let mut j = 1;
                    while p >= self.tl_low_bound && self.tape_long[p] == v32 {
                        p -= 1;
                        j += 1;
                    }
                    // j is one more as the first one is already checked with count_right
                    if j >= 2 {
                        // if tape_shifted_left_0 != v32 {
                        //     println!("Not v32 {v32} but {tape_shifted_left_0}");
                        // }
                        // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        // println!(
                        //     "Step {}: Long jump = {j} u32 = {} bits",
                        //     self.num_steps,
                        //     j * 32
                        // );
                        // shift out low bit after moving 32 bit
                        let tape_shifted_left_2 = (self.tape_shifted >> 32) as u32;
                        self.tape_long[self.tl_pos + 2] = tape_shifted_left_2;
                        self.tl_pos = p;
                        // println!("before {}", self.tape_shifted.to_binary_split_string());
                        self.tape_shifted = if self.tr_field_id & 1 == 0 {
                            0
                        } else {
                            u64::MAX as u128
                        };
                        // println!("filled {}", self.tape_shifted.to_binary_split_string());
                        self.pos_middle = LOW32_SWITCH_U128;
                        self.num_steps += j * 32 - 1;
                        // shift in high bits (high part is already cleared)
                        self.tape_shifted |=
                            (self.tape_long[self.tl_pos] as u128) << TAPE_SIZE_HALF_128;
                        // println!("fill 2 {}", self.tape_shifted.to_binary_split_string());
                        long_jump = true;
                    }
                    //                         else {
                    //                             self.pos_middle += jump;
                    //
                    //                             // shift tape
                    //                             // self.set_current_symbol();
                    //                             self.tape_shifted <<= jump;
                    //                             self.num_steps += jump as StepTypeBig - 1;
                    //                         }
                }
                if !long_jump {
                    if self.pos_middle < LOW32_SWITCH_U128 + jump {
                        jump = self.pos_middle - LOW32_SWITCH_U128;
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  jump left adjusted {jump}");
                    }
                    self.pos_middle -= jump;

                    // self.set_current_symbol();
                    // shift tape
                    self.tape_shifted >>= jump;
                    self.num_steps += jump as StepTypeBig - 1;
                }
                // #[cfg(feature = "bb_enable_html_reports")]
                // if self.write_html_step_limit > 0 && self.num_steps < self.write_html_step_limit
                // {
                //     let tl_pos_min_1 = if self.tl_pos == 0 { 0 } else { self.tl_pos - 1 };
                //     let s = format!(
                //         "num_steps: {}, t pos {}, tl: {}, [{},{},{},{}], {}",
                //         self.num_steps,
                //         self.tl_pos,
                //         self.tape_long[tl_pos_min_1],
                //         self.tape_long[self.tl_pos],
                //         self.tape_long[self.tl_pos + 1],
                //         self.tape_long[self.tl_pos + 2],
                //         self.tape_long[self.tl_pos + 3],
                //         self.tape_long[self.tl_pos + 4],
                //     );
                //     self.write_html_p(&s);
                // }
            } else {
                self.pos_middle -= 1;

                self.set_current_symbol();
                // shift tape
                self.tape_shifted >>= 1;
            }
            self.shift_tape_long_head_dir_left()
        };
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            if self.num_steps % 100 == 0 {
                println!();
            }
            println!("{}", self.step_to_string());
        }
        #[cfg(feature = "bb_enable_html_reports")]
        self.write_step_html();

        shift_ok
    }

    #[inline(always)]
    fn shift_tape_long_head_dir_left(&mut self) -> bool {
        // normal shift LEFT -> tape moves left
        if self.pos_middle == LOW32_SWITCH_U128 {
            // save high bytes
            if !self.shift_pos_to_left_checked() {
                return false;
            }

            // The shift is left, so tape_shifted wanders right -> store low 32 bits.
            self.tape_long[self.tl_pos + 3] = self.tape_shifted as u32;

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            println!(
                "  LEFT  SAVE HIGH P{}-{}: tape wanders right -> {:?}",
                self.pos_middle,
                self.tl_pos,
                self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
            );

            self.pos_middle = MIDDLE_BIT_U128;

            // load high bytes
            self.tape_shifted = (self.tape_shifted & CLEAR_HIGH95_64BITS_U128)
                | ((self.tape_long[self.tl_pos + 1] as u128) << TAPE_SIZE_HALF_128);

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            {
                println!(
                    "  ALoad {}",
                    crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                );
                println!(
                    "  LEFT  LOAD HIGH P{}-{}: tape wanders right -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );
                print!("");
            }
        }

        true
    }

    #[inline(always)]
    fn shift_tape_long_head_dir_right(&mut self) -> bool {
        // normal shift RIGHT -> tape moves left

        if self.pos_middle == HIGH32_SWITCH_U128 {
            // save high bytes
            if !self.shift_pos_to_right_checked() {
                return false;
            }

            // The shift is right, so tape_shifted wanders left -> store high 32 bits.
            self.tape_long[self.tl_pos] = (self.tape_shifted >> TAPE_SIZE_FOURTH_UPPER_128) as u32;

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            println!(
                "  RIGHT SAVE HIGH P{}-{}: tape wanders left -> {:?}",
                self.pos_middle,
                self.tl_pos,
                self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
            );

            self.pos_middle = MIDDLE_BIT_U128;

            // Load low 32 bit
            self.tape_shifted = (self.tape_shifted & CLEAR_LOW63_32BITS_U128)
                | ((self.tape_long[self.tl_pos + 2] as u128) << 32);

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            {
                println!(
                    "  ALoad {}",
                    crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted)
                );
                println!(
                    "  RIGHT LOAD LOW  P{}-{}: tape wanders left -> {:?}",
                    self.pos_middle,
                    self.tl_pos,
                    self.tape_long.to_hex_string_range(TAPE_DISPLAY_RANGE_128)
                );
                print!("");
            }
        }

        true
    }

    // Creates
    #[cfg(feature = "bb_enable_html_reports")]
    pub fn write_html_file_start(&mut self, decider_id: &DeciderId, machine: &Machine) {
        if self.is_write_html_in_limit() {
            if let Some(path) = self.path.as_ref() {
                let (file, _f_name) = html::create_html_file_start(path, decider_id.name, &machine)
                    .expect("Html file could not be written");
                self.file = Some(file);
                // file_name = f_name;
                self.write_html_p(
                "Note: Here only the 128 Bit Tape is shown. Whenever the tape 'jumps' a few bytes \
                    the working area needed to be shifted or previously shifted out data is reloaded.<br> \
                    'tape_long' stores the remaining tape.",
            );
                if self
                    .transition_table
                    .eval_set_has_self_referencing_transition()
                {
                    self.write_html_p("Note: This machine has self-referencing transitions (e.g. Field A1: 1RA) \
                which leads to repeatedly calling itself in case of tape head reads 1. This is used to speed up the \
                decider by jumping over these repeated steps. Max jump is currently 32 steps.");
                }
            }
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn write_html_file_end(&mut self) {
        if self.file.is_some() {
            use num_format::ToFormattedString;
            let locale = crate::config::user_locale();
            if self.write_html_line_count >= self.write_html_line_limit {
                self.write_html_p(
                    format!(
                        "HTML Line Limit ({}) reached, total lines: {}.",
                        self.write_html_line_count.to_formatted_string(&locale),
                        self.num_steps.to_formatted_string(&locale)
                    )
                    .as_str(),
                );
            } else if self.write_html_line_count < self.num_steps {
                let p = ((self.write_html_line_count as f64 / self.num_steps as f64) * 1000.0)
                    .round()
                    / 100.0;
                self.write_html_p(
                    format!(
                        "Steps executed (single step or step jump): {} of {} = {p} %.",
                        self.write_html_line_count.to_formatted_string(&locale),
                        self.num_steps.to_formatted_string(&locale)
                    )
                    .as_str(),
                );
            }
            if self.num_steps >= self.write_html_step_limit {
                self.write_html_p(
                    format!(
                        "HTML Step Limit ({}) reached, total steps: {}.",
                        self.write_html_step_limit.to_formatted_string(&locale),
                        self.num_steps.to_formatted_string(&locale)
                    )
                    .as_str(),
                );
            }
            let text = format!("{}", self.status);
            self.write_html_p(&text);
            if let Some(file) = self.file.as_mut() {
                crate::html::write_file_end(file).expect("Html file could not be written")
            }
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn write_html_p(&mut self, text: &str) {
        if let Some(file) = self.file.as_mut() {
            crate::html::write_html_p(file, text);
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    pub fn write_step_html(&mut self) {
        if self.is_write_html_in_limit() {
            self.write_html_line_count += 1;
            crate::html::write_step_html_128(
                self.file.as_mut().unwrap(),
                self.num_steps as usize,
                self.tr_field_id,
                &self.tr,
                self.tape_shifted,
            );
        }
    }

    /// Debug info on current step
    pub fn step_to_string(&self) -> String {
        format!(
            "Step {:3} {}: P{}-{} {} Next {}{}",
            self.num_steps,
            self.tr,
            self.tl_pos,
            self.pos_middle,
            crate::tape_utils::U128Ext::to_binary_split_string(&self.tape_shifted),
            // self.get_tape_size(),
            self.tr.state_to_char(),
            self.get_current_symbol(),
        )
    }
}

impl Display for DeciderData128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO other fields
        write!(f, "{}", self.step_to_string(),)
    }
}
