//! This is a simple decider bouncer.\
//! It detects all bouncers which iterate over the tape start point left and right. \
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

#![allow(clippy::uninlined_format_args)]
#[cfg(debug_assertions)]
use std::io::Write;

/// This decider finds repeating machines, which do not have a loop, but a repeating rhythm on tape, which endlessly expands.
/// It is working on a 128 Bit tape.
// TODO identify non-bouncer to end quickly.
// TODO document logic
// TODO is this final?
// TODO Why so many steps required?
// TODO speed up by repeating rhythm
// #[cfg(debug_assertions)]
// #[cfg(all(debug_assertions, feature = "bb_debug"))]
// use crate::machine::EndlessReason;
// #[cfg(debug_assertions)]
// #[cfg(all(debug_assertions, feature = "bb_debug"))]
// use crate::utils::U64Ext;
use crate::{
    config::{Config, StepTypeBig, StepTypeSmall, MAX_STATES, N_STATES_DEFAULT},
    decider::{self, Decider, DECIDER_BOUNCER_ID},
    decider_result::BatchData,
    machine::Machine,
    status::{EndlessReason, ExpandingBouncerReason, MachineStatus, UndecidedReason},
    transition_symbol2::{TransitionSymbol2, TRANSITION_0RA},
    ResultUnitEndReason,
};

#[cfg(debug_assertions)]
const IS_DEBUG: bool = false;

// #[cfg(debug_assertions)]
// const DEBUG_MACHINE_NO: usize = 0; // 84080; // 351902; // 1469538; // 322636617; // BB3 max: 651320; // 46; //

type TapeType = u128;
type BitType = i8;
type SymbolStateType = u8;

const TAPE_SIZE_BIT: StepTypeSmall = 128;
const MIDDLE_BIT: BitType = (TAPE_SIZE_BIT / 2 - 1) as BitType;
const POS_HALF: TapeType = 1 << MIDDLE_BIT;

// const SINUS_RHYTHM_GIVE_UP: usize = 100;

pub struct DeciderBouncer {
    step_limit: StepTypeSmall,
    /// Stores each step and its tape
    steps: Vec<StepExpanding>,
    /// For each field in the transition matrix store which steps were used.
    maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
    /// Filter on steps for the relevant steps to check sinus expanding rhythm.
    sinus_steps: Vec<SinusStep>,
    /// For each sinus step the relevant tape part for change analysis.
    sinus_tapes: Vec<i64>,
    /// Difference between two sinus steps.
    deltas: Vec<i32>,
    /// Difference between two deltas.
    deltas2nd: Vec<i32>,
    /// For the deltas2nd elements a compressed list of the delta and how often it is repeated.
    deltas2nd_count: Vec<(i32, i32)>,
    sync_high_bit: BitType,
    sync_low_bit: BitType,
    expanding_sinus_reason: ExpandingBouncerReason,
    // #[cfg(all(debug_assertions, feature = "bb_debug"))]
    #[cfg(debug_assertions)]
    // TODO remove in final run
    // used in debugging to see which machine is currently worked on in sub-function
    machine_info: crate::machine_info::MachineInfo,
}

impl DeciderBouncer {
    pub fn new(config: &Config) -> Self {
        Self {
            step_limit: config.step_limit_bouncer(),
            ..Default::default()
        }
    }

    pub fn new_form_self(&self) -> Self {
        Self {
            step_limit: self.step_limit,
            ..Default::default()
        }
    }

