/// This is a fast version of the decider loop.
/// It is limited to a u64 shift.

// #[cfg(debug_assertions)]
#[cfg(all(debug_assertions, feature = "bb_debug"))]
use crate::tape_utils::U64Ext;
use crate::{
    config::Config,
    decider::{self, Decider},
    decider_result::BatchResult,
    machine::Machine,
    pre_decider::PreDeciderRun,
    status::{EndlessReason, MachineStatus, UndecidedReason},
    transition_symbol2::{DirectionType, TransitionSymbol2, TransitionType},
    StepType, MAX_STATES,
};

// #[cfg(debug_assertions)]
// const DEBUG_MACHINE_NO: u64 = 84080; // 351902; // 1469538; // 322636617; // BB3 max: 651320; // 46; //

type TapeType = u64;
const TAPE_SIZE_BIT: usize = 64;
// const TAPE_LONG_BYTE: usize = 8;
const MIDDLE_BIT: usize = TAPE_SIZE_BIT / 2 - 1;
const POS_HALF: TapeType = 1 << MIDDLE_BIT;
// const POS_HALF_TEST: u64 = 0b1000_0000_0000_0000_0000_0000_0000_0000;

pub const STEP_LIMIT_DECIDER_LOOP: StepType = 510; // STEP_LIMIT;
pub const MAX_INIT_CAPACITY: usize = 10_000;

// TODO self_ref for loop

#[derive(Debug)]
pub struct DeciderLoopV4 {
    steps: Vec<StepLoop>,
    /// stores the step ids for each State-Symbol combination (basically e.g. all from A0 steps)
    // TODO check if storage as u16 is faster
    maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
    /// Step limit for this decider. Should not exceed 2000 // TODO why
    step_limit: usize,
    // TODO tape_size
}

impl DeciderLoopV4 {
    pub fn new(step_limit: StepType) -> Self {
        // TODO reasoning for size, 510 was for BB5
        let cap = (step_limit as usize).min(MAX_INIT_CAPACITY);
        let step_limit = step_limit as usize;
        Self {
            steps: Vec::with_capacity(cap),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(cap / 4)),
            step_limit,
        }
    }

    fn new_from_self(&self) -> DeciderLoopV4 {
        let cap = self.step_limit.min(MAX_INIT_CAPACITY);
        DeciderLoopV4 {
            steps: Vec::with_capacity(cap),
            maps_1d: core::array::from_fn(|_| Vec::with_capacity(cap / 4)),
            step_limit: self.step_limit,
        }
    }

    // TODO fine tune and other states
    pub fn step_limit(n_states: usize) -> StepType {
        match n_states {
            1 => 100,
            2 => 100,
            3 => 100,
            4 => 300,
            5 => 510,
            _ => panic!("result_max_steps: Not build for this."),
        }
    }
}

impl Decider for DeciderLoopV4 {
    fn new_decider(&self) -> Self {
        Self::new_from_self(&self)
    }

    // tape_long_bits in machine?
    // TODO counter: longest loop
    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        // #[cfg(debug_assertions)]
        // {
        //     if machine.id != DEBUG_MACHINE_NO {
        //         // return MachineStatus::NoDecision;
        //     }
        //     println!("\nDecider Loop for {}", machine.to_string_without_status());
        // }
        // println!("Machine {}", m_info.id);

        // initialize decider

        // num steps, same as steps, but steps can be deactivated after a while
        // let mut steps: Vec<Step> = Vec::with_capacity(STEP_LIMIT_DECIDER_LOOP);
        self.steps.clear();

        // tape for storage in Step with cell before transition at position u32 top bit
        // this tape shifts in every step, so that the head is always at bit 31
        let mut tape_shifted: u64 = 0;
        let mut high_bound = 31;
        let mut low_bound = 31;

        // replaces tape_long as shift happens every 32 bit
        // let mut tape_long_u32 = [0; TAPE_LONG_BYTE * 2];
        // let mut pos_high_long_tape_shifted = TAPE_LONG_BYTE;

