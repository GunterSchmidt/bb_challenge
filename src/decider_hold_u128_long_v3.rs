// TODO inner repeated cycle to move faster
// Sometimes the same rhythm repeats itself until the tape bit change
// Example BB5_MAX: Here A1-C1-E1 repeat as long there are 3 '1' on the tape and are replaced with 001:
// The simple speed-up would be to shift the tape by 3 each time. This is simpler regarding the update of the tape and long tape,
// but will be much less efficient.
// The idea is to count the ones, shift the tape in one big step and continue normally. This has massive implications on long_tape and
// loading of long tape. All cells of long tape would need to be updated.
// On very large jumps it might be interesting to not update long_tape at all, but maintain a range of long tape fields which repeat.
// In this case this would be a set of 3 u32-bit fields, as 3 bits are repeated, which will not make identical u32.
// Step  4128 B1 1RB: 11111111111111111111111111111111_111111111111111111111111_11111111*11111111_111111110000000000000000_00000000000000000000000000000000
// Step  4144 B1 1RB: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4145 B0 1RC: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4146 C0 1RD: 11111111111111111111111111111111_111111111111111111111111_11111111*00000000_000000000000000000000000_00000000000000000000000000000000
// Step  4147 D0 1LA: 01111111111111111111111111111111_111111111111111111111111_111111[11*1]1000000_000000000000000000000000_00000000000000000000000000000000
// Step  4148 A1 1LC: 00111111111111111111111111111111_111111111111111111111111_11111111*11100000_000000000000000000000000_00000000000000000000000000000000
// Step  4149 C1 0LE: 00011111111111111111111111111111_111111111111111111111111_11111111*10110000_000000000000000000000000_00000000000000000000000000000000
// Step  4150 E1 0LA: 00001111111111111111111111111111_111111111111111111111111_11111111*1[001]1000_000000000000000000000000_00000000000000000000000000000000
// Step  4151 A1 1LC: 00000111111111111111111111111111_111111111111111111111111_11111111*11001100_000000000000000000000000_00000000000000000000000000000000
// Step  4152 C1 0LE: 00000011111111111111111111111111_111111111111111111111111_11111111*10100110_000000000000000000000000_00000000000000000000000000000000
// Step  4153 E1 0LA: 00000001111111111111111111111111_111111111111111111111111_11111111*10010011_000000000000000000000000_00000000000000000000000000000000
// Step  4154 A1 1LC: 00000000111111111111111111111111_111111111111111111111111_11111111*11001001_100000000000000000000000_00000000000000000000000000000000
// Step  4155 C1 0LE: 00000000011111111111111111111111_111111111111111111111111_11111111*10100100_110000000000000000000000_00000000000000000000000000000000
// Step  4156 E1 0LA: 00000000001111111111111111111111_111111111111111111111111_11111111*10010010_011000000000000000000000_00000000000000000000000000000000
// Step  4157 A1 1LC: 00000000000111111111111111111111_111111111111111111111111_11111111*11001001_001100000000000000000000_00000000000000000000000000000000
// Step  4158 C1 0LE: 00000000000011111111111111111111_111111111111111111111111_11111111*10100100_100110000000000000000000_00000000000000000000000000000000
// Step  4159 E1 0LA: 00000000000001111111111111111111_111111111111111111111111_11111111*10010010_010011000000000000000000_00000000000000000000000000000000
// Step  4160 A1 1LC: 00000000000000111111111111111111_111111111111111111111111_11111111*11001001_001001100000000000000000_00000000000000000000000000000000
// Step  4161 C1 0LE: 00000000000000011111111111111111_111111111111111111111111_11111111*10100100_100100110000000000000000_00000000000000000000000000000000
// Step  4162 E1 0LA: 00000000000000001111111111111111_111111111111111111111111_11111111*10010010_010010011000000000000000_00000000000000000000000000000000
// ...
// Step  4261 E1 0LA: 00000000000000000000000000000000_000000000000000000000000_000000[11*1]0010010_010010010010010010010010_01001001001001001001001001001001
// Step  4262 A1 1LC: 00000000000000000000000000000000_000000000000000000000000_00000001*11001001_001001001001001001001001_00100100100100100100100100100100
// Step  4263 C1 0LE: 00000000000000000000000000000000_000000000000000000000000_00000000*10100100_100100100100100100100100_10010010010010010010010010010010
// Step  4264 E1 0LA: 00000000000000000000000000000000_000000000000000000000000_00000000*0[001]0010_010010010010010010010010_01001001001001001001001001001001
//   no more 111
// Step  4265 A0 1RB: 00000000000000000000000000000000_000000000000000000000000_00000001*00100100_100100100100100100100100_10010010010010010010010010010010
// Step  4266 B0 1RC: 00000000000000000000000000000000_000000000000000000000000_00000011*01001001_001001001001001001001001_00100100100100100100100100100100
// Step  4267 C0 1RD: 00000000000000000000000000000000_000000000000000000000000_00000111*10010010_010010010010010010010010_01001001001001001001001001001000
// Step  4271 D1 1LD: 00000000000000000000000000000000_000000000000000000000000_00000000*01111001_001001001001001001001001_00100100100100100100100100100100
// Step  4272 D0 1LA: 00000000000000000000000000000000_000000000000000000000000_00000000*01111100_100100100100100100100100_10010010010010010010010010010010
use std::fmt::Display;

