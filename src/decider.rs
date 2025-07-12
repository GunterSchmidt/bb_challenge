use std::{fmt::Display, time::Duration};

use crate::{
    config::Config,
    decider_bouncer::DeciderBouncer,
    decider_result::{BatchData, DeciderResultStats, EndReason, PreDeciderCount},
    decider_result_worker::FnResultWorker,
    machine::Machine,
    machine_info::MachineInfo,
    pre_decider::{run_pre_decider_simple, run_pre_decider_strict, PreDeciderRun},
    status::MachineStatus,
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
                crate::decider_cycler_v5::DeciderCycler::decider_run_batch,
            ),
            DeciderStandard::Hold => DeciderCaller::new(
                &DECIDER_HOLD_ID,
                crate::decider_hold_u128_long_v2::DeciderHoldU128Long::decider_run_batch,
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
                crate::decider_cycler_v5::DeciderCycler::decider_run_batch,
                config,
            ),
            DeciderStandard::Hold => DeciderConfig::new(
                &DECIDER_HOLD_ID,
                crate::decider_hold_u128_long_v2::DeciderHoldU128Long::decider_run_batch,
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
