//! This is a simple decider bouncer.\
//! It detects all bouncers which iterate over the tape start point left and right. \
//! Then it is checked if the left and right side each have the same bits which are added, see examples below. \
//! TODO This bouncer works on the long tape but does not use it. This is actually worse than not using the long tape
//! as now it is not known if the full sides were used.
//! This also means step_limit_bouncer can be set high as mostly the tape borders are reached.
//! This is still highly effective and eliminates >90% of the machines the cycler does not catch. \
//! BB4 with cycler limit 1500 leaves 63,130 of the 6,975,757,441 machines undecided.
//! Using this bouncer with limit 20_000 reduces this to 1,664 machines (only 2,7% left). \
//! Since only very few machines are left, the cycler can now be run again with a limit of 100_000 steps; for
//! BB4 actually the biggest cycle detected requires 95450 steps. \
//! **This leaves only 908 machines undecided.** \
//! All this is done on my Notebook in about 15 seconds.
//!
//! For BB5, first 100,000,000,000: \
//! Cycler (limit 1_500): undecided = 204,762 (runtime 32 seconds) \
//! Bouncer (limit 20_000): undecided = 9,708 (runtime 34 seconds) \
//! Cycler (limit 110_000): undecided = 4,954 (runtime 125 seconds) \
//! Hold (limit 50_000_000): undecided = 4,954 (runtime 3,5 minutes) \
//!
//! Running BB5 complete, with first cycler and first bouncer only takes about 50 Minutes
//! for 16,679,880,978,201 machines with a pre-decider reducing this to 59,649,822,720
//! which need to be evaluated.
//! 7,827,594 machines are left undecided.
//!
//! # Examples
//! When left is 0, then right must be expanding with the same bits as before, \
//! e.g. 11337065 1RB0LB_1LA0LC_---1RD_0RA0RA: \
//! Here step 18 and 46 are identical (both first with left 0), only 46 is expanded by 01 which is ok, as 01 is before that. \
//! Probably need to count the inner cycle which results from this, which is B0-A1 and thus has 2 elements. \
//! Step    18 B0 1LA: 000000000000000000000000_00000000\***110101**00_000000000000000000000000 \
//! Step    19 A1 0LB: 000000000000000000000000_00000000\*00101010_000000000000000000000000 \
//! ... \
//! Step    44 B0 1LA: 000000000000000000000000_00000010\*11010101_000000000000000000000000 \
//! Step    45 A1 0LB: 000000000000000000000000_00000001\*00101010_100000000000000000000000 \
//! Step    46 B0 1LA: 000000000000000000000000_00000000\***11010101**_010000000000000000000000 \
//! \
//! Same goes for right side 0: Step 11 & 35 \
//! Step    10 B1 0LC: 000000000000000000000000_00000010\*10000000_000000000000000000000000 \
//! Step    11 C1 1RD: 000000000000000000000000_00000**101**\*00000000_000000000000000000000000 \
//! Step    12 D0 0RA: 000000000000000000000000_00001010\*00000000_000000000000000000000000 \
//! Step    13 A0 1RB: 000000000000000000000000_00010101\*00000000_000000000000000000000000 \
//! repeat B0-A1 while going left \
//! Step    14 B0 1LA: 000000000000000000000000_00001010\*11000000_000000000000000000000000 \
//! Step    15 A1 0LB: 000000000000000000000000_00000101\*00100000_000000000000000000000000 \
//! Step    16 B0 1LA: 000000000000000000000000_00000010\*11010000_000000000000000000000000 \
//! Step    17 A1 0LB: 000000000000000000000000_00000001\*00101000_000000000000000000000000 \
//! Step    18 B0 1LA: 000000000000000000000000_00000000\*11010100_000000000000000000000000 \
//! Step    19 A1 0LB: 000000000000000000000000_00000000\*00101010_000000000000000000000000 \
//! Step    20 B0 1LA: 000000000000000000000000_00000000\*01010101_000000000000000000000000 \
//! Step    21 A0 1RB: 000000000000000000000000_00000001\*10101010_000000000000000000000000 \
//! Step    22 B1 0LC: 000000000000000000000000_00000000\*10010101_000000000000000000000000 \
//! Step    23 C1 1RD: 000000000000000000000000_00000001\*00101010_000000000000000000000000 \
//! repeat D0-A0-B1-C1 while going right, thus extending by 1010 \
//! Step    24 D0 0RA: 000000000000000000000000_00000010\*01010100_000000000000000000000000 \
//! Step    25 A0 1RB: 000000000000000000000000_00000101\*10101000_000000000000000000000000 \
//! Step    26 B1 0LC: 000000000000000000000000_00000010\*10010100_000000000000000000000000 \
//! Step    27 C1 1RD: 000000000000000000000000_00000101\*00101000_000000000000000000000000 \
//! Step    28 D0 0RA: 000000000000000000000000_00001010\*01010000_000000000000000000000000 \
//! Step    29 A0 1RB: 000000000000000000000000_00010101\*10100000_000000000000000000000000 \
//! Step    30 B1 0LC: 000000000000000000000000_00001010\*10010000_000000000000000000000000 \
//! Step    31 C1 1RD: 000000000000000000000000_00010101\*00100000_000000000000000000000000 \
//! Step    32 D0 0RA: 000000000000000000000000_00101010\*01000000_000000000000000000000000 \
//! Step    33 A0 1RB: 000000000000000000000000_01010101\*10000000_000000000000000000000000 \
//! Step    34 B1 0LC: 000000000000000000000000_00101010\*10000000_000000000000000000000000 \
//! Step    35 C1 1RD: 000000000000000000000000_0**1010101**\*00000000_000000000000000000000000 \
//! This needs approval by 3rd, step 71, as 4 is longer then the existing 3: \
//! Step    64 D0 0RA: 000000000000000000000000_10101010\*01010000_000000000000000000000000 \
//! Step    65 A0 1RB: 000000000000000000000001_01010101\*10100000_000000000000000000000000 \
//! Step    66 B1 0LC: 000000000000000000000000_10101010\*10010000_000000000000000000000000 \
//! Step    67 C1 1RD: 000000000000000000000001_01010101\*00100000_000000000000000000000000 \
//! Step    68 D0 0RA: 000000000000000000000010_10101010\*01000000_000000000000000000000000 \
//! Step    69 A0 1RB: 000000000000000000000101_01010101\*10000000_000000000000000000000000 \
//! Step    70 B1 0LC: 000000000000000000000010_10101010\*10000000_000000000000000000000000 \
//! Step    71 C1 1RD: 000000000000000000000**101_01010101**\*00000000_000000000000000000000000 \
//! \
//! Machine 18226348 0RB---_1LC1RB_0LD0LC_0RA0RA behaves differently: \
//! left is just growing by 1, so same here, but right inserts a 0 always. This is a different kind of extension. \
//! Step     6 B1 1RB: 000000000000000000000000_0000000**1**\*00000000_000000000000000000000000 \
//! Step     7 B0 1LC: 000000000000000000000000_00000000\***11**000000_000000000000000000000000 \
//! ... \
//! Step    17 B1 1RB: 000000000000000000000000_000000**11**\*00000000_000000000000000000000000 \
//! Step    18 B0 1LC: 000000000000000000000000_00000001\*11000000_000000000000000000000000 \
//! Step    19 C1 0LC: 000000000000000000000000_00000000\***101**00000_000000000000000000000000 \
//! ... \
//! Step    40 B1 1RB: 000000000000000000000000_00000**111**\*00000000_000000000000000000000000 \
//! Step    41 B0 1LC: 000000000000000000000000_00000011\*11000000_000000000000000000000000 \
//! Step    42 C1 0LC: 000000000000000000000000_00000001\*10100000_000000000000000000000000 \
//! Step    43 C1 0LC: 000000000000000000000000_00000000\***1001**0000_000000000000000000000000 \
//! ... \
//! Step    87 B1 1RB: 000000000000000000000000_0000**1111**\*00000000_000000000000000000000000 \
//! Step    88 B0 1LC: 000000000000000000000000_00000111\*11000000_000000000000000000000000 \
//! Step    89 C1 0LC: 000000000000000000000000_00000011\*10100000_000000000000000000000000 \
//! Step    90 C1 0LC: 000000000000000000000000_00000001\*10010000_000000000000000000000000 \
//! Step    91 C1 0LC: 000000000000000000000000_00000000\***10001**000_000000000000000000000000 \
//! \
//! Machine Id: 247831398 1RB---_1LC0RD_0LC0LE_0RB0RA_0RA0RA \
//! That is a good test case \

