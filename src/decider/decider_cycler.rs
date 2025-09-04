//! This is the cycler decider (working on a 128-bit tape, with long_tape in background). \
//! It is a very effective decider and should run first with a small number of steps to eliminate most \
//! of the cyclers and machines which hold quickly (both are identified).
//! Example for BB4 with 6,975,757,441 machines. If only this decider is run with 150 step limit, then all
//! machines can be checked in less than 3 seconds, leaving only 65,530 undecided.
//! Most machines are eliminated by the pre-decider, leaving only 30,199,552 machines which need to be checked further.
//! This decider than classifies 10,758,178 as Hold and 19,375,844 as Cycler.
//! Of the 10,758,178 Hold machines, only 184 machines run more than 50 steps.
//! Of the 19,375,844 Cycler machines, only 4740 machines are not detected within 50 steps.
//! If the limit is set to 10,000 machines, then the runtime will be around 65 seconds (on my machine), \
//! which is a factor of >20. \
//! Additionally 27,488 Cyclers are found, eliminating one third more. But is also means < 0,2% of the machines take 95% of the time.\
//! Therefore it is faster, to run the Cycler with a small limit first, then run the Bouncer on the remaining undecided machines,
//! which will eliminate a high number of those. Then run the cycler again with a higher limit.
//! A reasonable size for first run limit is between 300 and 1500, the runtime does differ, but this is not much overall.
//! Limit 1,500: For BB5 the first 100,000,000,000 can be tested in about 34 seconds, leaving only 204,762 undecided. \
//! Limit 2,500: For BB5 the first 100,000,000,000 can be tested in about 40 seconds, leaving only 172,913 undecided \
//! Limit 5,000: For BB5 the first 100,000,000,000 can be tested in about 65 seconds, leaving only 132,196 undecided \
//! Limit 10,000: For BB5 the first 100,000,000,000 can be tested in about 145 seconds, leaving only 115,224 undecided \
//! Limit 25,000: For BB5 the first 100,000,000,000 can be tested in about 13 minutes, leaving only 110,727 undecided \
//! The runtimes largely depend on how many CPUs the system has, this is measured on an older 8 core (4 core / 4 Hyper-threading) notebook.
//! Limit 10,000: For BB5 the first 100,000,000,000 can be tested in about 75 seconds, leaving only 115,224 undecided on a faster 12 core (6/6) desktop system. \
//! Limit 50,000: For BB5 the first 100,000,000,000 can be tested in about 30 minutes, leaving only 110,267 undecided on a faster 12 core (6/6) desktop system. \
//! Therefore it is wiser to run the bouncer before attempting to catch the other cyclers.
//! Limit 10,000: For the bb_challenge file of 88,664,064 machines, 62,291,319 are identified as cyclers.
//! How it works: \
//! When run, every step is recorded (StepCycler) so repeating steps can be identified.
//! A map is created for all table fields which stores the steps which used this table field, \
//! e.g. A0 was used in step 0, 14, 28, 42 etc.
//! In this case when 28 is found, all steps will be compared between 0 to 14 and 14 to 28 and \
//! checked if each step is identical. \
//! If this is the case then also the tape will be compared. It needs to match for the \
//! relevant part, meaning all cells touched in this cycle will be compared.

// TODO cycle validation with 3rd and 4th cycle

#[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
use crate::tape_utils::U128Ext;
use crate::{
    config::{Config, StepTypeBig, StepTypeSmall, MAX_STATES},
    decider::{
        self,
        decider_data_long::DeciderDataLong,
        decider_result::{BatchData, ResultUnitEndReason},
        step_record::StepRecordU128,
        Decider, DECIDER_CYCLER_ID,
    },
    machine_binary::MachineBinary,
    status::{MachineStatus, NonHaltReason},
    // step_record::StepRecordU128,
    tape::tape_utils::{MIDDLE_BIT_U128, TAPE_SIZE_BIT_U128},
};

#[cfg(debug_assertions)]
const DEBUG_EXTRA: bool = false;
#[cfg(debug_assertions)]
const DEBUG_MIN_DISTANCE: usize = 75;

/// Initial capacity for step recorder. Not so relevant.
const MAX_INIT_CAPACITY: usize = 10_000;
/// Reduces number of checks. This relies on a cycle which always has one tape side 0.
const SEARCH_ONLY_0_SIDE_FROM: usize = 50;

#[derive(Debug)]
pub struct DeciderCycler {
    data: DeciderDataLong,
    /// Store all steps to do comparisons (test if a cycle is repeating)
    steps: Vec<StepRecordU128>,
    /// Stores the step ids (2 = 3rd step) for each field in the transition table. \
    /// (basically e.g. all steps for e.g. field 'B0' steps: 1 if A0 points to B, as step 1 then has state B and head symbol 0.)
    // TODO performance: extra differentiation for 0/1 at head position? The idea is, that the field cannot be identical if head read is different
    maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
}

