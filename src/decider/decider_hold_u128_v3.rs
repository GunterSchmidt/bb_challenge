//! This decider just runs all steps until either a hold or limit is encountered. \
//! This is just a test for 128-Bit without long tape and speed up.
use std::fmt::Display;

use crate::{
    config::Config,
    decider::{self, Decider, DECIDER_HOLD_ID},
    decider_data_128::DeciderData128,
    decider_result::BatchData,
    machine::Machine,
    status::MachineStatus,
    ResultUnitEndReason,
};

pub struct DeciderHoldU128 {
    data: DeciderData128,
    // machine id, just for debugging
    // machine_id: IdBig,
}

impl DeciderHoldU128 {
    pub fn new(config: &Config) -> Self {
        #[allow(unused_mut)]
        let mut decider = Self {
            data: DeciderData128::new(config),
            // machine_id: 0,
        };

        #[cfg(feature = "bb_enable_html_reports")]
        {
            decider
                .data
                .set_path_option(crate::html::get_html_path("hold", config));
        }

        decider
    }

    fn decide_machine_with_self_referencing_transition(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            self.data.next_transition();

            // check if done
            if self.data.is_done() {
                return self.data.status;
            }

            if !self.data.update_tape_self_ref_speed_up() {
                return self.data.status;
            };
        }
    }

    /// Returns the MachineStatus:Hold with steps if steps were found within limits of tape and max steps. \
    /// This version has a long tape, so it is not restricted to the 128 bit range.
    /// This is not using the self reference speed-up and should only be used if those would mess up the tests.
    fn decide_machine_without_self_referencing_transitions(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            self.data.next_transition();

            // check if done
            if self.data.is_done() {
                return self.data.status;
            }

            if !self.data.update_tape_single_step() {
                return self.data.status;
            };
        }
    }

    // /// Returns the given machine reference
    // pub fn machine(&self) -> &'a Machine {
    //     self.machine
    // }
}

impl Decider for DeciderHoldU128 {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_HOLD_ID
    }

    // tape_long_bits in machine?
    // TODO counter: longest loop
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        self.data.clear();
        // self.machine_id = machine.id();
        self.data.transition_table = *machine.transition_table();

        #[cfg(feature = "bb_enable_html_reports")]
        self.data.write_html_file_start(Self::decider_id(), machine);

        let result_status = if cfg!(feature = "bb_no_self_ref") {
            self.decide_machine_without_self_referencing_transitions()
        } else if self
            .data
            .transition_table
            .has_self_referencing_transition_store_result()
        {
            self.decide_machine_with_self_referencing_transition()
        } else {
            self.decide_machine_without_self_referencing_transitions()
        };

        #[cfg(feature = "bb_enable_html_reports")]
        self.data.write_html_file_end();

        result_status
    }

    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }

    // fn new_from_self(&self) -> Self {
    //     todo!()
    // }
}

impl Display for DeciderHoldU128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let s = String::new();
        // println!("State: Undecided: Too many steps to left.");

        write!(f, "{}", self.data.step_to_string(),)
    }
}

// pub fn test_decider_hold_u128_applies_bb4_max() {
//     let config = Config::new_default(4);
//     // BB4 Max
//     let machine_bb4_max = Machine::build_machine("BB4_MAX").unwrap();
//
//     let mut decider: DeciderU128Long<SubDeciderDummy> = DeciderU128Long::new(config);
//     let check_result = decider.decide_machine(&machine_bb4_max);
//     // println!("{}", check_result);
//
//     assert_eq!(check_result, MachineStatus::DecidedHolds(107));
// }

pub fn test_decider_hold_u128(tm_text_format: &str) {
    let machine = Machine::from_standard_tm_text_format(0, tm_text_format).unwrap();
    // let config = Config::new_default(5);
    let config = Config::builder(machine.n_states())
        .write_html_file(true)
        .step_limit_hold(50_000_000)
        .build();
    let check_result = DeciderHoldU128::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    // assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

pub fn test_decider_hold_u128_applies_bb5_max() {
    let config = Config::new_default(5);
    // let config = Config::builder(5)
    //     .write_html_file(true)
    //     .write_html_step_limit(50_000_000)
    //     // .step_limit_hold(5_000_000)
    //     .build();
    // BB5 Max
    let machine = Machine::build_machine("BB5_MAX").unwrap();
    let check_result = DeciderHoldU128::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_hold_u128_applies_bb4_max() {
        // let config = Config::new_default(4);
        let config = Config::builder(4).write_html_file(true).build();

        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let mut decider = DeciderHoldU128::new(&config);
        let check_result = decider.decide_machine(&machine);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
        let full = decider.data.status_full();
        println!("{}", full);
        assert_eq!(full, MachineStatus::DecidedHoldsDetail(107, 1, 12));
    }

    #[test]
    /// This test runs 50 mio steps, so turn off default = ["bb_debug"].
    fn decider_hold_u128_applies_bb5_max() {
        // let config = Config::new_default(5);
        let config = Config::builder(5)
            .write_html_file(true)
            .write_html_line_limit(100_000)
            .step_limit_hold(50_000_000)
            .build();
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let check_result = DeciderHoldU128::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        // assert_eq!(check_result, MachineStatus::DecidedHolds(47_176_870));
        assert_eq!(
            check_result,
            MachineStatus::Undecided(crate::status::UndecidedReason::TapeSizeLimit, 1337, 23)
        );
    }
}