use std::fmt::Display;

// #[cfg(debug_assertions)]
// use bb_challenge::config::StepTypeBig;

use crate::decider::{
    self,
    decider_data_128::DeciderData128,
    decider_result::{BatchData, ResultUnitEndReason},
    Decider,
};
use crate::{
    config::Config,
    machine_binary::MachineBinary,
    status::{MachineStatus, NonHaltReason},
    tape::{tape_utils::U64Ext, Tape},
};

// #[cfg(debug_assertions)]
// const DEBUG_EXTRA: bool = false;

/// Initial capacity for step recorder. Not so relevant.
const MAX_INIT_CAPACITY: usize = 10_000;

// TODO Use long tape, or tape_shifted left & right bound could be introduced.
#[derive(Debug)]
pub struct DeciderBouncer128 {
    data: DeciderData128,
    /// Store all steps to do comparisons (test if a cycle is repeating)
    /// All even are lower bits, all odd upper bits
    steps: Vec<StepBouncer>,
    // / Stores the step ids (2 = 3rd step) for each field in the transition table. \
    // / (basically e.g. all steps for e.g. field 'B0' steps: 1 if A0 points to B, as step 1 then has state B and head symbol 0.)
    // TODO performance: extra differentiation for 0/1 at head position? The idea is, that the field cannot be identical if head read is different
    // maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
}