    // tape_long_bits in machine?
    // TODO counter: longest loop
    pub fn decide_machine_main(&mut self, machine: &Machine) -> MachineStatus {
        #[cfg(debug_assertions)]
        let mut file = None;
        #[cfg(debug_assertions)]
        if IS_DEBUG {
            println!("\nDecider Expanding Sinus for {machine}");
            // TODO why not simply machine?
            self.machine_info =
                crate::machine_info::MachineInfo::from_machine(machine, &MachineStatus::NoDecision);
            file = Some(std::fs::File::create("debug_info.txt").unwrap());
        }

        // initialize decider

        // num steps, same as steps, but steps can be deactivated after a while
        self.steps.clear();

        // tape for storage in Step with cell before transition at position u32 top bit
        // this tape shifts in every step, so that the head is always at bit MIDDLE_BIT
        let mut tape_shifted: u128 = 0;
        let mut high_bound = MIDDLE_BIT;
        let mut low_bound = MIDDLE_BIT;
        let mut pos_middle_bit = MIDDLE_BIT;

        // map for each transition, which step went into it
        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
        // Initialize transition with A0 as start
        let mut tr = TransitionSymbol2 {
            transition: TRANSITION_0RA,
            #[cfg(debug_assertions)]
            text: ['0', 'R', 'A'],
        };

        // loop over transitions to write tape
        loop {
            // store next step
            let curr_read_symbol = ((tape_shifted & POS_HALF) != 0) as usize; // resolves to one if bit is set

            // maps: store step id leading to this
            // TODO map_id as for_symbol_state
            self.maps_1d[tr.state_x2() + curr_read_symbol].push(self.steps.len());
            let step = StepExpanding::new(
                tr,
                curr_read_symbol as u8,
                tape_shifted,
                high_bound,
                low_bound,
                pos_middle_bit,
            );
            self.steps.push(step.clone());
            tr = machine.transition(tr.state_x2() + curr_read_symbol);
            // #[cfg(all(debug_assertions, feature = "bb_debug"))]
            // println!("TR Step {}: {}", self.steps.len(), tr);

            // check if done
            // halt is regarded as step, so always count step
            if tr.is_hold() {
                // Hold found
                // write last symbol
                // TODO count ones
                #[allow(unused_assignments)]
                if tr.symbol() < 2 {
                    tape_shifted = if tr.is_symbol_one() {
                        tape_shifted | POS_HALF
                    } else {
                        tape_shifted & !POS_HALF
                    };
                }
                // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
                return MachineStatus::DecidedHolds(self.steps.len() as StepTypeBig);
            }
            if self.steps.len() > self.step_limit as usize {
                return MachineStatus::Undecided(
                    UndecidedReason::StepLimit,
                    self.steps.len() as StepTypeBig,
                    TAPE_SIZE_BIT,
                );
            }

            // print step info
            #[cfg(debug_assertions)]
            if IS_DEBUG {
                // let read_symbol_next = ((tape_shifted & POS_HALF) != 0) as usize;
                let step = self.steps.last().unwrap();
                let s = format!(
                    "Step {:3}: {}{} {} before: Tape shifted {} H{high_bound}{} L{low_bound}{} P{pos_middle_bit:>3}{} {}", // , next {}{} {}",
                    self.steps.len() -1,
                    (step.for_state() + 64)  as char,
                    step.for_symbol(),
                    tr,
                    crate::tape_utils::U128Ext::to_binary_split_string_half(&tape_shifted),
                    if step.is_a0() && high_bound == MIDDLE_BIT {'*'} else {' '},
                    if step.is_a0() && low_bound == MIDDLE_BIT {'*'} else {' '},
                    if step.is_a0() && pos_middle_bit == MIDDLE_BIT {'*'} else {' '},
                    if step.is_a0() {"A0"} else {""},
                );
                println!("{s}");
                if let Some(mut file) = file.as_ref() {
                    _ = writeln!(file, "{s}");
                }
            }

            // check sinus loop for multiple steps with A0, this covers most sinus loops
            self.sync_high_bit = 0;
            self.sync_low_bit = 0;
            if step.is_a0() && self.maps_1d[2].len() > 4 {
                let step_2 = self.steps[self.maps_1d[2][1]].clone();
                self.sync_high_bit = step_2.high_bound_before;
                self.sync_low_bit = step_2.low_bound_before;
                if high_bound == self.sync_high_bit {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!(
                            "  Check high_bound step {}: {}{}",
                            self.steps.len() - 1,
                            self.steps.last().unwrap().text[0],
                            self.steps.last().unwrap().text[1],
                        );
                    }
                    // let mut relevant_steps = Vec::with_capacity(10);

                    // if relevant_steps.len() > SINUS_RHYTHM_GIVE_UP {
                    //     return MachineStatus::Undecided(
                    //         UndecidedReason::NoSinusRhythmIdentified,
                    //         self.steps.len() as StepType,
                    //         TAPE_LONG_BYTE * 8,
                    //     );
                    // }

                    // skip start transition as it sometimes is out of sync
                    self.sinus_steps.clear();
                    for &step_id in self.maps_1d[2][1..].iter() {
                        let step = &self.steps[step_id];
                        if step.high_bound_before == self.sync_high_bit {
                            // relevant_steps.push((step_id as isize, step.pos_middle_bit_before));
                            self.sinus_steps.push(SinusStep {
                                step_id: step_id as i32,
                                pos_middle: step.pos_middle_bit_before,
                            });
                        }
                    }

                    // let old = check_rhythm(&relevant_steps);
                    // let new = self.check_sinus_rhythm();
                    // if old != new && old == true {
                    //     let old = check_rhythm(&relevant_steps);
                    //     let new = self.check_sinus_rhythm();
                    //     println!("old {old}, new {new}");
                    // }

                    // check rhythm
                    if self.decide_is_bouncer() {
                        return MachineStatus::DecidedEndless(EndlessReason::ExpandingBouncer(
                            self.expanding_sinus_reason,
                        ));
                    }
                } else if low_bound == self.sync_low_bit {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!(
                            "  Check low_bound step {}: {}{}",
                            self.steps.len() - 1,
                            self.steps.last().unwrap().text[0],
                            self.steps.last().unwrap().text[1],
                        );
                    }
                    // let mut relevant_steps = Vec::with_capacity(10);

                    self.sinus_steps.clear();
                    // skip Start as it sometimes is out of sync
                    for &step_id in self.maps_1d[2][1..].iter() {
                        let step = &self.steps[step_id];
                        if step.low_bound_before == self.sync_low_bit {
                            // relevant_steps.push((step_id as isize, step.pos_middle_bit_before));
                            self.sinus_steps.push(SinusStep {
                                step_id: step_id as i32,
                                pos_middle: step.pos_middle_bit_before,
                            });
                        }
                    }
                    // if relevant_steps.len() > SINUS_RHYTHM_GIVE_UP {
                    //     return MachineStatus::Undecided(
                    //         UndecidedReason::NoSinusRhythmIdentified,
                    //         self.steps.len() as StepType,
                    //         TAPE_LONG_BYTE * 8,
                    //     );
                    // }

                    // if check_rhythm(&relevant_steps) != self.check_sinus_rhythm() {
                    //     let old = check_rhythm(&relevant_steps);
                    //     let mut new = !old;
                    //     if old {
                    //         check_rhythm(&relevant_steps);
                    //         new = self.check_sinus_rhythm();
                    //     }
                    //     println!("old {old}, new {new}");
                    // }
                    // check rhythm
                    if self.decide_is_bouncer() {
                        return MachineStatus::DecidedEndless(EndlessReason::ExpandingBouncer(
                            self.expanding_sinus_reason,
                        ));
                    }
                } else if pos_middle_bit == MIDDLE_BIT {
                    // head stays on middle bit, check if it expands left or right
                    // TODO unclear if this is needed, works maybe faster than other check, finds as ExpandingSinusReason::Delta2ndRepeating
                    // TODO move relevant_steps into decider or array
                    // TODO remove step_id
                    let mut relevant_steps = Vec::with_capacity(10);
                    for &step_id in self.maps_1d[2][1..].iter() {
                        let step = &self.steps[step_id];
                        if step.pos_middle_bit_before == MIDDLE_BIT {
                            // TODO remove 2nd par
                            relevant_steps.push((
                                step_id as isize,
                                step.low_bound_before,
                                step.high_bound_before,
                            ));
                        }
                    }

                    // check rhythm
                    // would be safer to check also extension, e.g. 01
                    // TODO shift calc based on delta and first and last, just one calc
                    // TODO shift calc first
                    // TODO step map only A0
                    // TODO == 6 and then do not check further, for all tests
                    if relevant_steps.len() > 7 {
                        // expanding to right
                        let delta_low = relevant_steps[1].1 - relevant_steps[0].1;
                        if delta_low == relevant_steps[2].1 - relevant_steps[1].1 {
                            let last = relevant_steps.len() - 1;
                            if relevant_steps[last].1
                                == relevant_steps[0].1 + delta_low * last as BitType
                            {
                                return MachineStatus::DecidedEndless(
                                    EndlessReason::ExpandingBouncer(
                                        ExpandingBouncerReason::HeadMiddleExpanding,
                                    ),
                                );
                            }
                        };

                        // expanding to left
                        let delta_high = relevant_steps[1].2 - relevant_steps[0].2;
                        if delta_high == relevant_steps[2].2 - relevant_steps[1].2 {
                            let last = relevant_steps.len() - 1;
                            if relevant_steps[last].2
                                == relevant_steps[0].2 + delta_high * last as BitType
                            {
                                return MachineStatus::DecidedEndless(
                                    EndlessReason::ExpandingBouncer(
                                        ExpandingBouncerReason::HeadMiddleExpanding,
                                    ),
                                );
                            }
                        };

                        // not expanding
                        #[cfg(debug_assertions)]
                        if IS_DEBUG {
                            println!("  low and high bounds do not match");
                        }
                    }
                }
            }

