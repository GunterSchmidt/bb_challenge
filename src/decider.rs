pub mod decider_bouncer_128;
// pub mod decider_bouncer_128_speed_up;
// pub mod decider_bouncer_apex;
pub mod pre_decider;
// // pub mod decider_bouncer_v1; old decider with different logic, may contain some re-usable code
pub mod decider_cycler;
pub mod decider_data_128;
// pub mod decider_data_apex;
// pub mod decider_data_compact;
pub mod decider_data_long;
pub mod decider_engine;
// pub mod decider_hold_compact;
pub mod decider_hold_long;
pub mod decider_result;
pub mod decider_result_worker;
pub mod step_record;

use std::{fmt::Display, sync::Arc, time::Duration};

use crate::{
    config::Config,
    decider::{
        decider_bouncer_128::DeciderBouncer128,
        decider_cycler::DeciderCycler,
        decider_hold_long::DeciderHoldLong,
        decider_result::{
            BatchData, DeciderResultStats, EndReason, PreDeciderCount, ResultUnitEndReason,
        },
        decider_result_worker::FnResultWorker,
        pre_decider::{run_pre_decider_simple, run_pre_decider_strict, PreDeciderRun},
    },
    machine_binary::MachineBinary,
    machine_info::MachineInfo,
    status::MachineStatus,
};
// use crate::{
//     decider::{
//         decider_bouncer_128::DeciderBouncer128,
//         decider_cycler::DeciderCycler,
//         decider_hold_long_v3::DeciderHoldLong,
//         decider_result::{
//             BatchData, DeciderResultStats, EndReason, PreDeciderCount, ResultUnitEndReason,
//         },
//         decider_result_worker::FnResultWorker,
//     },
//     machine_id::MachineId,
// };

// Deciders in this library
pub const DECIDER_HOLD_ID: DeciderId = DeciderId {
    id: 10,
    name: "Decider Hold",
    sub_dir: "hold",
};
pub const DECIDER_CYCLER_ID: DeciderId = DeciderId {
    id: 20,
    name: "Decider Cycler",
    sub_dir: "cycler",
};
pub const DECIDER_CYCLER_LONG_ID: DeciderId = DeciderId {
    id: 21,
    name: "Decider Cycler Long",
    sub_dir: "cycler_long",
};
pub const DECIDER_BOUNCER_ID: DeciderId = DeciderId {
    id: 20,
    name: "Decider Bouncer",
    sub_dir: "bouncer",
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

/// These are the provided deciders. This library should enable you to write your own decider.
pub enum DeciderStandard {
    // BouncerV1,
    Bouncer128,
    Cycler,
    Hold,
}

impl DeciderStandard {
    pub fn decider_caller(&self) -> DeciderCaller<'_> {
        match self {
            // DeciderStandard::BouncerV1 => {
            //     DeciderCaller::new(&DECIDER_BOUNCER_ID, DeciderBouncerV1::decider_run_batch)
            // }
            DeciderStandard::Bouncer128 => {
                DeciderCaller::new(&DECIDER_BOUNCER_ID, DeciderBouncer128::decider_run_batch)
            }
            DeciderStandard::Cycler => {
                DeciderCaller::new(&DECIDER_CYCLER_ID, DeciderCycler::decider_run_batch)
            }
            DeciderStandard::Hold => {
                DeciderCaller::new(&DECIDER_HOLD_ID, DeciderHoldLong::decider_run_batch)
            }
        }
    }

    pub fn decider_config<'a>(&self, config: &'a Config) -> DeciderConfig<'a> {
        match self {
            // DeciderStandard::BouncerV1 => DeciderConfig::new(
            //     &DECIDER_BOUNCER_ID,
            //     DeciderBouncerV1::decider_run_batch,
            //     config,
            // ),
            DeciderStandard::Bouncer128 => DeciderConfig::new(
                &DECIDER_BOUNCER_ID,
                DeciderBouncer128::decider_run_batch,
                config,
            ),
            DeciderStandard::Cycler => {
                DeciderConfig::new(&DECIDER_CYCLER_ID, DeciderCycler::decider_run_batch, config)
            }
            DeciderStandard::Hold => {
                DeciderConfig::new(&DECIDER_HOLD_ID, DeciderHoldLong::decider_run_batch, config)
            }
        }
    }
}

/// This struct defines the call to the decider function and its name.
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

/// This struct is used to chain the deciders, e.g. cycler low step count, bouncer, then cycler higher step count, then hold.
#[derive(Debug, Clone)]
pub struct DeciderConfig<'a> {
    decider_id: &'a DeciderId,
    f_decider_run_batch: FnDeciderRunBatchV2,
    pub fo_result_worker: Option<FnResultWorker>,
    config: Arc<&'a Config>,
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
            config: Arc::new(config),
        }
    }

    pub fn new_caller(decider_caller: &'a DeciderCaller, config: &'a Config) -> Self {
        Self {
            decider_id: decider_caller.decider_id,
            f_decider_run_batch: decider_caller.f_decider,
            fo_result_worker: None,
            config: Arc::new(config),
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
            config: Arc::new(config),
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
        *self.config
    }

    pub fn config_clone(&self) -> Arc<&'a Config> {
        Arc::clone(&self.config)
    }

    pub fn decider_id(&self) -> &DeciderId {
        self.decider_id
    }
}

/// Decider identification. As only the function to run the decider is passed, the id can not be requested
/// and needs to be part of the DeciderConfig.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeciderId {
    pub id: usize,
    pub name: &'static str,
    pub sub_dir: &'static str,
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

    /// Returns the result of this decider for one single machine. \
    /// Each run must clear self variables as the decider is re-used for all machines (in a batch).
    fn decide_machine(&mut self, machine: &MachineBinary) -> MachineStatus;

    /// Allows to test a single machine. This is just a convenience function, where a decider
    /// is created and one machine is run. This causes more overhead than setting up the decider once
    /// and use it for multiple machines.
    fn decide_single_machine(machine: &MachineBinary, config: &Config) -> MachineStatus;

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason;
}

#[inline]
pub fn decider_generic_run_batch(
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
        PreDeciderRun::RunNormalForward => {
            for machine in batch_data.machines.iter() {
                let mut status = run_pre_decider_simple(machine);
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
                let mut status = run_pre_decider_strict(machine);
                if status == MachineStatus::NoDecision {
                    status = decider.decide_machine(machine);
                }
                // This part is identical for all branches
                // match_status(status, batch_data, machine, limit_decided);
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

// Works, but even with inline 20-30% performance decrease
// #[inline(always)]
// fn match_status(
//     status: MachineStatus,
//     batch_data: &mut BatchData,
//     machine: &Machine,
//     limit_decided: usize,
// ) {
//     match status {
//         MachineStatus::Undecided(_, _, _) => {
//             batch_data.machines_undecided.machines.push(*machine);
//             batch_data.machines_undecided.states.push(status);
//         }
//         _ => {
//             if limit_decided > 0 && batch_data.machines_decided.machines.len() < limit_decided {
//                 batch_data.machines_decided.machines.push(*machine);
//                 batch_data.machines_decided.states.push(status);
//             }
//             batch_data.result_decided.add(machine, &status);
//         }
//     }
// }

#[derive(Default)]
pub struct ThreadResultDataProvider {
    pub batch_no: usize,
    pub machines: Vec<MachineBinary>,
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