impl DeciderBouncer128 {
    /// Creates a new bouncer. Only uses step_limit_bouncer from config.
    pub fn new(config: &Config) -> Self {
        let cap = (config.step_limit_decider_bouncer() as usize).min(MAX_INIT_CAPACITY);
        let mut decider = Self {
            data: DeciderData128::new(config),
            steps: Vec::with_capacity(cap),
            // maps_1d: core::array::from_fn(|_| Vec::with_capacity(cap / 4)),
        };
        decider.data.step_limit = config.step_limit_decider_bouncer();

        #[cfg(feature = "enable_html_reports")]
        {
            decider
                .data
                .set_path_option(crate::html::get_html_path("bouncer", config));
        }

        decider
    }

    #[inline]
    fn clear(&mut self) {
        self.data.clear();
        self.steps.clear();
        // for map in self.maps_1d.iter_mut() {
        //     map.clear();
        // }
    }
}

impl Decider for DeciderBouncer128 {
    fn decider_id() -> &'static decider::DeciderId {
        // &DECIDER_BOUNCER_ID
        &decider::DeciderId {
            id: 21,
            name: "Decider Bouncer 128",
            sub_dir: "decider_bouncer_128",
        }
    }

    fn decide_machine(&mut self, machine: &MachineBinary) -> MachineStatus {
        #[cfg(feature = "enable_html_reports")]
        self.data.write_html_file_start(Self::decider_id(), machine);

        // initialize decider
        self.clear();

        self.data.machine = *machine;
        let mut last_left_empty_step_no = 0;
        let mut last_right_empty_step_no = 0;
        let mut is_bouncing_right = false;

        // loop over transitions to write tape
        loop {
            self.data.next_transition();

            // check if done
            if self.data.is_done() {
                break;
            }

            if !self.data.update_tape_single_step() {
                break;
            }

            // get first step where left half tape is empty
            if self.data.tape.is_left_empty()
                && self.data.step_no > last_right_empty_step_no
                && last_left_empty_step_no <= last_right_empty_step_no
            {
                last_left_empty_step_no = self.data.step_no;
                // store step
                let step = StepBouncer {
                    #[cfg(debug_assertions)]
                    _step_no: self.data.step_no,
                    #[cfg(debug_assertions)]
                    _is_upper_bits: true,
                    tape_after: self.data.tape.right_64_bit(),
                };
                self.steps.push(step);
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    let text = format!("  Step {}: tape LEFT empty: comparing", self.data.step_no);
                    println!("{text}");
                    self.data.write_html_p(&text);
                }
                // compare and check if same expanding bits for three consecutive steps
                if self.steps.len() > 7 {
                    let i = self.steps.len() - 1;
                    let changed = [
                        Changed::new(self.steps[i - 4].tape_after, self.steps[i - 6].tape_after),
                        Changed::new(self.steps[i - 2].tape_after, self.steps[i - 4].tape_after),
                        Changed::new(self.steps[i].tape_after, self.steps[i - 2].tape_after),
                    ];
                    is_bouncing_right = Changed::is_bouncer_3(&changed);
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    {
                        let text = if is_bouncing_right {
                            "  Bouncing right!"
                        } else {
                            "  Not Bouncing right!"
                        };
                        println!("{text}");
                        self.data.write_html_p(&text);
                    }
                    // compare and check if same expanding bits for three steps but leaving one out each time
                    if self.steps.len() > 13 {
                        let changed = [
                            Changed::new(
                                self.steps[i - 8].tape_after,
                                self.steps[i - 12].tape_after,
                            ),
                            Changed::new(
                                self.steps[i - 4].tape_after,
                                self.steps[i - 8].tape_after,
                            ),
                            Changed::new(self.steps[i].tape_after, self.steps[i - 4].tape_after),
                        ];
                        is_bouncing_right = Changed::is_bouncer_3(&changed);
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        {
                            let text = if is_bouncing_right {
                                "  Bouncing right double"
                            } else {
                                "  Not Bouncing right double"
                            };
                            println!("{text}");
                            self.data.write_html_p(&text);
                        }
                    }
                }

                // get first step where right half tape is empty
            } else if self.data.tape.is_right_empty()
                && self.data.step_no > last_left_empty_step_no
                && last_right_empty_step_no <= last_left_empty_step_no
            {
                last_right_empty_step_no = self.data.step_no;
                // store step
                let step = StepBouncer {
                    #[cfg(debug_assertions)]
                    _step_no: self.data.step_no,
                    #[cfg(debug_assertions)]
                    _is_upper_bits: false,
                    tape_after: self.data.tape.left_64_bit(),
                };
                self.steps.push(step);
                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                {
                    let text = format!("  Step {}: tape RIGHT empty: comparing", self.data.step_no);
                    println!("{text}");
                    self.data.write_html_p(&text);
                }
                // compare and check if same expanding bits for both sides
                if is_bouncing_right && self.steps.len() > 7 {
                    let i = self.steps.len() - 1;
                    let changed = [
                        Changed::new(self.steps[i - 4].tape_after, self.steps[i - 6].tape_after),
                        Changed::new(self.steps[i - 2].tape_after, self.steps[i - 4].tape_after),
                        Changed::new(self.steps[i].tape_after, self.steps[i - 2].tape_after),
                    ];
                    if Changed::is_bouncer_3(&changed) {
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        {
                            let text = if is_bouncing_right {
                                "  Found a bouncer!"
                            } else {
                                "  Not Bouncing right!"
                            };
                            println!("{text}");
                            self.data.write_html_p(&text);
                        }
                        self.data.status = MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(
                            self.data.step_no,
                        ));
                        break;
                    }
                    if self.steps.len() > 13 {
                        let changed = [
                            Changed::new(
                                self.steps[i - 8].tape_after,
                                self.steps[i - 12].tape_after,
                            ),
                            Changed::new(
                                self.steps[i - 4].tape_after,
                                self.steps[i - 8].tape_after,
                            ),
                            Changed::new(self.steps[i].tape_after, self.steps[i - 4].tape_after),
                        ];
                        if Changed::is_bouncer_3(&changed) {
                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            {
                                let text = if is_bouncing_right {
                                    "  Found a bouncer (double step)!"
                                } else {
                                    "  Not a bouncer double."
                                };
                                println!("{text}");
                                self.data.write_html_p(&text);
                            }
                            self.data.status = MachineStatus::DecidedNonHalt(
                                NonHaltReason::Bouncer(self.data.step_no),
                            );
                            break;
                        }
                    }
                }
            }
        }

        #[cfg(feature = "enable_html_reports")]
        {
            self.data.write_html_file_end();
            // close the file so it can be renamed (not sure if necessary)
            // self.file = None;

            // html::rename_file_to_status(&self.data.path.unwrap(), &self.data.file_name.unwrap(), &ms);
            self.data.rename_html_file_to_status();
        }

        self.data.status
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