            if self.steps.len() > 50 && !step.is_a0() {
                // now check also fields other than A0, e.g. BB4 32538705
                let map_id = step.map_id();
                if self.maps_1d[map_id].len() > 4 {
                    let step_2 = self.steps[self.maps_1d[map_id][1]].clone();
                    let sync_high_bit = step_2.high_bound_before;
                    let sync_low_bit = step_2.low_bound_before;
                    if high_bound == sync_high_bit {
                        #[cfg(debug_assertions)]
                        if IS_DEBUG {
                            println!(
                                "  Extra check high_bound step {}: {}{}",
                                self.steps.len() - 1,
                                self.steps.last().unwrap().text[0],
                                self.steps.last().unwrap().text[1],
                            );
                        }
                        // let mut relevant_steps = Vec::with_capacity(10);

                        // skip first transition as it sometimes is out of sync
                        self.sinus_steps.clear();
                        for &step_id in self.maps_1d[map_id][1..].iter() {
                            let step = &self.steps[step_id];
                            if step.high_bound_before == self.sync_high_bit {
                                // relevant_steps.push((step_id as isize, step.pos_middle_bit_before));
                                self.sinus_steps.push(SinusStep {
                                    step_id: step_id as i32,
                                    pos_middle: step.pos_middle_bit_before,
                                });
                            }
                        }
                        // // skip Start as it sometimes is out of sync
                        // for &step_id in self.maps_1d[map_id][1..].iter() {
                        //     let step = &self.steps[step_id];
                        //     if step.high_bound_before == sync_high_bit {
                        //         relevant_steps.push((step_id as isize, step.pos_middle_bit_before));
                        //     }
                        // }

                        // check rhythm
                        if self.decide_is_bouncer() {
                            return MachineStatus::DecidedEndless(EndlessReason::ExpandingBouncer(
                                ExpandingBouncerReason::DeciderNoResult,
                            ));
                        }
                    } else if low_bound == sync_low_bit {
                        #[cfg(debug_assertions)]
                        if IS_DEBUG {
                            println!(
                                "  Check low_bound step {}: {}{}",
                                self.steps.len() - 1,
                                self.steps.last().unwrap().text[0],
                                self.steps.last().unwrap().text[1],
                            );
                        }
                        // let mut relevant_steps = Vec::with_capacity(10);

                        // skip Start as it sometimes is out of sync
                        self.sinus_steps.clear();
                        for &step_id in self.maps_1d[map_id][1..].iter() {
                            let step = &self.steps[step_id];
                            if step.low_bound_before == sync_low_bit {
                                // relevant_steps.push((step_id as isize, step.pos_middle_bit_before));
                                self.sinus_steps.push(SinusStep {
                                    step_id: step_id as i32,
                                    pos_middle: step.pos_middle_bit_before,
                                });
                            }
                        }
                        // if relevant_steps.len() > SINUS_RHYTHM_GIVE_UP {
                        //     return MachineStatus::Undecided(
                        //         UndecidedReason::NoSinusRhythmIdentified,
                        //         self.steps.len() as StepType,
                        //         TAPE_LONG_BYTE * 8,
                        //     );
                        // }

                        // check rhythm
                        if self.decide_is_bouncer() {
                            return MachineStatus::DecidedEndless(EndlessReason::ExpandingBouncer(
                                self.expanding_sinus_reason,
                            ));
                        }
                    }
                }
            }

