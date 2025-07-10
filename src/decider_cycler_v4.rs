// TODO document
//! This is the fast version of the cycler decider (limited to a 64-bit tape, so it may not find larger cycles). \
//! It is a very effective decider and should run first with a small number of steps to eliminate most \
//! of the cyclers and machines which hold quickly (both are identified).
//! Example for BB4 with 6,975,757,441 machines. If only this decider is run with 150 step limit, then all
//! machines can be checked in less than 5 seconds, leaving only 65,500 undecided.
//! Most machines are eliminated by the pre-decider, leaving only 30,199,552 machines which need to be checked further.
//! This decider than classifies 10,758,178 as Hold and 19,375,874 as Cycler.
//! Of the 10,758,178 Hold machines, only 184 machines run more than 50 steps.
//! Of the 19,375,874 Cycler machines, only 4740 machines are not detected within 50 steps.
//! If the limit is set to 10,000 machines, then the runtime will be around 110 seconds (on my machine), \
//! which is a factor of 20. \
//! Additionally 160 Cyclers are found. Which means < 0,01% of the machines take 95% of the time.\
//! For BB5 the first 100,000,000,000 can be tested in about 30 seconds, leaving only 214,996 undecided \
//! with step limit 500.
//! Therefore it is wiser to run the bouncer before attempting to catch the other cyclers.
//! How it works: \
//! When run, every step is recorded (StepCycler) so repeating steps can be identified.
//! A map is created for all table fields which stores the steps which used this table field, \
//! e.g. A0 was used in step 0, 14, 28, 42 etc.
//! In this case when 28 is found, all steps will be compared between 0 to 14 and 14 to 28 and \
//! checked if each step is identical. \
//! If this is the case then also the tape will be compared. It needs to match for the \
//! relevant part, meaning all cells touched in this cycle will be compared.

#[cfg(feature = "bb_enable_html_reports")]
use std::{io::Write, path::MAIN_SEPARATOR_STR};

#[cfg(feature = "bb_enable_html_reports")]
use crate::html;
#[cfg(feature = "bb_enable_html_reports")]
use crate::transition_symbol2::TransitionSymbol2;
use crate::{
    config::{Config, StepTypeBig, StepTypeSmall, MAX_STATES},
    decider::{self, Decider, DECIDER_CYCLER_ID},
    decider_result::BatchData,
    machine::Machine,
    status::{EndlessReason, MachineStatus, UndecidedReason},
    transition_symbol2::{DirectionType, TransitionType, TRANSITION_SYM2_START},
    ResultUnitEndReason,
};

const DEBUG_EXTRA: bool = true;
const DEBUG_MIN_DISTANCE: usize = 75;

type TapeType = u64;
const TAPE_SIZE_BIT: StepTypeSmall = 64;
const MIDDLE_BIT: StepTypeSmall = TAPE_SIZE_BIT / 2 - 1;
const POS_HALF: TapeType = 1 << MIDDLE_BIT;

pub const MAX_INIT_CAPACITY: usize = 10_000;

// TODO for clarity move tr and tape_shifted into this struct
// TO DO Performance: Check if really all found duplicates must be checked, maybe some learnings from previous.
#[derive(Debug)]
pub struct DeciderCyclerV4 {
    /// Store all steps to do comparisons (test if a cycle is repeating)
    steps: Vec<StepCycler>,
    /// Stores the step ids (2 = 3rd step) for each field in the transition table. \
    /// (basically e.g. all steps for e.g. field 'B0' steps: 1 if A0 points to B, as step 1 then has state B and head symbol 0.)
    // TODO performance: check if storage as u16 is faster
    // TODO performance: extra level for 0/1 at head position? , maybe better calc single array
    maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
    /// Step limit for this decider. Should not exceed 2000 // TODO why: u64 tape, cannot be so large
    step_limit: StepTypeSmall,
    tr_field_id: usize,
    #[cfg(feature = "bb_enable_html_reports")]
    write_html: bool,
    #[cfg(feature = "bb_enable_html_reports")]
    path: String,
    #[cfg(feature = "bb_enable_html_reports")]
    file: Option<std::fs::File>,
}