/// This struct only stores the tape if either the left or right side of the tape is 0.
/// Every even entry is left side empty, odd right side empty.
/// Since only consecutive entries are checked, the step_no is not relevant.
// TODO step_no could be interesting to check if a rhythm is there (e.g. prev. distance + 2)
#[derive(Debug)]
struct StepBouncer {
    /// only for debugging purposes
    #[cfg(debug_assertions)]
    _step_no: crate::config::StepTypeBig,
    /// only for debugging purposes
    #[cfg(debug_assertions)]
    _is_upper_bits: bool,
    /// tape after transition was executed
    tape_after: u64,
}

/// Function to test single machine
pub fn test_decider(transition_tm_format: &str) {
    // let config = Config::new_default(5);
    let machine = MachineBinary::try_from(transition_tm_format).unwrap();
    let config = Config::builder(machine.n_states())
        .write_html_file(true)
        .write_html_step_start(792_199_000)
        .write_html_line_limit(500_000)
        .step_limit_decider_bouncer(800_000_000)
        .build();
    let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    // assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

/// stores the changed bits between two consecutive relevant steps
struct Changed {
    // start of change
    pos: i32,
    change_moved: u64,
}

impl Changed {
    fn new(newer_tape: u64, older_tape: u64) -> Self {
        // identify changed bits
        let changed = newer_tape ^ older_tape;
        // let pos = changed.leading_zeros();
        let trailing_zeros = if changed != 0 {
            changed.trailing_zeros()
        } else {
            0
        };
        // let len_1 = 64 - pos_1 - trailing_zeros;
        // let pos_1 = pos_1 as i32;
        // let change_moved = changed >> trailing_zeros;
        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            use bb_challenge::tape::tape_utils::U64Ext;

            println!(" OLD {}", older_tape.to_binary_split_string());
            println!(" NEW {}", newer_tape.to_binary_split_string());
        }
        Self {
            pos: trailing_zeros as i32,
            change_moved: changed >> trailing_zeros,
        }
    }

    fn is_bouncer_3(changed: &[Self]) -> bool {
        assert_eq!(3, changed.len());
        changed[0].change_moved == changed[1].change_moved
            && changed[1].change_moved == changed[2].change_moved
            && changed[1].pos - changed[0].pos != 0
            && changed[1].pos - changed[0].pos == changed[2].pos - changed[1].pos
    }

    // TODO generic with more to compare
    // fn is_bouncer(changed: &[Self]) -> bool {
    //     assert!(4 >= changed.len());
    //     for ...
    // }
}