            // update tape: write symbol at head position into cell
            tape_shifted = if tr.is_symbol_one() {
                tape_shifted | POS_HALF
            } else {
                tape_shifted & !POS_HALF
            };

            tape_shifted = if tr.is_dir_right() {
                pos_middle_bit += 1;
                if high_bound == (TAPE_SIZE_BIT - 1) as BitType {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!(
                            "tape shifted {}",
                            crate::tape_utils::U128Ext::to_binary_split_string_half(&tape_shifted)
                        );
                        println!("State: Undecided: Too many steps to right.");
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitLeftBoundReached,
                        self.steps.len() as StepTypeBig,
                        TAPE_SIZE_BIT,
                    );
                }
                // adding high bound here, so i8 will not overflow
                high_bound += 1;
                if low_bound < MIDDLE_BIT {
                    low_bound += 1;
                }
                tape_shifted << 1
            } else {
                pos_middle_bit -= 1;
                low_bound -= 1;
                if low_bound == -1 {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!(
                            "tape shifted {}",
                            crate::tape_utils::U128Ext::to_binary_split_string_half(&tape_shifted)
                        );
                        println!("State: Undecided: Too many steps to left.");
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitRightBoundReached,
                        self.steps.len() as StepTypeBig,
                        TAPE_SIZE_BIT,
                    );
                }
                if high_bound > MIDDLE_BIT {
                    high_bound -= 1;
                }
                tape_shifted >> 1
            };
        }
    }

    /// returns true if this is a bouncer
    // #[inline(always)]
    fn decide_is_bouncer(&mut self) -> bool {
        // check rhythm
        // would be safer to check also extension, e.g. 01
        if self.sinus_steps.len() < 8 {
            #[cfg(debug_assertions)]
            if IS_DEBUG {
                println!("  only {} sinus steps", self.sinus_steps.len());
            }
            return false;
        } else if self.sinus_steps[1].pos_middle - self.sinus_steps[0].pos_middle
            != self.sinus_steps[2].pos_middle - self.sinus_steps[1].pos_middle
        {
            // filter steps without movement
            let mut last_pos = self.sinus_steps.last().unwrap().pos_middle;
            for i in (1..self.sinus_steps.len()).rev() {
                if self.sinus_steps[i - 1].pos_middle == last_pos {
                    self.sinus_steps.remove(i);
                } else {
                    last_pos = self.sinus_steps[i - 1].pos_middle;
                }
            }
            if self.sinus_steps.len() < 8 {
                #[cfg(debug_assertions)]
                if IS_DEBUG {
                    println!("  only {} reduced sinus steps", self.sinus_steps.len());
                }
                return false;
            }
        }

        #[cfg(all(debug_assertions, feature = "bb_debug"))]
        {
            self.deltas.clear();
            self.deltas2nd.clear();
            self.deltas2nd_count.clear();
            self.sinus_tapes.clear();
        }

        // check shift is equal
        let shift = (self.sinus_steps[1].pos_middle - self.sinus_steps[0].pos_middle) as i32;
        if shift * (self.sinus_steps.len() - 1) as i32
            == (self.sinus_steps[self.sinus_steps.len() - 1].pos_middle
                - self.sinus_steps[0].pos_middle) as i32
        {
            // check step delta, either equal in all steps or halves every time
            let sd1 = self.sinus_steps[1].step_id - self.sinus_steps[0].step_id;
            let sd2 = self.sinus_steps[2].step_id - self.sinus_steps[1].step_id;
            let sd3 = self.sinus_steps[3].step_id - self.sinus_steps[2].step_id;
            let step_delta_2nd_1_2 = sd2 - sd1;
            let step_delta_2nd_2_3 = sd3 - sd2;
            if step_delta_2nd_1_2 == step_delta_2nd_2_3 {
                // let sd4 = self.sinus_steps[4].step_id - self.sinus_steps[3].step_id;
                // let sd5 = self.sinus_steps[5].step_id - self.sinus_steps[4].step_id;
                // if sd2 == self.sinus_steps[4].step_id - self.sinus_steps[3].step_id
                //     && sd1 == self.sinus_steps[5].step_id - self.sinus_steps[4].step_id
                if self.sinus_steps[0].step_id + sd1 * 7 == self.sinus_steps[7].step_id {
                    self.expanding_sinus_reason = ExpandingBouncerReason::StepDeltaIdentical;
                    return true;
                }
            }

            // Distance doubles every sinus step
            if step_delta_2nd_2_3 == step_delta_2nd_1_2 * 2 {
                // validate with higher steps
                let mut d2nd = step_delta_2nd_2_3 * 2;
                let mut d = sd3 + step_delta_2nd_2_3 * 2;
                #[allow(clippy::never_loop)]
                'dd: loop {
                    // emulate goto
                    for i in 3..7 {
                        let s = self.sinus_steps[i].step_id + d;
                        if self.sinus_steps[i + 1].step_id != s {
                            break 'dd;
                        }
                        d2nd *= 2;
                        d += d2nd;
                    }
                    self.expanding_sinus_reason = ExpandingBouncerReason::StepDelta2ndDoubles;
                    return true;
                }
            }

            // Distance iterates same value positive and negative
            if step_delta_2nd_1_2 == -step_delta_2nd_2_3 {
                // let sd4 = self.sinus_steps[4].step_id - self.sinus_steps[3].step_id;
                // let sd5 = self.sinus_steps[5].step_id - self.sinus_steps[4].step_id;
                if sd2 == self.sinus_steps[4].step_id - self.sinus_steps[3].step_id
                    && sd1 == self.sinus_steps[5].step_id - self.sinus_steps[4].step_id
                {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!("{}", self.machine_info);
                        todo!("Distance iterates same value positive and negative");
                    }
                    // TODO This code section is incomplete
                    return false;
                    // self.expanding_sinus_reason =
                    // return true;
                }
            }

            // try every 2nd step
            // if shift == 0 && self.sinus_steps.len() > 10 {
            //     // check step delta, either equal in all steps or halves every time
            //     let sd1 = self.sinus_steps[2].step_id - self.sinus_steps[0].step_id;
            //     let sd2 = self.sinus_steps[4].step_id - self.sinus_steps[2].step_id;
            //     let sd3 = self.sinus_steps[6].step_id - self.sinus_steps[4].step_id;
            //     let step_delta_1 = sd2 - sd1;
            //     let step_delta_2 = sd3 - sd2;
            //     // TODO safety check higher elements?
            //     if step_delta_1 == step_delta_2
            //         || step_delta_2 == step_delta_1 * 2
            //         || step_delta_1 == step_delta_2 * -1
            //     {
            //         return true;
            //     }
            // }

            // find similar delta of deltas
            let shift = self.sinus_steps[1].pos_middle - self.sinus_steps[0].pos_middle;
            self.deltas.clear();
            self.deltas
                .push(self.sinus_steps[1].step_id - self.sinus_steps[0].step_id);
            let mut last_step = 1;
            for (i, r) in self.sinus_steps.iter().enumerate().skip(2) {
                if r.pos_middle == self.sinus_steps[last_step].pos_middle + shift {
                    self.deltas
                        .push(r.step_id - self.sinus_steps[last_step].step_id);
                    last_step = i;
                }
            }

            self.deltas2nd.clear();
            for d_pair in self.deltas.windows(2) {
                self.deltas2nd.push(d_pair[1] - d_pair[0]);
            }
            // check special constellation repeat of one, two or triple
            if self.deltas2nd.len() > 5
                && self.deltas2nd[0] == self.deltas2nd[3]
                && self.deltas2nd[1] == self.deltas2nd[4]
                && self.deltas2nd[2] == self.deltas2nd[5]
            {
                // TODO more elements to check?
                self.expanding_sinus_reason = ExpandingBouncerReason::StepDelta2ndRepeating;
                return true;
            }

            self.sinus_tapes.clear();
            // let bitshift = self.sync_low_bit as i8;
            for step in self.sinus_steps.iter() {
                // bitshift not necessary, but safes memory and allows to work with i64
                self.sinus_tapes.push(
                    (self.steps[step.step_id as usize].tape_before >> self.sync_low_bit) as i64,
                );
            }
            // check tape_delta
            let tape_delta_1 = self.sinus_tapes[1] - self.sinus_tapes[0];
            let tape_delta_2 = self.sinus_tapes[2] - self.sinus_tapes[1];

            // step 2nd distance identical
            if tape_delta_1 == tape_delta_2
                && self.sinus_tapes[7] == self.sinus_tapes[0] + tape_delta_1 * 7
            {
                // validated with step 7
                self.expanding_sinus_reason = ExpandingBouncerReason::TapeValueDeltaIdentical;
                return true;
            }

            // step 2nd distance identical every other 2nd distance
            if self.sinus_tapes[2] == self.sinus_tapes[0]
                && self.sinus_tapes[4] == self.sinus_tapes[0]
                && self.sinus_tapes[6] == self.sinus_tapes[0]
                && self.sinus_tapes[3] == self.sinus_tapes[1]
                && self.sinus_tapes[5] == self.sinus_tapes[1]
                && self.sinus_tapes[7] == self.sinus_tapes[1]
            {
                self.expanding_sinus_reason = ExpandingBouncerReason::TapeValueDeltaAlternating;
                return true;
            }

            if self.deltas2nd.len() > 55 {
                // Find repeated values, e.g. if the delta2nd is -2 29 times in a row, then store (-2, 29).
                // The idea is to treat the repeated values as one step and find a rhythm.
                self.deltas2nd_count.clear();
                let mut count = 1;
                let mut last_d = self.deltas2nd[0];
                for &d in self.deltas2nd[1..].iter() {
                    if d == last_d {
                        count += 1;
                    } else {
                        self.deltas2nd_count.push((last_d, count));
                        last_d = d;
                        count = 1;
                    }
                }
                if self.deltas2nd_count.last().unwrap().0 == last_d {
                    self.deltas2nd_count.last_mut().unwrap().1 = count;
                } else {
                    self.deltas2nd_count.push((last_d, count));
                }
                if self.deltas2nd_count.len() >= 12 {
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!("Machine {}", self.machine_info);
                    }
                    // Check special constellation with 3 repeating values for BB4 64379691
                    // TODO other constellations
                    let mut d3rd = [[0; 4]; 3];
                    // let size3 = self.deltas2nd_count.len()/3;
                    // for i in (0..size3*3).step_by(3) {
                    for i in (0..12).step_by(3) {
                        let p = i / 3;
                        d3rd[0][p] = self.deltas2nd_count[i].0;
                        d3rd[1][p] = self.deltas2nd_count[i + 1].0;
                        d3rd[2][p] = self.deltas2nd_count[i + 2].0;
                    }
                    #[allow(clippy::never_loop)]
                    'check: loop {
                        for d3 in d3rd {
                            let d1 = d3[1] - d3[0];
                            if d3[2] - d3[1] == d1 {
                                // check all are identical
                                if d3[3] - d3[2] != d1 {
                                    break 'check;
                                }
                            } else {
                                // check all are growing identical
                                let d2 = d3[2] - d3[1];
                                let dd = d2 - d1;
                                // 16, 32, 64, ?
                                // could be 128 or 16*3 or 16*4, too generic
                                // requires one more step, but that requires a larger tape
                                // TODO larger tape and more steps
                                if d3[3] != d3[2] + dd * 4 {
                                    break 'check;
                                }
                            }
                        }
                        // for row in 0..3 {
                        //     let d1 = d3rd[row][1] - d3rd[row][0];
                        //     if d3rd[row][2] - d3rd[row][1] == d1 {
                        //         // check all are identical
                        //         if d3rd[row][3] - d3rd[row][2] != d1 {
                        //             break 'check;
                        //         }
                        //     } else {
                        //         // check all are growing identical
                        //         let d2 = d3rd[row][2] - d3rd[row][1];
                        //         let dd = d2 - d1;
                        //         // 16, 32, 64, ?
                        //         // could be 128 or 16*3 or 16*4, too generic
                        //         // requires one more step, but that requires a larger tape
                        //         // TODO larger tape and more steps
                        //         if d3rd[row][3] != d3rd[row][2] + dd * 4 {
                        //             break 'check;
                        //         }
                        //     }
                        // }
                        #[cfg(debug_assertions)]
                        if IS_DEBUG {
                            println!("OK: Find repeat value at step {}", self.steps.len());
                            println!("Deltas: {:?}", d3rd);
                        }
                        self.expanding_sinus_reason =
                            ExpandingBouncerReason::StepDelta2ndCompressedRepeating;
                        // break;
                        return true;
                    }
                    #[cfg(debug_assertions)]
                    if IS_DEBUG {
                        println!("Failed: Find repeat value at step {}", self.steps.len());
                        println!("Deltas: {:?}", d3rd);
                        todo!("check if false should be returned")
                    }
                    // TODO false is a quick fix
                    return false;
                }
            }

            #[cfg(debug_assertions)]
            if IS_DEBUG {
                println!("  double: step delta does not match");
            }
        } else {
            #[cfg(debug_assertions)]
            if IS_DEBUG {
                println!("  shift does not match");
            }
        }

        // try ascending, descending position, e.g. BB4 15783962
        let shift = self.sinus_steps[1].pos_middle - self.sinus_steps[0].pos_middle;
        // let mut deltas = Vec::new();
        self.deltas.clear();
        self.deltas
            .push(self.sinus_steps[1].step_id - self.sinus_steps[0].step_id);
        let mut last_step = 1;
        // let mut count = 2;
        // TODO remove skip 2
        for (i, r) in self.sinus_steps.iter().enumerate().skip(2) {
            if r.pos_middle == self.sinus_steps[last_step].pos_middle + shift {
                self.deltas
                    .push(r.step_id - self.sinus_steps[last_step].step_id);
                last_step = i;
            }
        }
        if self.deltas.len() >= 4 {
            let d1 = self.deltas[1] - self.deltas[0];
            let d2 = self.deltas[2] - self.deltas[1];
            let d3 = self.deltas[3] - self.deltas[2];
            if d2 == d1 * 2 && d3 == d2 + d1 * 2 {
                // distance between deltas grows linear by d1 (*1, *2, *3, etc)
                #[cfg(debug_assertions)]
                if IS_DEBUG {
                    // TODO check extension, e.g. 110, 1
                    todo!("Found ascending delta");
                    // println!("Found ascending delta",);
                    // return true
                };
                return false;
            }
        }

        false
    }
}