        // map for each transition, which step went into it
        // TODO extra level for 0/1 at head position, maybe better calc single array
        // TODO 1D array
        // let mut maps: [[Vec<usize>; 2]; MAX_STATES + 1] =
        //     core::array::from_fn(|_| [Vec::new(), Vec::new()]);
        // let mut maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)] = core::array::from_fn(|_| Vec::new());
        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
        // Initialize transition with A0 as start
        let mut tr = TransitionSymbol2 {
            transition: crate::transition_symbol2::TRANSITION_0RA,
            #[cfg(debug_assertions)]
            text: ['0', 'R', 'A'],
        };

        // The tape ist just an u64 with each bit representing one cell
        // let mut tape: u64 = 0;
        // let mut position_one = POS_HALF;

        // loop over transitions to write tape
        loop {
            // store next step
            let curr_read_symbol = ((tape_shifted & POS_HALF) != 0) as usize; // resolves to one if bit is set

            // maps: store step id leading to this
            // maps[tr.state_next as usize][curr_read_symbol].push(self.steps.len());
            self.maps_1d[tr.state_x2() + curr_read_symbol].push(self.steps.len());
            let mut step = StepLoop::new(
                tr.transition,
                curr_read_symbol as TransitionType,
                0,
                tape_shifted,
            );
            tr = machine.transition(tr.state_x2() + curr_read_symbol);
            step.direction = tr.direction();
            self.steps.push(step);

            // halt is regarded as step, so always count step
            // check if done
            if tr.is_hold() {
                // Hold found
                // write last symbol
                // TODO count ones
                #[allow(unused_assignments)]
                if tr.symbol() < 2 {
                    if tr.is_symbol_one() {
                        tape_shifted |= POS_HALF
                    } else {
                        tape_shifted &= !POS_HALF
                    };
                }
                // println!("Check Loop: ID {}: Steps till hold: {}", m_info.id, steps);
                return MachineStatus::DecidedHolds(self.steps.len() as StepType);
            } else if self.steps.len() >= self.step_limit {
                if self.steps.len() >= self.step_limit {
                    return MachineStatus::Undecided(
                        UndecidedReason::StepLimit,
                        self.step_limit as StepType,
                        TAPE_SIZE_BIT,
                    );
                }
            }

            // update tape: write symbol at head position into cell
            tape_shifted = if tr.is_symbol_one() {
                tape_shifted | POS_HALF
            } else {
                tape_shifted & !POS_HALF
            };

            tape_shifted = if tr.is_dir_right() {
                high_bound += 1;
                if high_bound == 64 {
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!("tape shifted {}", tape_shifted.to_binary_split_string());
                        println!("State: Undecided: Too many steps to right.");
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitLeftBoundReached,
                        self.steps.len() as StepType,
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
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    {
                        println!("\n{}", machine);
                        println!("step         {}", self.steps.len());
                        println!("low_bound    {}", low_bound);
                        println!("high_bound   {}", high_bound);
                        println!("tape shifted {}", tape_shifted.to_binary_split_string());
                        println!("State: Undecided: Too many steps to left.");
                    }
                    return MachineStatus::Undecided(
                        UndecidedReason::TapeLimitRightBoundReached,
                        self.steps.len() as StepType,
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

            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            println!(
                "Step {:3}: {}{} {} Tape shifted after: {:032b} {:032b}, next {}{} {}",
                self.steps.len() - 1,
                (self.steps.last().unwrap().for_state() + 64) as u8 as char,
                self.steps.last().unwrap().for_symbol(),
                tr,
                (tape_shifted >> 32) as u32,
                tape_shifted as u32,
                tr.state_to_char(),
                read_symbol_next,
                machine.transition(tr.state_x2() + read_symbol_next),
            );

            // check endless loop for multiple steps
            if self.maps_1d[tr.state_x2() + read_symbol_next].len() > 1 {
                'steps: for &step_id in self.maps_1d[tr.state_x2() + read_symbol_next][1..]
                    .iter()
                    // .skip(1) // slow
                    .rev()
                {
                    let distance = self.steps.len() - step_id;
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    // #[cfg(debug_assertions)]
                    {
                        println!("  Endless loop check: Step {step_id} with distance {distance}");
                    }

                    // check if we have two repeated loops
                    if distance > step_id {
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  * Fail {step_id}: Min Distance");
                        // step_id will get smaller, distance larger
                        break;
                    }

                    // check loop steps are identical
                    for (i, step) in self.steps.iter().enumerate().skip(step_id) {
                        if step.for_symbol_state != self.steps[i - distance].for_symbol_state {
                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            println!("  * Fail: Loop steps different");
                            continue 'steps;
                        }
                    }

                    // Same, we found a loop candidate!
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    println!(
                        "  *** Loop candidate found: First step {}, distance {distance}!",
                        self.steps.len() - distance
                    );

                    let step_tape_before = self.steps[step_id].tape_before;

                    // #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    // {
                    //     println!("Step Tape     : {}", step_tape.to_binary_split_string());
                    //     println!("Tape shifted  : {}", tape_shifted.to_binary_split_string());
                    // }

                    // check if full tape is identical (this is not necessary, only relevant bytes)
                    // TODO requires comparison of long_tape
                    if step_tape_before == tape_shifted {
                        // Same, we found a loop!
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("*** Found Loop without mask!");
                        return MachineStatus::DecidedEndless(EndlessReason::Loop(
                            self.steps.len() as StepType,
                            distance as StepType,
                        ));
                    }

                    // identify affected bits in the loop steps
                    // this does not work: if a bit left/right is required for the next candidate of same type, then is wrongly cut
                    let mut total_shift = 0;
                    let mut max_r = 0;
                    let mut min_l = 0;
                    // add all steps including next step, because result bit is also relevant
                    for step in self.steps.iter().skip(step_id) {
                        // total_shift += m_info.transitions[step.from_state as usize]
                        //     [step.from_symbol as usize]
                        //     .direction as isize;
                        // let state = step.from_symbol_state & MASK_STATE;
                        // let symbol = step.from_symbol_state & MASK_SYMBOL;
                        // let org = m_info.transitions[(step.from_symbol_state & MASK_STATE) as usize]
                        //     [((step.from_symbol_state & MASK_SYMBOL) >> 6) as usize]
                        //     .direction;
                        // let new = step.direction;
                        // if org != new {
                        //     todo!()
                        // }
                        // total_shift += m_info.transitions
                        //     [(step.from_symbol_state & MASK_STATE) as usize]
                        //     [((step.from_symbol_state & MASK_SYMBOL) >> 6) as usize]
                        //     .direction as isize;
                        total_shift += step.direction as isize;
                        if min_l > total_shift {
                            min_l = total_shift
                        };
                        if max_r < total_shift {
                            max_r = total_shift
                        };
                    }
                    // When shifted, eventually all bits on that side are used after x loops, check all
                    // TODO limit to byte or shift in
                    #[allow(clippy::comparison_chain)]
                    if total_shift > 0 {
                        max_r = 31
                    } else if total_shift < 0 {
                        min_l = -32
                    }
                    // add dir of next step: take(self.steps.len() - step_id + 1)

                    // extract relevant bits and compare (bits counted from right, starting with 0, middle is bit 31)
                    let start_bit = 31 - max_r;
                    let end_bit = 31 - min_l; // Inclusive
                    let num_bits = end_bit - start_bit + 1;
                    // Create the mask for the lowest 'num_bits' bits.
                    //    (1 << 10) gives 0b10000000000 (1 followed by 10 zeros)
                    //    Subtracting 1 gives 0b01111111111 (10 ones) -> 0x3FF in hex
                    let mask = ((1u64 << num_bits) - 1) << start_bit;
                    // #[cfg(feature = "bb_debug")]
                    #[cfg(all(debug_assertions, feature = "bb_debug"))]
                    {
                        // for i in step_id - distance..step_id {
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
                        // Same, we found a loop!#
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  *** Found Loop with mask!");
                        return MachineStatus::DecidedEndless(EndlessReason::Loop(
                            self.steps.len() as StepType,
                            distance as StepType,
                        ));
                    } else {
                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                        println!("  * Fail: Mask");
                    }
                }
            }
        }
    }

    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus {
        let mut d = Self::new(Self::step_limit(config.n_states()));
        d.decide_machine(machine)
    }

    fn decider_run_batch(
        machines: &[Machine],
        run_predecider: PreDeciderRun,
        config: &Config,
    ) -> Option<BatchResult> {
        let decider = Self::new(Self::step_limit(config.n_states()));
        decider::decider_generic_run_batch(decider, machines, run_predecider, config)
    }

    fn name(&self) -> &str {
        "Decider Loop V4"
    }
}

// impl Default for DeciderLoopV4CompactP {
//     fn default() -> Self {
//         Self {
//             steps: Vec::with_capacity(STEP_LIMIT_DECIDER_LOOP),
//             maps_1d: core::array::from_fn(|_| Vec::with_capacity(STEP_LIMIT_DECIDER_LOOP / 4)),
//         }
//     }
// }

/// Single Step when run, records the state before to identify loops
// TODO remove from_state and from_symbol, only for debugging purposes
// TODO integrate state & symbol in one number and match it with 1D array, so no calc for lookup required, array would have 32 fields
// TODO pub(crate)
#[derive(Debug)]
pub struct StepLoop {
    /// Allows quick compare of symbol & state in one step
    /// symbol: bit 0
    /// state: bits 1-4
    for_symbol_state: TransitionType,
    /// step goes to this direction, which is the result from symbol_state lookup
    direction: DirectionType,
    tape_before: u64,
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    #[allow(dead_code)]
    text: [char; 3],
}

impl StepLoop {
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    const FILTER_SYMBOL_PURE: i16 = 0b0000_0001;
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
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
            for_symbol_state: (for_transition & crate::transition_symbol2::FILTER_STATE)
                | for_symbol,
            direction,
            tape_before,
            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            text: Self::to_chars(for_transition, for_symbol, direction),
        }
    }

