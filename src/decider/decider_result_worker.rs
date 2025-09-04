//! A result worker is a function which optionally can be called on the result of a decider.
//! It allows flexible functionality, e.g. only write html files for the undecided machines for later analysis.
// For performance reasons this should be done after each package and in a separate thread.

use std::fs::File;
use std::io::Write;

use chrono::{DateTime, Local, Utc};

use crate::{
    config::{Config, PATH_DATA},
    decider::decider_result::{BatchData, BatchResult, EndReason},
    status::{MachineStatus, UndecidedReason},
};

pub type FnResultWorker = fn(&mut BatchData) -> ResultWorker;
pub type ResultWorker = std::result::Result<(), EndReason>;
pub type ResultString = std::result::Result<(), String>;

pub fn save_machines_undecided(batch_result: &BatchResult, config: &Config) -> ResultWorker {
    let machine_infos = batch_result.machines_undecided.to_machine_info();

    let time_string = if config.use_local_time() {
        let datetime_local: DateTime<Local> = config.creation_time().into();
        datetime_local.format("%Y%m%d_%H%M%S").to_string()
    } else {
        let datetime_utc: DateTime<Utc> = config.creation_time().into();
        datetime_utc.format("%Y%m%d_%H%M%S").to_string()
    };

    // thread::spawn(move || {
    let path = PATH_DATA;
    let file_name =
        time_string.to_owned() + "_undecided_step_limit " + &batch_result.decider_name + ".txt";
    let mut file_step_limit = open_file_for_append(path, &file_name)?;
    let file_name =
        time_string.to_owned() + "_undecided_tape_bound " + &batch_result.decider_name + ".txt";
    let mut file_tape_bound = open_file_for_append(path, &file_name)?;
    let file_name =
        time_string.to_owned() + "_undecided_other " + &batch_result.decider_name + ".txt";
    // let mut file_other = open_file_for_append(path, &file_name)?;
    let mut file_other = None;

    // save machines
    for mi in machine_infos.iter().take(500) {
        match mi.status() {
            MachineStatus::Undecided(undecided_reason, _, _) => match undecided_reason {
                UndecidedReason::TapeLimitLeftBoundReached
                | UndecidedReason::TapeLimitRightBoundReached => {
                    writeln!(file_tape_bound, "{}: {}", batch_result.decider_name, mi)?
                }
                UndecidedReason::NoSinusRhythmIdentified => todo!(),
                UndecidedReason::StepLimit => {
                    writeln!(file_step_limit, "{}: {}", batch_result.decider_name, mi)?
                }
                UndecidedReason::TapeSizeLimit => todo!(),
                UndecidedReason::Undefined => todo!(),
                _ => {
                    if file_other.is_none() {
                        file_other = Some(open_file_for_append(path, &file_name)?);
                    }
                    writeln!(
                        file_other.as_ref().unwrap(),
                        "{}: {}",
                        batch_result.decider_name,
                        mi
                    )?
                }
            },
            _ => panic!("Must not happen"),
        }
    }
    // });

    Ok(())
}

fn open_file_for_append(path: &str, file_name: &str) -> Result<File, EndReason> {
    // open file for append
    let file_path = path.to_owned() + file_name;
    let r = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_path);

    match r {
        Ok(file) => Ok(file),
        Err(e) => Err(EndReason::Error(
            0,
            e.to_string() + ":" + file_path.as_str(),
        )),
    }
}

// pub fn save_machines_undecided_to_file(machine_infos: &[MachineInfo]) -> ResultWorker {
//     let path = PATH_DATA;
//     let file_name = "undecided_machines.txt";
//     let file_path = path.to_owned() + file_name;

//     // open file for append
//     let mut file = std::fs::OpenOptions::new()
//         .append(true)
//         .create(true)
//         .open(file_path)?;

//     Ok(())
// }

pub fn print_batch_result(batch_result: &BatchResult, _config: &Config) -> ResultWorker {
    let machine_infos = batch_result.machines_undecided.to_machine_info();

    // thread::spawn(move || {
    for mi in machine_infos.iter().take(500) {
        println!("{}: {}", batch_result.decider_name, mi);
    }
    // });

    Ok(())
}

// #[derive(Debug)]
// pub struct ResultWorkerError {
//     machine_id: IdBig,
//     message: String,
// }
//
// impl std::error::Error for ResultWorkerError {}
//
// // Implement std::convert::From for AppError; from io::Error
// impl From<std::io::Error> for ResultWorkerError {
//     fn from(error: std::io::Error) -> Self {
//         ResultWorkerError {
//             machine_id: 0,
//             message: error.to_string(),
//         }
//     }
// }
//
// impl Display for ResultWorkerError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "Machine Id: {}, Error: {}",
//             self.machine_id, self.message
//         )
//     }
// }