impl Default for DeciderBouncer {
    fn default() -> Self {
        let step_limit = Config::step_limit_bouncer_default(N_STATES_DEFAULT);
        Self {
            // TODO fine tune capacity. Lower may be faster in general.
            steps: Vec::with_capacity(step_limit as usize),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(step_limit as usize / 4)),
            sinus_steps: Vec::new(),
            sinus_tapes: Vec::new(),
            deltas: Vec::new(),
            deltas2nd: Vec::new(),
            deltas2nd_count: Vec::new(),
            sync_high_bit: 0,
            sync_low_bit: 0,
            expanding_sinus_reason: ExpandingBouncerReason::DeciderNoResult,
            #[cfg(debug_assertions)]
            machine_info: crate::machine_info::MachineInfo::new(
                0,
                crate::transition_symbol2::TransitionTableSymbol2::new_default(0),
                MachineStatus::NoDecision,
            ),
            step_limit,
        }
    }
}

impl Decider for DeciderBouncer {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_BOUNCER_ID
    }

    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        self.decide_machine_main(machine)
    }

    fn decide_single_machine(machine: &Machine, config: &crate::config::Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine_main(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }
}

/// Single Step when run, records the state before to identify loops
#[derive(Clone)]
struct StepExpanding {
    /// Allows quick compare of symbol & state in one step.
    /// Also state == state*2, so full number is the map_id, e.g. C1 would translate to 3*2 + 1 = 7.
    /// symbol: bit 0 (0,1 only, no hold or undefined)  
    /// state: bits 1-4
    for_symbol_state: SymbolStateType,
    tape_before: TapeType,
    high_bound_before: BitType,
    low_bound_before: BitType,
    pos_middle_bit_before: BitType,

