//! This decider just runs all steps until either a halt or limit is encountered. \
//! In case of the self-ref speed-up not all steps are executed, as the self-referencing transition repetitions are omitted.  
//! For BB5_MAX only 91,021 of 47,176,870 steps = 0.02 % are actually executed. The missing steps are calculated. This makes the
//! execution of the halt decider really fast, with just about 2 ms instead of 150 ms for BB5_MAX. \
//! # Example:
//! Here one can see 1LD and 1RB are repeated multiple time. 1LD does not change the status,
//! as it is the transition for D1, as such is self-referencing. \
//! Also the tape does not change, so the symbols do not need to be written.
//! If only the search for Halt is relevant, then these steps can be skipped. \
//! Step   184 B0 1RC: 000000000000000001111111_11111111\*01001001_001100000000000000000000 \
//! Step   185 C0 1RD: 000000000000000011111111_11111111\*10010010_011000000000000000000000 \
//! Step   186 D1 1LD: 000000000000000001111111_11111111\*11001001_001100000000000000000000 \
//! Step   187 D1 1LD: 000000000000000000111111_11111111\*11100100_100110000000000000000000 \
//! Step   188 D1 1LD: 000000000000000000011111_11111111\*11110010_010011000000000000000000 \
//! Step   189 D1 1LD: 000000000000000000001111_11111111\*11111001_001001100000000000000000 \
//! Step   190 D1 1LD: 000000000000000000000111_11111111\*11111100_100100110000000000000000 \
//! Step   191 D1 1LD: 000000000000000000000011_11111111\*11111110_010010011000000000000000 \
//! Step   192 D1 1LD: 000000000000000000000001_11111111\*11111111_001001001100000000000000 \
//! Step   193 D1 1LD: 000000000000000000000000_11111111\*11111111_100100100110000000000000 \
//! Step   194 D1 1LD: 000000000000000000000000_01111111\*11111111_110010010011000000000000 \
//! Step   195 D1 1LD: 000000000000000000000000_00111111\*11111111_111001001001100000000000 \
//! Step   196 D1 1LD: 000000000000000000000000_00011111\*11111111_111100100100110000000000 \
//! Step   197 D1 1LD: 000000000000000000000000_00001111\*11111111_111110010010011000000000 \
//! Step   198 D1 1LD: 000000000000000000000000_00000111\*11111111_111111001001001100000000 \
//! Step   199 D1 1LD: 000000000000000000000000_00000011\*11111111_111111100100100110000000 \
//! Step   200 D1 1LD: 000000000000000000000000_00000001\*11111111_111111110010010011000000 \
//! Step   201 D1 1LD: 000000000000000000000000_00000000\*11111111_111111111001001001100000 \
//! Step   202 D1 1LD: 000000000000000000000000_00000000\*01111111_111111111100100100110000 \
//! Step   203 D0 1LA: 000000000000000000000000_00000000\*01111111_111111111110010010011000 \
//! Step   204 A0 1RB: 000000000000000000000000_00000001\*11111111_111111111100100100110000 \
//! Step   205 B1 1RB: 000000000000000000000000_00000011\*11111111_111111111001001001100000 \
//! Step   206 B1 1RB: 000000000000000000000000_00000111\*11111111_111111110010010011000000 \
//! Step   207 B1 1RB: 000000000000000000000000_00001111\*11111111_111111100100100110000000 \
//! Step   208 B1 1RB: 000000000000000000000000_00011111\*11111111_111111001001001100000000 \
//! Step   209 B1 1RB: 000000000000000000000000_00111111\*11111111_111110010010011000000000 \
//! Step   210 B1 1RB: 000000000000000000000000_01111111\*11111111_111100100100110000000000 \
//! Step   211 B1 1RB: 000000000000000000000000_11111111\*11111111_111001001001100000000000 \
//! Step   212 B1 1RB: 000000000000000000000001_11111111\*11111111_110010010011000000000000 \
//! Step   213 B1 1RB: 000000000000000000000011_11111111\*11111111_100100100110000000000000 \
//! Step   214 B1 1RB: 000000000000000000000111_11111111\*11111111_001001001100000000000000 \
//! Step   215 B1 1RB: 000000000000000000001111_11111111\*11111110_010010011000000000000000 \
//! Step   216 B1 1RB: 000000000000000000011111_11111111\*11111100_100100110000000000000000 \
//! Step   217 B1 1RB: 000000000000000000111111_11111111\*11111001_001001100000000000000000 \
//! Step   218 B1 1RB: 000000000000000001111111_11111111\*11110010_010011000000000000000000 \
//! Step   219 B1 1RB: 000000000000000011111111_11111111\*11100100_100110000000000000000000 \
//! Step   220 B1 1RB: 000000000000000111111111_11111111\*11001001_001100000000000000000000 \
//! Step   221 B1 1RB: 000000000000001111111111_11111111\*10010010_011000000000000000000000 \
//! Step   222 B1 1RB: 000000000000011111111111_11111111\*00100100_110000000000000000000000 \
//! Step   223 B0 1RC: 000000000000111111111111_11111111\*01001001_100000000000000000000000 \
//! Step   224 C0 1RD: 000000000001111111111111_11111111\*10010011_000000000000000000000000 \
//! \
//! So instead this is executed: \
//! Here between 185 and 202 16 steps are skipped. \
//! Step   184 B0 1RC: 000000000000000001111111_11111111\*01001001_001100000000000000000000 \
//! Step   **185** C0 1RD: 000000000000000011111111_11111111\*10010010_011000000000000000000000 \
//! Step   **202** D1 1LD: 000000000000000000000000_00000000\*01111111_111111111100100100110000 \
//! Step   203 D0 1LA: 000000000000000000000000_00000000\*01111111_111111111110010010011000 \
//! Step   **204** A0 1RB: 000000000000000000000000_00000001\*11111111_111111111100100100110000 \
//! Step   **222** B1 1RB: 000000000000011111111111_11111111\*00100100_110000000000000000000000 \
//! Step   223 B0 1RC: 000000000000111111111111_11111111\*01001001_100000000000000000000000 \
//! Step   224 C0 1RD: 000000000001111111111111_11111111\*10010011_000000000000000000000000 \
//! \
//! Later the number of skipped steps can be very large, here more than 12.000 steps are skipped: \ \
//! Step 47152294 D0 1LA: 0000000000000000_000000000000000000000000_00000000\*01111111_111111111111111111111111_111111111111111 \
//! Step 47152295 A0 1RB: 0000000000000000_000000000000000000000000_00000001\*11111111_111111111111111111111111_111111111111111 \
//! Step **47152344** B1 1RB: 1111111111111111_111111111111111111111111_11111111\*11111111_111111111111111111111111_000000000000000 \
//! Step **47164568** B1 1RB: 1111111111111111_111111111111111111111111_11111111\*11111111_111000000000000000000000_000000000000000 \
//! Step 47164579 B1 1RB: 1111111111111111_111111111111111111111111_11111111\*00000000_000000000000000000000000_000000000000000 \
//! Step 47164580 B0 1RC: 1111111111111111_111111111111111111111111_11111111\*00000000_000000000000000000000000_000000000000000 \
//! Step 47164581 C0 1RD: 1111111111111111_111111111111111111111111_11111111\*00000000_000000000000000000000000_000000000000000 \

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