impl Display for Changed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CHG {}: pos {}",
            self.change_moved.to_binary_split_string(),
            self.pos
        )
    }
}

// Note: 'is_not_decider_bouncer_1RB1LC_0RCZZZ_1LD1RC_0RC0RA' will take 16 seconds if not --release
#[cfg(test)]
#[allow(non_snake_case)]
mod tests {

    use crate::status::UndecidedReason;

    use super::*;

    #[test]
    fn is_bouncer_1RB0LB_1LA0LC_ZZZ1RD_0RA0RA() {
        let machine = MachineBinary::try_from("1RB0LB_1LA0LC_---1RD_0RA0RA").unwrap();
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        // println!("{}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(119))
        );
    }

    /// This works almost identical, only every second step needs to be compared, here only the right empty side:
    /// Step     1 A0 1RB: 000000000000000000000000_00000001\*00000000 P: 64 TL 30 30..33 \
    /// Step    10 B1 0RA: 000000000000000000000000_00011010\*00000000 P: 65 TL 30 30..33 \
    /// Step    24 B1 0RA: 000000000000000000000000_00101010\*00000000 P: 67 TL 30 30..33 \
    /// Step    46 B1 0RA: 000000000000000000000110_10101010\*00000000 P: 69 TL 30 30..33 \
    /// Step    72 B1 0RA: 00000000000000000000**1010_10**101010\*00000000 P: 71 TL 30 30..33 \
    /// Step   106 B1 0RA: 000000000000000110101010_10101010\*00000000 P: 73 TL 30 30..33 \
    /// Step   144 B1 0RA: 00000000000000**101010**1010_10101010\*00000000 P: 75 TL 30 30..33
    #[test]
    fn is_bouncer_1RBZZZ_1LC0RA_0LD0LB_1RA0RA() {
        let machine = MachineBinary::try_from("1RB---_1LC0RA_0LD0LB_1RA0RA").unwrap();
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_bouncer(500)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        println!("{}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(190))
        );
    }

    // TODO does not work
    /// This is a different bouncer:
    /// - left is only once 0
    /// - right expands left and right
    /// Step     2 B0 1LC: 000000000000000000000000_00000000\*11000000 P: 63 TL 30 30..33 \
    /// Step     6 A1 1RA: 000000000000000000000000_00000101\*00000000 P: 65 TL 30 30..33 \
    /// Step    14 A1 1RA: 000000000000000000000000_000**1**101**1**\*00000000 P: 67 TL 30 30..33 \
    /// Step    24 A1 1RA: 000000000000000000000000_0**1**11011**1**\*00000000 P: 69 TL 30 30..33 \
    /// Step    36 A1 1RA: 00000000000000000000000**1**_1110111**1**\*00000000 P: 71 TL 30 30..33 \
    /// Step    50 A1 1RA: 000000000000000000000**1**11_1101111**1**\*00000000 P: 73 TL 30 30..33 \
    /// Step    66 A1 1RA: 0000000000000000000**1**1111_1011111**1**\*00000000 P: 75 TL 30 30..33 \
    /// Step    84 A1 1RA: 00000000000000000**1**111111_0111111**1**\*00000000 P: 77 TL 30 30..33

    #[test]
    fn is_decider_bouncer_1RB1RA_1LCZZZ_1RD1LC_0RA0RA() {
        let machine = MachineBinary::try_from("1RB1RA_1LC---_1RD1LC_0RA0RA").unwrap();
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_bouncer(500)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        println!("{}", check_result);
        // assert_eq!(
        //     check_result,
        //     MachineStatus::DecidedEndless(EndlessReason::Bouncer(119))
        // );
    }

    #[test]
    fn is_not_decider_bouncer_bb3_41399() {
        // BB3 41399 (this is a cycler, but it actually expands endless with 0)
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("0RC", "1RB"));
        transitions.push(("1RA", "0RA"));

        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        // let check_result = DeciderCycler::decide_single_machine(&machine, &config);
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        let ok = if let MachineStatus::Undecided(_, _, _) = check_result {
            true
        } else {
            println!("Result: {}", check_result);
            false
        };
        assert!(ok);
    }

    #[test]
    fn is_decider_bouncer_bb3_84080() {
        // BB3 84080 (high bound check)
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LA", "0RA"));

        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(48))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_112641() {
        // BB3 112641
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("1LA", "0RA"));

        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(80))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_569564() {
        // BB3 569564
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "0LA"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LB", "1RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(56))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_584567() {
        // BB3 584567 step_delta doubles
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("0RA", "0LB"));
        transitions.push(("1LB", "1RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(112))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_1265977() {
        // BB3 1265977 step_delta doubles
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "---"));
        transitions.push(("0LA", "0RB"));
        transitions.push(("1RB", "1LA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(123))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_1970063() {
        // BB3 1970063 step_delta iterates same delta +-
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RB", "0LA"));
        transitions.push(("1RC", "---"));
        transitions.push(("1LA", "1RB"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(113))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_3044529() {
        // BB3 3044529 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("0LA", "0RC"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(93))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_3554911() {
        // BB3 3554911 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "---"));
        transitions.push(("1LC", "1RB"));
        transitions.push(("0RA", "0LC"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(87))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_6317243() {
        // BB4 Start out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("1RD", "0LC"));
        transitions.push(("1LB", "0RB"));
        transitions.push(("0RA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(138))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_13318557() {
        // BB4 Start High bound out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("0LD", "1LB"));
        transitions.push(("0LB", "1RC"));
        transitions.push(("0RA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(37))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_15783962() {
        // BB4 ascending shift with gap and linear growing distance between head pos
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LB", "1RD"));
        transitions.push(("1LC", "---"));
        transitions.push(("1RA", "1LC"));
        transitions.push(("0RA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(71))
        );
    }

    #[test]
    fn is_decider_bouncer_bb3_32538705() {
        // BB4 sinus, but not with A0
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
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(106))
        );
    }

    // TODO other bouncer or extend
    /// This is an interesting case, but is not caught by this bouncer.
    /// Step   2 C0 1RB: 00000000000000000000000000000000_000000000000000000000000_00000001\*00000000 P: 63 TL 2 2..5 \
    /// Step  24 D1 0RA: 00000000000000000000000000000000_000000000000000000000000_00011100\*00000000 P: 65 TL 2 2..5 \
    /// Step  70 D1 0RA: 00000000000000000000000000000000_0000000000000000000000**11_010**11100\*00000000 P: 67 TL 2 2..5 \
    /// Step 130 D1 0RA: 00000000000000000000000000000000_0000000000000000000**101**11_01011100\*00000000 P: 69 TL 2 2..5 \
    /// Step 210 D1 0RA: 00000000000000000000000000000000_000000000000000**1110**10111_01011100\*00000000 P: 71 TL 2 2..5 \
    /// Step 312 D1 0RA: 00000000000000000000000000000000_0000000000**11010**111010111_01011100\*00000000 P: 73 TL 2 2..5 \
    /// Step 428 D1 0RA: 00000000000000000000000000000000_0000000**101**11010111010111_01011100\*00000000 P: 75 TL 2 2..5 \
    /// Step 564 D1 0RA: 00000000000000000000000000000000_000**11**1010111010111010111_01011100\*00000000 P: 77 TL 2 2..5 \
    /// Step 894 D1 0RA: 000000000000000000000000000**10111_010**111010111010111010111_01011100\*00000000 P: 81 TL 2 2..5 \

    #[test]
    fn is_not_decider_bouncer_bb4_45935166() {
        // BB4 delta of delta rhythm 22, 14, 20 repeats; requires 128-bit tape
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LC", "1LA"));
        transitions.push(("0RD", "---"));
        transitions.push(("1RB", "1LD"));
        transitions.push(("1RA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_bouncer(2000)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::Undecided(UndecidedReason::StepLimit, 2000, 59)
        );
    }

    #[test]
    fn is_decider_bouncer_bb4_2793430() {
        // BB4 every 2nd step
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "0LD"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("---", "1RA"));
        transitions.push(("0RA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::DecidedNonHalt(NonHaltReason::Bouncer(132))
        );
    }

    // TODO interesting machine, endless, but need other check
    #[test]
    fn is_not_decider_bouncer_bb4_64379691() {
        // BB4 every steps repeating, but with growing amount of identical steps
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "1RA"));
        transitions.push(("---", "1RD"));
        transitions.push(("1RB", "1LC"));
        transitions.push(("0LA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_decider_bouncer(2000)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        if let MachineStatus::Undecided(UndecidedReason::TapeSizeLimit, _, _) = check_result {
        } else {
            panic!("{check_result}");
        }

        // good example of switched status, else same machine
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "1RA"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("---", "1RD"));
        transitions.push(("0LA", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        if let MachineStatus::Undecided(UndecidedReason::TapeSizeLimit, _, _) = check_result {
        } else {
            panic!("{check_result}");
        }
    }

    #[test]
    fn is_not_decider_bouncer_bb3_max_651320() {
        // BB3 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("1RB", "0LC"));
        transitions.push(("1RC", "1RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(check_result, MachineStatus::DecidedHalts(21));
    }

    #[test]
    fn is_not_decider_bouncer_bb4_max_322636617() {
        // BB4 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        assert_eq!(check_result, MachineStatus::DecidedHalts(107));
    }

    #[test]
    fn is_not_decider_bouncer_bb5_max() {
        // BB5 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1LC"));
        transitions.push(("1RC", "1RB"));
        transitions.push(("1RD", "0LE"));
        transitions.push(("1LA", "1LD"));
        transitions.push(("---", "0LA"));
        let machine = MachineBinary::from_string_tuple(&transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        let ok = if let MachineStatus::Undecided(_, _, _) = check_result {
            true
        } else {
            println!("Result: {}", check_result);
            false
        };
        assert!(ok);
    }

    #[test]
    /// This is a long running test checking if the tape_size_limit is reached. \
    /// It also demonstrates the use of write_html_step_start to produces a reasonable size html file. \
    /// Runtime is around 4 seconds in release mode, 16 s in normal mode.
    fn is_not_decider_bouncer_1RB1LC_0RCZZZ_1LD1RC_0RC0RA() {
        let machine = MachineBinary::try_from("1RB1LC_0RC---_1LD1RC_0RC0RA").unwrap();
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .write_html_step_start(792_199_000)
            .write_html_line_limit(500_000)
            .step_limit_decider_bouncer(800_000_000)
            .build();
        let check_result = DeciderBouncer128::decide_single_machine(&machine, &config);
        println!("{}", check_result);
        let ok = if let MachineStatus::Undecided(_, _, _) = check_result {
            true
        } else {
            println!("Result: {}", check_result);
            false
        };
        assert!(ok);
    }
}