    #[cfg(debug_assertions)]
    pub text: [char; 3],
}

impl StepExpanding {
    const FILTER_SYMBOL_PURE: u8 = 0b0000_0001;
    // #[cfg(all(debug_assertions, feature = "bb_debug"))]
    const FILTER_STATE: u8 = 0b0001_1110;
    // const FILTER_SYMBOL: u8 = 0b1100_0000;
    // const FILTER_SYMBOL_STATE: u8 = 0b0001_1111;

    // #[inline]
    fn new(
        transition: TransitionSymbol2,
        for_symbol: SymbolStateType,
        tape_before: TapeType,
        high_bound_before: BitType,
        low_bound_before: BitType,
        pos_middle_bit: BitType,
    ) -> Self {
        Self {
            for_symbol_state: (transition.transition & crate::transition_symbol2::FILTER_STATE)
                as SymbolStateType
                | for_symbol & Self::FILTER_SYMBOL_PURE,
            tape_before,
            high_bound_before,
            low_bound_before,
            pos_middle_bit_before: pos_middle_bit,
            #[cfg(debug_assertions)]
            text: Self::to_chars(transition.state() as SymbolStateType, for_symbol, 0),
        }
    }

    fn is_a0(&self) -> bool {
        self.for_symbol_state & Self::FILTER_STATE == 0b0000_0010
    }