impl DeciderCycler {
    pub fn new(config: &Config) -> Self {
        let cap = (config.step_limit_decider_cycler() as usize).min(MAX_INIT_CAPACITY);
        let mut decider = Self {
            data: DeciderDataLong::new(config),
            steps: Vec::with_capacity(cap),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(cap / 4)),
        };
        decider.data.step_limit = config.step_limit_decider_cycler();

        #[cfg(feature = "enable_html_reports")]
        {
            decider
                .data
                .set_path_option(crate::html::get_html_path("cycler", config));
        }

        decider
    }

    #[inline]
    fn clear(&mut self) {
        self.data.clear();
        self.steps.clear();
        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
    }
}

impl Decider for DeciderCycler {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_CYCLER_ID
    }

    fn decide_machine(&mut self, machine: &MachineBinary) -> MachineStatus {
        // #[cfg(debug_assertions)]
        // {
        //     if machine.id != DEBUG_MACHINE_NO {
        //         // return MachineStatus::NoDecision;
        //     }
        //     println!("\nDecider Cycle for {}", machine.to_string_without_status());
        // }
        // println!("Machine {}", machine);

        // if machine.id() >= 1_341_092 {
        //     print!("");
        // }

        #[cfg(feature = "enable_html_reports")]
        self.data.write_html_file_start(
            Self::decider_id(),
            &bb_challenge::machine_info::MachineInfo::from(machine),
        );

        // initialize decider
        self.clear();

        // Initialize transition with A0 as start
        let mut read_symbol_next;
        let mut tr_field_next = 2;

        // loop over transitions to write tape
        loop {
            // use previously identified field
            self.data.tr_field = tr_field_next;

            // store next step
            // map for each transition, which step went into it
            // maps: store step id leading to this
            self.maps_1d[self.data.tr_field].push(self.steps.len());
            let mut step = StepRecordU128::new(self.data.tr_field, 0, self.data.tape_shifted());
            self.data.tr = machine.transition(self.data.tr_field);
            step.direction = self.data.tr.direction();
            self.steps.push(step);

            // check if done
            if self.data.tr.is_halt() || self.steps.len() as StepTypeSmall >= self.data.step_limit()
            {
                self.data.step_no = self.steps.len() as StepTypeBig;
                if self.data.is_done() {
                    #[cfg(feature = "enable_html_reports")]
                    {
                        self.data.write_html_file_end();
                        // close the file so it can be renamed (not sure if necessary)
                        // self.file = None;

                        // html::rename_file_to_status(&self.data.path.unwrap(), &self.data.file_name.unwrap(), &ms);
                        self.data.rename_html_file_to_status();
                    }
                    return self.data.status;
                } else {
                    panic!("Logic error");
                }
            }

            #[cfg(feature = "enable_html_reports")]
            {
                // required because num_steps is not updated normally
                self.data.step_no = self.steps.len() as StepTypeBig;
            }
            if !self.data.update_tape_single_step() {
                return self.data.status;
            };

            // get next transition
            read_symbol_next = self.data.get_current_symbol();

            // print steps
            #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
            println!(
                "Step {:3}: {} {} Tape shifted after: {:032b} {:032b}, next {}{} {}",
                self.steps.len() - 1,
                self.steps.last().unwrap().field_id_to_string(),
                tr,
                (tape_shifted >> 64) as u64,
                tape_shifted as u64,
                tr.state_to_char(),
                read_symbol_next,
                machine.transition(tr.state_x2() + read_symbol_next),
            );

            // check endless cycle for multiple steps
            tr_field_next = self.data.tr.state_x2() + read_symbol_next;
            // must be repeated already and either side needs to be 0
            // This assumes, the tape is fluctuating around the start
            if self.maps_1d[tr_field_next].len() > 1
                && (self.steps.len() < SEARCH_ONLY_0_SIDE_FROM
                    || self.data.tape_shifted() as u64 == 0
                    || (self.data.tape_shifted() >> 64) as u64 == 0)
            {
                // TODO performance: Possibly one can skip the last x steps as the smaller cycles have been checked before; is that a valid hypothesis?
                'steps: for &step_id in self.maps_1d[tr_field_next][1..]
                    .iter()
                    // .skip(1) // slow
                    .rev()
                {
                    let distance = self.steps.len() - step_id;
                    // check if we have two repeated cycles
                    if distance > step_id {
                        // This case is not interesting
                        // #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        // {
                        //     let s = format!("  * Fail {step_id}: Min Distance");
                        //     println!("{s}");
                        //     #[cfg(feature = "enable_html_reports")]
                        //     let s = html::blanks(10) + &s;
                        //     self.write_html_p(&s);
                        // }
                        // step_id will get smaller, distance larger
                        break;
                    }

                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        let s = format!(
                            "  {} Endless cycle check: Step {step_id} with distance {distance}",
                            TransitionSymbol2::field_id_to_string(tr_field_next)
                        );
                        println!("{s}");
                        #[cfg(feature = "enable_html_reports")]
                        let s = html::blanks(10)
                            + &format!(
                                "  {} Endless cycle check: Step {} with distance {distance}",
                                step_id + 1,
                                TransitionSymbol2::field_id_to_string(tr_field_next)
                            );
                        self.write_html_p(&s);
                    }

                    // check cycle steps are identical
                    for (i, step) in self.steps.iter().enumerate().skip(step_id) {
                        if step.for_field_id != self.steps[i - distance].for_field_id {
                            #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                            {
                                let s = "  * Fail: Cycle steps different";
                                println!("{s}");
                                #[cfg(feature = "enable_html_reports")]
                                let s = html::blanks(10) + &s;
                                self.write_html_p(&s);
                            }
                            // not identical, try next distance
                            continue 'steps;
                        }
                    }

                    // Same, we found a cycle candidate!
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        println!(
                            "  *** Cycle candidate found: First step {}, distance {distance}!",
                            self.steps.len() - distance
                        );
                        if self.write_html {
                            let text = format!(
                                "  *** Cycle candidate found: First step {}, distance {distance}!",
                                self.steps.len() - distance + 1
                            );
                            self.write_html_p(&text);
                        }
                    }

                    let step_tape_before = self.steps[step_id].tape_before;

                    // #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    // {
                    //     println!("Step Tape     : {}", step_tape.to_binary_split_string());
                    //     println!("Tape shifted  : {}", tape_shifted.to_binary_split_string());
                    // }

                    // check if full tape is identical (this is not necessary, only relevant bytes)
                    // TODO requires comparison of long_tape
                    if step_tape_before == self.data.tape_shifted() {
                        // Same, we found a cycle!
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("*** Found Cycle (tape identical)!");
                        #[cfg(feature = "enable_html_reports")]
                        if self.data.is_write_html_in_limit() {
                            let text = format!(
                                "  Decided: Found Cycle (tape identical): Start {} and {}, length: {distance}", 
                                step_id-distance+1,
                                step_id+1
                            );
                            self.data.write_html_p(&text);
                        }
                        #[cfg(debug_assertions)]
                        if DEBUG_EXTRA && distance >= DEBUG_MIN_DISTANCE {
                            println!(
                                "cycle size = {}, current step = {}: M {}",
                                distance,
                                self.steps.len(),
                                machine
                            );
                        }
                        return MachineStatus::DecidedNonHalt(NonHaltReason::Cycler(
                            self.steps.len() as StepTypeSmall,
                            distance as StepTypeSmall,
                        ));
                    }

                    // identify affected bits in the cycle steps
                    let mut total_shift: isize = 0;
                    let mut max_r: isize = 0;
                    let mut min_l: isize = 0;
                    // add all steps including next step, because result bit is also relevant
                    for step in self.steps.iter().skip(step_id) {
                        total_shift += step.direction as isize;
                        if min_l > total_shift {
                            min_l = total_shift
                        };
                        if max_r < total_shift {
                            max_r = total_shift
                        };
                    }
                    // When shifted, eventually all bits on that side are used after x cycles, check all
                    #[allow(clippy::comparison_chain)]
                    if total_shift > 0 {
                        max_r = MIDDLE_BIT_U128 as isize // 31 / 63
                    } else if total_shift < 0 {
                        min_l = TAPE_SIZE_BIT_U128 as isize / -2 // -32 / -64
                    }

                    // extract relevant bits and compare (bits counted from right, starting with 0, middle is bit 31)
                    let start_bit = MIDDLE_BIT_U128 as isize - max_r;
                    let end_bit = MIDDLE_BIT_U128 as isize - min_l; // Inclusive
                    let num_bits = end_bit - start_bit + 1;
                    // Create the mask for the lowest 'num_bits' bits.
                    //    (1 << 10) gives 0b10000000000 (1 followed by 10 zeros)
                    //    Subtracting 1 gives 0b01111111111 (10 ones) -> 0x3FF in hex
                    if num_bits > 127 {
                        println!("{machine}");
                    }
                    let mask: u128 = ((1 << num_bits) - 1) << start_bit;
                    // #[cfg(feature = "bb_debug_cycler")]
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        // for (i, step) in self.steps.iter().enumerate().skip(step_id) {
                        //     let t = machine.transition(step.for_field_id);
                        //     println!(
                        //         "   Step {i:3}: {} {}: {}",
                        //         step.field_id_to_string(),
                        //         t,
                        //         step.tape_before.to_binary_split_string()
                        //     );
                        // }
                        println!(
                            "Step {step_id:3} before    : {}",
                            step_tape_before.to_binary_split_string()
                        );
                        println!(
                            "Step {:3} T shifted : {}",
                            self.steps.len(),
                            tape_shifted.to_binary_split_string()
                        );
                        println!("Mask               : {}", mask.to_binary_split_string());
                        println!(
                            "Step relevant      : {}",
                            (step_tape_before & mask).to_binary_split_string()
                        );
                        println!(
                            "Tape_sh relevant   : {}",
                            (tape_shifted & mask).to_binary_split_string()
                        );
                    }

                    // check if full tape is identical (this is not necessary, only relevant bytes)
                    if step_tape_before & mask == self.data.tape_shifted() & mask {
                        // Same, we found a cycle!
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("  *** Found Cycle with mask!");
                        #[cfg(feature = "enable_html_reports")]
                        if self.data.is_write_html_in_limit() {
                            let text =
                                format!("  Decided: Found Cycle (tape for relevant part identical): Start {} and {}, length: {distance}", step_id-distance+1,step_id+1);
                            self.data.write_html_p(&text);
                        }
                        #[cfg(debug_assertions)]
                        if DEBUG_EXTRA && distance >= DEBUG_MIN_DISTANCE {
                            println!(
                                "cycle size = {}, current step = {}: M {}",
                                distance,
                                self.steps.len(),
                                machine
                            );
                        }
                        return MachineStatus::DecidedNonHalt(NonHaltReason::Cycler(
                            self.steps.len() as StepTypeSmall,
                            distance as StepTypeSmall,
                        ));
                    } else {
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("  * Fail: Mask");
                    }
                }
            }
        }
    }

    // tape_long_bits in machine?
    // TODO counter: longest cycle

    fn decide_single_machine(machine: &MachineBinary, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn is_cycler(machine: &MachineBinary) -> bool {
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_cycler(5000)
            .build();
        let check_result = DeciderCycler::decide_single_machine(&machine, &config);
        if check_result.is_cycler() {
            true
        } else {
            println!("{}", check_result);
            // assert_eq!(
            //     check_result,
            //     MachineStatus::DecidedEndless(EndlessReason::Bouncer(999))
            // );
            false
        }
    }

    #[test]
    fn decider_cycler_is_cycle_bb4_1166084() {
        let tm = "1RB1LD_1RC---_1LC0RA_0RA0RA";
        let machine = MachineBinary::try_from(tm).unwrap();
        assert!(is_cycler(&machine));
    }

    #[test]
    fn decider_cycler_is_cycle_bb4_43788688() {
        let tm = "1RB---_1LC0RC_0LD1LC_1RA0RA";
        let machine = MachineBinary::try_from(tm).unwrap();
        assert!(is_cycler(&machine));
    }

    #[test]
    fn decider_cycler_holds_after_107_steps() {
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        transitions.push(("0RA", "0RA"));

        let machine = MachineBinary::from_string_tuple(&transitions);
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_cycler(5000)
            .build();
        let res = DeciderCycler::decide_single_machine(&machine, &config);
        assert_eq!(res, MachineStatus::DecidedHalts(107));
    }

//     #[test]
//     fn is_decider_bouncer_bb3_584567() {
//         // BB3 584567 really odd, needs more investigation, possibly cycler
//         let machine = MachineBinary::try_from("1RC---_0RA0LB_1LB1RA").unwrap();
//         assert!(is_cycler(&machine));
//     }
// 
//     #[test]
//     fn is_decider_bouncer_bb3_1265977() {
//         // BB3 1265977 odd behavior, possibly cycler
//         let mut transitions: Vec<(&str, &str)> = Vec::new();
//         transitions.push(("1LC", "---"));
//         transitions.push(("0LA", "0RB"));
//         transitions.push(("1RB", "1LA"));
//         let machine = MachineBinary::from_string_tuple(&transitions);
//         assert!(is_cycler(&machine));
//     }
// 
    #[test]
    fn decider_cycler_unspecified() {
        // free test without expected result
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1LC"));
        transitions.push(("---", "1RC"));
        transitions.push(("1LD", "1RB"));
        transitions.push(("1RA", "0RA"));

        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let machine_status = DeciderCycler::decide_single_machine(&machine, &config);
        println!("result: {}", machine_status);
        let ok = match machine_status {
            MachineStatus::Undecided(_, _, _) => true,
            _ => false,
        };
        assert!(ok);
    }
}