impl DeciderCyclerV4 {
    pub fn new(config: &Config) -> Self {
        let cap = (config.step_limit_cycler() as usize).min(MAX_INIT_CAPACITY);
        Self {
            steps: Vec::with_capacity(cap),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(cap / 4)),
            step_limit: config.step_limit_cycler(),
            tr_field_id: 0,
            #[cfg(feature = "bb_enable_html_reports")]
            write_html: config.write_html_file(),
            #[cfg(feature = "bb_enable_html_reports")]
            path: Self::get_html_path(config.write_html_file(), config.n_states()),
            #[cfg(feature = "bb_enable_html_reports")]
            file: None,
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.steps.clear();
        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_file_end(&mut self) {
        if let Some(file) = self.file.as_mut() {
            html::write_file_end(file).expect("Html file could not be written")
        }
    }

    fn decide_machine_cycler(&mut self, machine: &Machine) -> MachineStatus {
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

        // initialize decider
        self.clear();

        // tape for storage in Step with cell before transition at position u32 top bit
        // this tape shifts in every step, so that the head is always at bit 31
        let mut tape_shifted: u64 = 0;
        let mut high_bound = 31;
        let mut low_bound = 31;

        // Initialize transition with A0 as start
        let mut tr = TRANSITION_SYM2_START;

        // loop over transitions to write tape
        loop {
            // read symbol at tape head
            let curr_read_symbol = ((tape_shifted & POS_HALF) != 0) as usize; // resolves to one if bit is set
            self.tr_field_id = tr.state_x2() + curr_read_symbol;

            // store next step
            // map for each transition, which step went into it
            // maps: store step id leading to this
            self.maps_1d[self.tr_field_id].push(self.steps.len());
            let mut step = StepCycler::new(
                tr.transition,
                curr_read_symbol as TransitionType,
                0,
                tape_shifted,
            );
            tr = machine.transition(self.tr_field_id);
            step.direction = tr.direction();
            self.steps.push(step);

            // check if done
            if tr.is_hold() {
                // Hold found
                // TODO count ones
                #[allow(unused_assignments)]
                if tr.symbol() < 2 {
                    // write last symbol
                    if tr.is_symbol_one() {
                        tape_shifted |= POS_HALF
                    } else {
                        tape_shifted &= !POS_HALF
                    };
                }
                // println!("Check Cycle: ID {}: Steps till hold: {}", m_info.id, steps);
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html {
                    self.write_step_html(&tr, tape_shifted);
                    self.write_html_p(
                        format!("Decided: Holds after {} steps.", self.steps.len()).as_str(),
                    );
                }
                return MachineStatus::DecidedHolds(self.steps.len() as StepTypeBig);
            } else if self.steps.len() as StepTypeSmall >= self.step_limit {
                #[cfg(feature = "bb_enable_html_reports")]
                if self.write_html {
                    self.write_step_html(&tr, tape_shifted);
                    self.write_html_p(
                        format!("Undecided: Limit of {} steps reached.", self.step_limit).as_str(),
                    );
                }
                return MachineStatus::Undecided(
                    UndecidedReason::StepLimit,
                    self.step_limit as StepTypeBig,
                    TAPE_SIZE_BIT,
                );
            }

            // update tape: write symbol at head position into cell
            tape_shifted = if tr.is_symbol_one() {
                tape_shifted | POS_HALF
            } else {
                tape_shifted & !POS_HALF
            };

            // check if tape bound is reached
            tape_shifted = if tr.is_dir_right() {
                high_bound += 1;
                if high_bound == 64 {
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!("tape shifted {}", tape_shifted.to_binary_split_string());
                        println!("State: Undecided: Too many steps to right.");
                        // panic!("State: Undecided: Too many steps to right.");
                    }
                    #[cfg(feature = "bb_enable_html_reports")]
                    if self.write_html {
                        html::write_html_p(
                            self.file.as_mut().unwrap(),
                            "Undecided: Too many steps to right.",
                        );
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitLeftBoundReached,
                        self.steps.len() as StepTypeBig,
                        TAPE_SIZE_BIT,
                    );
                }
                if low_bound < 31 {
                    low_bound += 1;
                }
                tape_shifted << 1
            } else {
                low_bound -= 1;
                if low_bound == -1 {
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!("tape shifted {}", tape_shifted.to_binary_split_string());
                        println!("State: Undecided: Too many steps to left.");
                        // panic!("State: Undecided: Too many steps to left.");
                    }
                    #[cfg(feature = "bb_enable_html_reports")]
                    if self.write_html {
                        html::write_html_p(
                            self.file.as_mut().unwrap(),
                            "Undecided: Too many steps to left.",
                        );
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitRightBoundReached,
                        self.steps.len() as StepTypeBig,
                        TAPE_SIZE_BIT,
                    );
                }
                if high_bound > 31 {
                    high_bound -= 1;
                }
                tape_shifted >> 1
            };

            // get next transition
            let read_symbol_next = ((tape_shifted & POS_HALF) != 0) as usize; // resolves to one if bit is set

            // print steps
            #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
            println!(
                "Step {:3}: {} {} Tape shifted after: {:032b} {:032b}, next {}{} {}",
                self.steps.len() - 1,
                self.steps.last().unwrap().for_state_symbol_to_string(),
                tr,
                (tape_shifted >> 32) as u32,
                tape_shifted as u32,
                tr.state_to_char(),
                read_symbol_next,
                machine.transition(tr.state_x2() + read_symbol_next),
            );
            #[cfg(feature = "bb_enable_html_reports")]
            if self.write_html {
                self.write_step_html(&tr, tape_shifted);
            }

            // check endless cycle for multiple steps
            if self.maps_1d[tr.state_x2() + read_symbol_next].len() > 1 {
                'steps: for &step_id in self.maps_1d[tr.state_x2() + read_symbol_next][1..]
                    .iter()
                    // .skip(1) // slow
                    .rev()
                {
                    let distance = self.steps.len() - step_id;
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        println!("  Endless cycle check: Step {step_id} with distance {distance}");
                    }

                    // check if we have two repeated cycles
                    if distance > step_id {
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("  * Fail {step_id}: Min Distance");
                        // step_id will get smaller, distance larger
                        break;
                    }

                    // check cycle steps are identical
                    for (i, step) in self.steps.iter().enumerate().skip(step_id) {
                        if step.for_state_symbol != self.steps[i - distance].for_state_symbol {
                            #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                            println!("  * Fail: Cycle steps different");
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
                                self.steps.len() - distance
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
                    if step_tape_before == tape_shifted {
                        // Same, we found a cycle!
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("*** Found Cycle (tape identical)!");
                        #[cfg(feature = "bb_enable_html_reports")]
                        if self.write_html {
                            let text = format!(
                                "  Decided: Found Cycle (tape identical): Start {} and {}, length: {distance}", 
                                step_id-distance+1,
                                step_id+1
                            );
                            self.write_html_p(&text);
                        }
                        #[cfg(debug_assertions)]
                        if DEBUG_EXTRA {
                            if distance >= DEBUG_MIN_DISTANCE {
                                println!(
                                    "cycle size = {}, current step = {}: M {}",
                                    distance,
                                    self.steps.len(),
                                    machine
                                );
                            }
                        }
                        return MachineStatus::DecidedEndless(EndlessReason::Cycle(
                            self.steps.len() as StepTypeSmall,
                            distance as StepTypeSmall,
                        ));
                    }

                    // identify affected bits in the cycle steps
                    let mut total_shift = 0;
                    let mut max_r = 0;
                    let mut min_l = 0;
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
                        max_r = 31
                    } else if total_shift < 0 {
                        min_l = -32
                    }

                    // extract relevant bits and compare (bits counted from right, starting with 0, middle is bit 31)
                    let start_bit = 31 - max_r;
                    let end_bit = 31 - min_l; // Inclusive
                    let num_bits = end_bit - start_bit + 1;
                    // Create the mask for the lowest 'num_bits' bits.
                    //    (1 << 10) gives 0b10000000000 (1 followed by 10 zeros)
                    //    Subtracting 1 gives 0b01111111111 (10 ones) -> 0x3FF in hex
                    let mask = ((1u64 << num_bits) - 1) << start_bit;
                    // #[cfg(feature = "bb_debug_cycler")]
                    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                    {
                        for (i, step) in self.steps.iter().enumerate().skip(step_id) {
                            let t = machine.transition(
                                step.for_state() as usize * 2 + step.for_symbol() as usize,
                            );
                            println!(
                                "   Step {i:3}: {}{} {}: {}",
                                (step.for_state() + 64) as u8 as char,
                                step.for_symbol(),
                                t,
                                step.tape_before.to_binary_split_string()
                            );
                        }
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
                    if step_tape_before & mask == tape_shifted & mask {
                        // Same, we found a cycle!
                        #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
                        println!("  *** Found Cycle with mask!");
                        #[cfg(feature = "bb_enable_html_reports")]
                        if self.write_html {
                            let text =
                                format!("  Decided: Found Cycle (tape for relevant part identical): Start {} and {}, length: {distance}", step_id-distance+1,step_id+1);
                            self.write_html_p(&text);
                        }
                        #[cfg(debug_assertions)]
                        if DEBUG_EXTRA {
                            if distance >= DEBUG_MIN_DISTANCE {
                                println!(
                                    "cycle size = {}, current step = {}: M {}",
                                    distance,
                                    self.steps.len(),
                                    machine
                                );
                            }
                        }
                        return MachineStatus::DecidedEndless(EndlessReason::Cycle(
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

    #[cfg(feature = "bb_enable_html_reports")]
    fn decide_machine_cycler_html(&mut self, machine: &Machine) -> MachineStatus {
        let (file, file_name) =
            html::create_html_file_start(&self.path, Self::decider_id().name, machine)
                .expect("Html file could not be written");
        self.file = Some(file);

        let ms = self.decide_machine_cycler(machine);
        self.write_file_end();
        // close the file so it can be renamed
        self.file = None;
        // rename file depending on status
        // TODO generalize and use for other deciders
        match ms {
            MachineStatus::NoDecision => todo!(),
            MachineStatus::EliminatedPreDecider(_) => todo!(),
            MachineStatus::Undecided(_, _, _) => {
                // rename file
                let f_name_new = "undecided_".to_string() + &file_name;
                let old_path = format!("{}{}{}", self.path, MAIN_SEPARATOR_STR, file_name);
                let new_path = format!("{}{}{}", self.path, MAIN_SEPARATOR_STR, f_name_new);
                std::fs::rename(old_path, new_path).expect("Could not rename file");
            }
            _ => {
                // rename file
                let f_name_new = "decided_".to_string() + &file_name;
                let old_path = format!("{}{}{}", self.path, MAIN_SEPARATOR_STR, file_name);
                let new_path = format!("{}{}{}", self.path, MAIN_SEPARATOR_STR, f_name_new);
                // println!("{old_path}");
                // let x = std::fs::exists(&old_path);
                std::fs::rename(old_path, new_path).expect("Could not rename file");
            }
        }

        ms
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_html_p(&self, text: &str) {
        writeln!(self.file.as_ref().unwrap(), "<p>{text}</p>",).expect("Html write error");
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn write_step_html(&mut self, transition: &TransitionSymbol2, tape_shifted: u64) {
        html::write_step_html_64(
            self.file.as_mut().unwrap(),
            self.steps.len(),
            self.tr_field_id,
            transition,
            tape_shifted,
        );
    }

    #[cfg(feature = "bb_enable_html_reports")]
    fn get_html_path(write_html: bool, n_states: usize) -> String {
        if write_html {
            let p = format!(
                "{}{}{}{n_states}",
                Config::get_result_path(),
                MAIN_SEPARATOR_STR,
                "cycler_bb",
            );
            html::create_css(&p).expect("CSS files could not be created.");
            p
        } else {
            String::new()
        }
    }
}

impl Decider for DeciderCyclerV4 {
    fn decider_id() -> &'static decider::DeciderId {
        &DECIDER_CYCLER_ID
    }

    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        #[cfg(feature = "bb_enable_html_reports")]
        if self.write_html {
            self.decide_machine_cycler_html(machine)
        } else {
            self.decide_machine_cycler(machine)
        }
        #[cfg(not(feature = "bb_enable_html_reports"))]
        self.decide_machine_cycler(machine)
    }

    // tape_long_bits in machine?
    // TODO counter: longest cycle

    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch_v2(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }
}

/// Record of every step to identify cycles.
#[derive(Debug)]
pub struct StepCycler {
    /// Allows quick compare of symbol & state in one step
    /// symbol: bit 0
    /// state: bits 1-4
    // TO DO performance: could possibly be tr_field_id instead, probably no gain
    pub for_state_symbol: TransitionType,
    /// step goes to this direction, which is the result from symbol_state lookup
    pub direction: DirectionType,
    pub tape_before: u64,
    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    #[allow(dead_code)]
    text: [char; 3],
}

impl StepCycler {
    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    const FILTER_SYMBOL_PURE: i16 = 0b0000_0001;
    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    const FILTER_STATE: i16 = 0b0001_1110;
    // const FILTER_SYMBOL: u8 = 0b1100_0000;
    // const FILTER_SYMBOL_STATE: u8 = 0b0001_1111;

    #[inline]
    pub fn new(
        for_transition: TransitionType,
        for_symbol: TransitionType,
        direction: DirectionType,
        tape_before: u64,
    ) -> Self {
        Self {
            for_state_symbol: (for_transition & crate::transition_symbol2::FILTER_STATE)
                | for_symbol,
            direction,
            tape_before,
            #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
            text: Self::to_chars(for_transition, for_symbol, direction),
        }
    }

    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    pub fn for_state(&self) -> i16 {
        (self.for_state_symbol & Self::FILTER_STATE) >> 1
    }

    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    pub fn for_symbol(&self) -> i16 {
        self.for_state_symbol & Self::FILTER_SYMBOL_PURE
    }

    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    pub fn for_state_symbol_to_string(&self) -> String {
        let mut s = String::with_capacity(2);
        s.push((self.for_state() as u8 + b'A' - 1) as char);
        s.push((self.for_symbol() as u8 + b'0') as char);
        s
    }

    #[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
    fn to_chars(from_state: i16, from_symbol: i16, direction: i16) -> [char; 3] {
        let dir = match direction {
            -1 => 'L',
            1 => 'R',
            _ => '-',
        };
        let state = if from_state & crate::transition_symbol2::FILTER_STATE == 0 {
            'Z'
        } else {
            (((from_state & crate::transition_symbol2::FILTER_STATE) >> 1) as u8 + b'A' - 1) as char
        };

        [state, from_symbol as u8 as char, dir]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_cycler_v4_compact_holds_after_107_steps() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        transitions.push(("0RA", "0RA"));

        let machine = Machine::from_string_tuple(0, &transitions);
        let config = Config::new_default(machine.n_states());
        let mut d = DeciderCyclerV4::new(&config);
        let machine_status = d.decide_machine(&machine);
        assert_eq!(machine_status, MachineStatus::DecidedHolds(107));
    }

    #[test]
    fn decider_cycler_v4_compact_unspecified() {
        // free test without expected result
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1LC"));
        transitions.push(("---", "1RC"));
        transitions.push(("1LD", "1RB"));
        transitions.push(("1RA", "0RA"));

        let machine = Machine::from_string_tuple(32538705, &transitions);
        let config = Config::new_default(machine.n_states());
        let mut d = DeciderCyclerV4::new(&config);
        let machine_status = d.decide_machine(&machine);
        println!("result: {}", machine_status);
        let ok = match machine_status {
            MachineStatus::Undecided(_, _, _) => true,
            _ => false,
        };
        assert!(ok);
    }
}