    fn map_id(&self) -> usize {
        self.for_symbol_state as usize
        // ((self.from_symbol_state & Self::FILTER_STATE_STEP) << 1
        //     | (self.from_symbol_state & Self::FILTER_SYMBOL_PURE_STEP) >> 6) as usize
    }

    #[cfg(debug_assertions)]
    fn for_state(&self) -> SymbolStateType {
        self.for_symbol_state & Self::FILTER_STATE
    }

    #[cfg(debug_assertions)]
    fn for_symbol(&self) -> SymbolStateType {
        self.for_symbol_state & Self::FILTER_SYMBOL_PURE
    }

    #[cfg(debug_assertions)]
    fn to_chars(
        for_state: SymbolStateType,
        for_symbol: SymbolStateType,
        direction: BitType,
    ) -> [char; 3] {
        let dir = match direction {
            -1 => 'L',
            1 => 'R',
            _ => '-',
        };
        let state = if for_state & Self::FILTER_STATE == 0 {
            'Z'
        } else {
            (((for_state & Self::FILTER_STATE) >> 1) + b'A' - 1) as char
        };

        [state, (for_symbol + b'0') as char, dir]
    }
}

struct SinusStep {
    step_id: i32,
    pos_middle: BitType,
}

#[cfg(test)]
mod tests {
    use crate::status::MachineStatus;