use crate::machine_binary::MachineId;
use crate::{config::Config, status::MachineStatus};
use crate::{
    decider::{
        self,
        decider_data_long::DeciderDataLong,
        decider_result::{BatchData, ResultUnitEndReason},
        Decider, DECIDER_HALT_ID,
    },
    machine_binary::NotableMachineBinary,
};

/// This decider runs on a 128-Bit number and moves data out to a long tape (Vec). \
/// The tape is not limited in size other than Vec memory limitations.
/// # Usage
/// - Set step_limit and tape_size_limit in [Config]
/// - Single machine: run [Self::decide_single_machine], see the tests for this.
/// - Batch:  run [Self::decider_run_batch]
// TODO machine 1RB---_1LC0RB_0LC1RB holds after 25 steps, why?
// TODO Longer jump if multiple u32 in tape_long are FFFF
// TODO Multiple repeating steps, e.g 3 on 001
// TODO version with output tape, visualize
// TODO performance html: keep 1000 lines in memory, then write
// TO DO many steps: stop after limit, but write last 1000 lines (this is difficult without creating the lines anyway)
// TO DO speedup u64 than handover? Probably only very small gain
// TODO Find self-ref cycle, e.g. ID_29439_1RB0RZ_0RC0RA_0LC1RD_1LE1RA_1RD0LD:
// - A0 1RB -> B1 0RA: check 63 0 and 62 1 and right repeat 01 (x*0101) 10101010←01010101_010101010101010101010101_01010101010101010101010100000000
// - D0 1LE -> E1 0LD: check 63 1 and 64 0 and left repeat 01 (01010*1) 011101010101010101010101_01010101→00101010
// BB5_MAX:
// - A1 1LC -> C1 0LE -> E1 0LA: check 63 1, 64 1 and 65 1 and left repeat 1 (1111*1) 00000000000000001111111111111111_111111111111111111111111_11111111→10010010
// This is the same as decider_halt_u128_long_v2 only with split and moved functionality to DeciderData128. May have an insignificant performance loss.
pub struct DeciderHaltLong {
    data: DeciderDataLong,
}