#[cfg(all(debug_assertions, feature = "bb_debug"))]
use crate::tape_utils::{VecU32Ext, TAPE_DISPLAY_RANGE_128};
use crate::{
    config::Config,
    decider::{self, Decider, DeciderData128, DECIDER_HOLD_ID},
    decider_result::BatchData,
    machine::Machine,
    status::MachineStatus,
    ResultUnitEndReason,
};

/// This decider runs on a 128-Bit number and moves data out to a long tape (Vec). \
/// The tape is not limited in size other than Vec memory limitations.
/// Usage:
/// # Set step_limit and tape_size_limit in the machine to evaluate
/// # Create a new decider for a single machine
/// # Run run_check_hold to check if the machine holds
// TODO Longer jump if multiple u32 in tape_long are FFFF
// TODO Multiple repeating steps, e.g 3 on 001
// TODO version with output tape, visualize
// TODO performance html: keep 1000 lines in memory, then write
// TO DO many steps: stop after limit, but write last 1000 lines (this is difficult without creating the lines anyway)
// TO DO speedup u64 than handover? Probably only very small gain
// This is the same as decider_hold_u128_long_v2 only with split and moved functionality to DeciderData128. May have an insignificant performance loss.
pub struct DeciderHoldU128Long {
    data: DeciderData128,
    // machine id, just for debugging
    // machine_id: IdBig,
}

impl DeciderHoldU128Long {
    pub fn new(config: &Config) -> Self {
        #[allow(unused_mut)]
        let mut decider = Self {
            data: DeciderData128::new(config),
            // machine_id: 0,
        };

        #[cfg(feature = "bb_enable_html_reports")]
        {
            decider.data.path = crate::html::get_html_path("hold", config);
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

            self.data.update_tape_self_ref_speed_up();
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

            self.data.update_tape_single_step();
        }
    }

    // /// Returns the given machine reference
    // pub fn machine(&self) -> &'a Machine {
    //     self.machine
    // }
}

impl Decider for DeciderHoldU128Long {
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
            .eval_set_has_self_referencing_transition()
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

impl Display for DeciderHoldU128Long {
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

pub fn test_decider_hold_u128_applies_bb5_max() {
    let config = Config::new_default(5);
    // let config = Config::builder(5)
    //     .write_html_file(true)
    //     .write_html_step_limit(50_000_000)
    //     // .step_limit_hold(5_000_000)
    //     .build();
    // BB5 Max
    let machine = Machine::build_machine("BB5_MAX").unwrap();
    let check_result = DeciderHoldU128Long::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_hold_u128_applies_bb4_max() {
        let config = Config::new_default(4);
        // let config = Config::builder(4).write_html_file(true).build();

        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let check_result = DeciderHoldU128Long::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    /// This test runs 50 mio steps, so turn off default = ["bb_debug"].
    fn decider_hold_u128_applies_bb5_max() {
        let config = Config::new_default(5);
        // let config = Config::builder(5)
        //     .write_html_file(false)
        //     .write_html_step_limit(1_000_000)
        //     // .step_limit_hold(1_000_000)
        //     .build();
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let check_result = DeciderHoldU128Long::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHolds(47_176_870));
    }
}