    use super::*;

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_41399() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 41399 (low bound check)
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("0RC", "1RB"));
        transitions.push(("1RA", "0RA"));

        let machine = Machine::from_string_tuple(41399, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDeltaIdentical
            )),
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_84080() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 84080 (high bound check)
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LA", "0RA"));

        let machine = Machine::from_string_tuple(84080, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_112641() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 112641
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("1LA", "0RA"));

        let machine = Machine::from_string_tuple(112641, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_569564() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 569564
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "0LA"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LB", "1RA"));
        let machine = Machine::from_string_tuple(569564, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        // println!("Result: {}", check_result);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_584567() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 584567 step_delta doubles
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("0RA", "0LB"));
        transitions.push(("1LB", "1RA"));
        let machine = Machine::from_string_tuple(584567, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::TapeValueDeltaAlternating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_1265977() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 1265977 step_delta doubles
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "---"));
        transitions.push(("0LA", "0RB"));
        transitions.push(("1RB", "1LA"));
        let machine = Machine::from_string_tuple(1265977, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::TapeValueDeltaIdentical
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_1970063() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 1970063 step_delta iterates same delta +-
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RB", "0LA"));
        transitions.push(("1RC", "---"));
        transitions.push(("1LA", "1RB"));
        let machine = Machine::from_string_tuple(1970063, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::TapeValueDeltaAlternating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_3044529() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 3044529 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("0LA", "0RC"));
        let machine = Machine::from_string_tuple(3044529, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::TapeValueDeltaIdentical
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_3554911() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 3554911 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "---"));
        transitions.push(("1LC", "1RB"));
        transitions.push(("0RA", "0LC"));
        let machine = Machine::from_string_tuple(3554911, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::TapeValueDeltaAlternating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_6317243() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 Start out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("1RD", "0LC"));
        transitions.push(("1LB", "0RB"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(6317243, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_13318557() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 Start High bound out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("0LD", "1LB"));
        transitions.push(("0LB", "1RC"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(13318557, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_15783962() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 ascending shift with gap and linear growing distance between head pos
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LB", "1RD"));
        transitions.push(("1LC", "---"));
        transitions.push(("1RA", "1LC"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(15783962, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndDoubles
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_32538705() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 sinus, but not with A0
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1LC"));
        transitions.push(("---", "1RC"));
        transitions.push(("1LD", "1RB"));
        transitions.push(("1RA", "0RA"));
        let machine = Machine::from_string_tuple(32538705, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb3_45935166() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 delta of delta rhythm 22, 14, 20 repeats; requires 128-bit tape
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LC", "1LA"));
        transitions.push(("0RD", "---"));
        transitions.push(("1RB", "1LD"));
        transitions.push(("1RA", "0RA"));
        let machine = Machine::from_string_tuple(45935166, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb4_2793430() {
        let config = Config::new_default(4);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 every 2nd step
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "0LD"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("---", "1RA"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(2793430, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_bb4_64379691() {
        let config = Config::new_default(4);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 every steps repeating, but with growing amount of identical steps
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "1RA"));
        transitions.push(("---", "1RD"));
        transitions.push(("1RB", "1LC"));
        transitions.push(("0LA", "0RA"));
        let machine = Machine::from_string_tuple(64379691, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndCompressedRepeating
            ))
        );

        // good example of switched status, else same machine
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "1RA"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("---", "1RD"));
        transitions.push(("0LA", "0RA"));
        let machine = Machine::from_string_tuple(68106631, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_eq!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::StepDelta2ndCompressedRepeating
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_not_bb3_max_651320() {
        let config = Config::new_default(3);
        let mut decider = DeciderBouncer::new(&config);

        // BB3 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("1RB", "0LC"));
        transitions.push(("1RC", "1RA"));
        let machine = Machine::from_string_tuple(651320, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_ne!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::DeciderNoResult
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_not_bb4_max_322636617() {
        let config = Config::new_default(4);
        let mut decider = DeciderBouncer::new(&config);

        // BB4 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        let machine = Machine::from_string_tuple(322636617, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_ne!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::DeciderNoResult
            ))
        );
    }

    #[test]
    fn test_decider_expanding_sinus_applies_not_bb5_max() {
        let config = Config::new_default(5);
        let mut decider = DeciderBouncer::new(&config);

        // BB5 Max
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1LC"));
        transitions.push(("1RC", "1RB"));
        transitions.push(("1RD", "0LE"));
        transitions.push(("1LA", "1LD"));
        transitions.push(("---", "0LA"));
        let machine = Machine::from_string_tuple(0, &transitions);
        let check_result = decider.decide_machine_main(&machine);
        assert_ne!(
            check_result,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingBouncer(
                ExpandingBouncerReason::DeciderNoResult
            ))
        );
    }
}