impl DeciderHaltLong {
    pub fn new(config: &Config) -> Self {
        Self {
            data: DeciderDataLong::new(config),
        }
    }

    fn decide_machine_with_self_referencing_transition(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            if self.data.next_transition() {
                // is done
                return self.data.status;
            }

            if !self.data.update_tape_self_ref_speed_up() {
                return self.data.status;
            };
        }
    }

    /// Returns the [MachineStatus:DecidedHalt] with steps if steps were found within limits of tape and max steps. \
    /// This version has a long tape, so it is not restricted to the 128 bit range.
    /// This is not using the self reference speed-up and should only be used if those would mess up the tests.
    fn decide_machine_without_self_referencing_transitions(&mut self) -> MachineStatus {
        // loop over transitions to write tape
        loop {
            if self.data.next_transition() {
                // is done
                return self.data.status;
            }

            if !self.data.update_tape_single_step() {
                return self.data.status;
            };
        }
    }
}

impl Decider for DeciderHaltLong {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_HALT_ID
    }

    fn decide_machine(&mut self, machine: &MachineId) -> MachineStatus {
        self.data.clear();
        self.data.transition_table = *machine.machine();

        #[cfg(feature = "enable_html_reports")]
        self.data
            .write_html_file_start(Self::decider_id(), &machine);

        #[cfg(feature = "without_self_ref_acceleration")]
        let result_status = self.decide_machine_without_self_referencing_transitions();

        #[cfg(not(feature = "without_self_ref_acceleration"))]
        let result_status = if self
            .data
            .transition_table
            .has_self_referencing_transition_store_result()
        {
            self.decide_machine_with_self_referencing_transition()
        } else {
            self.decide_machine_without_self_referencing_transitions()
        };

        #[cfg(feature = "enable_html_reports")]
        self.data.write_html_file_end();

        result_status
    }

    fn decide_single_machine(machine: &MachineId, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch(decider, batch_data)
    }
}

impl Display for DeciderHaltLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let s = String::new();
        // println!("State: Undecided: Too many steps to left.");

        write!(f, "{}", self.data.step_to_string(),)
    }
}

pub fn test_decider_halt(tm_text_format: &str) {
    let machine = MachineId::try_from(tm_text_format).unwrap();
    // let config = Config::new_default(5);
    let config = Config::builder(machine.n_states())
        .write_html_file(true)
        .step_limit_decider_halt(50_000_000)
        .build();
    let check_result = DeciderHaltLong::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    // assert_eq!(check_result, MachineStatus::DecidedHalt(47176870));
}

pub fn test_decider_halt_applies_bb5_max() {
    let config = Config::new_default(5);
    // let config = Config::builder(5)
    //     .write_html_file(true)
    //     .write_html_step_limit(50_000_000)
    //     // .step_limit_halt(5_000_000)
    //     .build();
    // BB5 Max
    let machine = NotableMachineBinary::BB5Max.machine_id();
    let start = std::time::Instant::now();
    let check_result = DeciderHaltLong::decide_single_machine(&machine, &config);
    let duration = start.elapsed();
    println!("Duration: {duration:?}");
    println!("{}", check_result);
    assert_eq!(check_result, MachineStatus::DecidedHalt(47176870));
}

/// One test runs 50 mio steps, so turn off default = ["bb_debug"].
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_halt_long_applies_bb4_max() {
        // let config = Config::new_default(4);
        let config = Config::builder(4).write_html_file(true).build();

        // BB4 Max
        let machine = NotableMachineBinary::BB4Max.machine_id();
        let mut decider = DeciderHaltLong::new(&config);
        let check_result = decider.decide_machine(&machine);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHalt(107));
        let full = decider.data.status_full();
        println!("{}", full);
        assert_eq!(full, MachineStatus::DecidedHaltDetail(107, 128, 12));
    }

    #[test]
    /// This test runs 50 mio steps, so turn off default = ["bb_debug"].
    fn decider_halt_long_applies_bb5_max() {
        let config = Config::builder(5)
            // write html can be turned on since html lines are limited
            .write_html_file(true)
            .write_html_line_limit(100_000)
            .step_limit_decider_halt(50_000_000)
            .build();
        // BB5 Max
        let machine = NotableMachineBinary::BB5Max.machine_id();
        let check_result = DeciderHaltLong::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        assert_eq!(check_result, MachineStatus::DecidedHalt(47_176_870));
    }
}