    // fn is_a0(&self) -> bool {
    //     self.for_symbol_state & Self::FILTER_STATE == 0b0000_0010
    // }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    fn for_state(&self) -> i16 {
        (self.for_symbol_state & Self::FILTER_STATE) >> 1
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    fn for_symbol(&self) -> i16 {
        self.for_symbol_state & Self::FILTER_SYMBOL_PURE
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    fn to_chars(from_state: i16, from_symbol: i16, direction: i16) -> [char; 3] {
        let dir = match direction {
            -1 => 'L',
            1 => 'R',
            _ => '-',
        };
        let state = if from_state & crate::transition_symbol2::FILTER_STATE == 0 {
            'Z'
        } else {
            (((from_state & crate::transition_symbol2::FILTER_STATE) >> 2) + 64) as u8 as char
        };

        [state, from_symbol as u8 as char, dir]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn decider_loop_v4_compact_holds_after_107_steps() {
        // check does not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "1LC"));
        transitions.push(("---", "1LD"));
        transitions.push(("1LA", "0LB"));
        transitions.push(("1RD", "0RA"));
        transitions.push(("0RA", "0RA"));

        let p = Machine::from_string_tuple(0, &transitions);
        let mut d = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
        let machine_status = d.decide_machine(&p);
        assert_eq!(machine_status, MachineStatus::DecidedHolds(107));
    }

    #[test]
    fn decider_loop_v4_compact_unspecified() {
        // free test without expected result
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1LC"));
        transitions.push(("---", "1RC"));
        transitions.push(("1LD", "1RB"));
        transitions.push(("1RA", "0RA"));

        let machine = Machine::from_string_tuple(32538705, &transitions);
        let mut d = DeciderLoopV4::new(STEP_LIMIT_DECIDER_LOOP);
        let machine_status = d.decide_machine(&machine);
        println!("result: {}", machine_status);
        let ok = match machine_status {
            MachineStatus::Undecided(_, _, _) => true,
            _ => false,
        };
        assert!(ok);
    }
}
