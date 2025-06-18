//! This decider does not work correctly. Some work is to be done.
// #![allow(unused)]

use crate::{
    machine::Machine,
    status::{MachineStatus, UndecidedReason},
    transition_symbol2::{TransitionSymbol2, TransitionType, TRANSITION_0RA},
    StepType, MAX_STATES,
};

const STEP_LIMIT_DECIDER_EXPANDING_LOOP: usize = 5000; // STEP_LIMIT;

type TapeType = u64;
type BitType = i8;
// type SymbolStateType = u8;

const TAPE_SIZE_BIT: usize = 64;
const MIDDLE_BIT: BitType = (TAPE_SIZE_BIT / 2 - 1) as BitType;
const POS_HALF: TapeType = 1 << MIDDLE_BIT;

/// This decider checks for loops which are expanded by repeated steps or inner loops which grow in number for each loop. \
/// E.g. BB4 Machine ID 38250788 \
/// A 0LB 1LC \
/// B 0LC --- \
/// C 1LD 1RC \
/// D 1RA 0RA \
/// After start A0, B0 the looping begins: \
/// 1st loop: C0 D0 A1 \[C1 C1] \[\(C0 D1 A1) \(C0 D1 A1)] \
/// 2nd loop: C0 D0 A1 \[C1 C1 C1 C1] \[\(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1)] \
/// 3rd loop: C0 D0 A1 \[C1 C1 C1 C1 C1 C1] \[\(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1) \(C0 D1 A1)] \
/// This will expand endlessly.
pub struct DeciderExpandingLoop {
    steps: Vec<StepLoop>,
    /// stores the step ids for each State-Symbol combination (basically e.g. all from A0 steps)
    // TODO check if storage as u16 is faster
    maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
}

impl DeciderExpandingLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decider_expanding_loop(&mut self, machine: &Machine) -> MachineStatus {
        // #[cfg(any(debug_assertions, test))]
        // {
        //     // if machine.id != DEBUG_MACHINE_NO {
        //     // return MachineStatus::NoDecision;
        //     // }
        //     println!("\nDecider Expanding Loop for {}", machine);
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

        for map in self.maps_1d.iter_mut() {
            map.clear();
        }
        // Initialize transition with A0 as start
        let mut tr = TransitionSymbol2 {
            transition: TRANSITION_0RA,
            #[cfg(debug_assertions)]
            text: ['0', 'R', 'A'],
        };

        // TODO const for size and stop condition
        let mut rel_step_ids = [0; 101];
        // gaps [(relative start gap in loop 1, len gap in loop 2)]
        let mut gaps = vec![];

        // loop over transitions to write tape
        loop {
            // store next step
            let curr_read_symbol = ((tape_shifted & POS_HALF) != 0) as usize; // resolves to one if bit is set

            // maps: store step id leading to this
            let map_id = tr.state_x2() + curr_read_symbol;
            self.maps_1d[map_id].push(self.steps.len());
            // TODO store map_id directly in step as for_symbol_state, much faster. Also, Step can be created without need to update.
            // TODO Why store direction? Could also be retrieved with for_symbol_state.
            // TODO store step_ids as u32
            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            {
                let step = StepLoop::new(tr.transition, curr_read_symbol as i16, tape_shifted);
                self.steps.push(step);
            }
            #[cfg(not(all(debug_assertions, feature = "bb_debug")))]
            self.steps.push(StepLoop {
                for_symbol_state: map_id as TransitionType,
            });

            tr = machine.transition(tr.state_x2() + curr_read_symbol);

            // halt is regarded as step, so always count step
            // check if done
            if tr.is_hold() || self.steps.len() > STEP_LIMIT_DECIDER_EXPANDING_LOOP {
                if self.steps.len() > STEP_LIMIT_DECIDER_EXPANDING_LOOP {
                    return MachineStatus::Undecided(
                        UndecidedReason::StepLimit,
                        self.steps.len() as StepType,
                        TAPE_SIZE_BIT,
                    );
                } else {
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
                    return MachineStatus::DecidedHolds(self.steps.len() as StepType);
                }
            }

            // print step info
            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            {
                // let read_symbol_next = ((tape_shifted & POS_HALF) != 0) as usize;
                let step = self.steps.last().unwrap();
                let s =
                    format!(
                    // "Step {:3}: {}{} {} before: Tape shifted {} H{high_bound}{} L{low_bound}{} P{pos_middle_bit:>3}{} {}", // , next {}{} {}",
                    "Step {:3}: {}{} {} before: Tape shifted {} H{high_bound}{} L{low_bound}{} {}", // , next {}{} {}",
                    self.steps.len() -1,
                    (step.for_state() + 64) as u8 as char,
                    step.for_symbol(),
                    tr,
                    crate::tape_utils::U64Ext::to_binary_split_string(&tape_shifted),
                    if step.is_a0() && high_bound == MIDDLE_BIT {'*'} else {' '},
                    if step.is_a0() && low_bound == MIDDLE_BIT {'*'} else {' '},
                    // if step.is_a0() && pos_middle_bit == MIDDLE_BIT {'*'} else {' '},
                    if step.is_a0() {"A0"} else {""},
                );
                println!("{s}");
                // _ = writeln!(file, "{s}");
            }

            // if self.steps.len() % 100 == 0 {
            //     println!();
            // }

            // check endless expanding loop
            // TODO check only elements which have the correct next element,
            // Step 60 should be good for 3 loops, or 95 for four loops
            let last = self.maps_1d[map_id].len() - 1;
            // Four loops are required for decider, so check element exists at least 5 times (including start of loop 5).
            // The check looks for the 2nd element of loop 5, which are the same two repeating transitions for each loop,
            // disallowing identical elements. last > 3 is element 4 of an array, so 5 elements.
            if last > 3 && self.maps_1d[map_id][last] - self.maps_1d[map_id][last - 1] > 1 {
                'test: loop {
                    // This is not a real loop, it just uses break to avoid too many nesting ifs
                    // Find loops with first two elements identical.
                    let step_id_last_loop = self.maps_1d[map_id][last] - 1;
                    let map_id_loop_1st = self.steps[step_id_last_loop].for_symbol_state as usize;
                    rel_step_ids[0] = step_id_last_loop;
                    // if self.maps_1d[map_id][last] > 93 {
                    //     println!("{step_id_last_loop}");
                    // }

                    if map_id_loop_1st != map_id {
                        // Now loop the known positions of the first element backwards and check if 2nd element is identical.
                        let mut count = 0;
                        for &check_id in self.maps_1d[map_id_loop_1st].iter().rev().skip(1) {
                            if map_id == self.steps[check_id + 1].for_symbol_state as usize {
                                count += 1;
                                rel_step_ids[count] = check_id;
                                // TODO only 4 required?
                                if count == 4 {
                                    break;
                                }
                            }
                        }
                        if count < 4 {
                            break;
                        }
                        // first to second rel_step_id is the reference loop
                        // compare loops
                        rel_step_ids[0..count + 1].reverse();
                        let dist1 = rel_step_ids[1] - rel_step_ids[0];
                        let dist2 = rel_step_ids[2] - rel_step_ids[1];
                        let dist3 = rel_step_ids[3] - rel_step_ids[2];
                        // loop grows each round with an identical number of elements
                        // let dist2nd = dist2 - dist1;
                        if dist2 <= dist1 || dist3 != dist2 + dist2 - dist1 {
                            break;
                        }

                        // find loop deviations
                        gaps.clear();
                        // add +1 to next_loop_start to address next element
                        let next_loop_start = rel_step_ids[1] + 1;
                        let mut gap_steps = 0;
                        let mut i = 0;
                        // add +1 in range to address next element
                        for step_id_check in rel_step_ids[0] + 1..next_loop_start {
                            if self.steps[step_id_check].for_symbol_state
                                != self.steps[next_loop_start + i + gap_steps].for_symbol_state
                            {
                                // loop deviates, find next expected element
                                let expected = self.steps[step_id_check].for_symbol_state;
                                for step_idx in next_loop_start + i + 1..rel_step_ids[2] {
                                    if self.steps[step_idx].for_symbol_state == expected {
                                        #[cfg(all(debug_assertions, feature = "bb_debug"))]
                                        println!(
                                            "  Found gap: {}-{}",
                                            next_loop_start + i + gap_steps,
                                            step_idx - 1
                                        );

                                        let gap_size = step_idx - (next_loop_start + i + gap_steps);
                                        gaps.push((i + 1, gap_size));
                                        gap_steps += gap_size;
                                        break;
                                    }
                                }
                            }
                            i += 1;
                        }
                        // check if 2nd loop has more elements at the end
                        if rel_step_ids[2] > rel_step_ids[1] + i + gap_steps {
                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                            println!(
                                "  Found gap end: {}-{}",
                                rel_step_ids[1] + i + gap_steps,
                                rel_step_ids[2] - 1
                            );

                            let gap_size = rel_step_ids[2] - (rel_step_ids[1] + i + gap_steps);
                            gaps.push((i, gap_size));
                        }
                        // validate gaps in following loop (s? TODO multiple)
                        if !gaps.is_empty() {
                            // Gaps found in loop 2 which are not present in loop 1.
                            // Validate if the gaps are repeating all their elements in loops 3 and 4.
                            // let mut gap_add = 0;
                            // let second_start_step = rel_step_ids[1];
                            // let mut second_pos_rel = 0;
                            // let mut end_pos = 0;
                            for loop_no in 2..4 {
                                // let comp = rel_step_ids[l];
                                let mut second_pos = rel_step_ids[1];
                                let mut comp_pos = rel_step_ids[loop_no];
                                let mut last_gap_0 = 0;
                                // for (gi, gap) in gaps.iter().enumerate() {
                                for gap in gaps.iter() {
                                    // check normal steps between gaps
                                    for i in 0..gap.0 - last_gap_0 {
                                        if self.steps[second_pos + i].for_symbol_state
                                            != self.steps[comp_pos + i].for_symbol_state
                                        {
                                            // step order does not match
                                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                                            println!("  Loop does not match (normal check)!");

                                            break 'test;
                                        }
                                    }
                                    second_pos += gap.0 - last_gap_0;
                                    comp_pos += gap.0 - last_gap_0;
                                    // second_pos_rel += gap.0 + gap.1;
                                    // let end_pos = second_start_step + gap.0 + gap.1 + gap_add;
                                    // // let end_tr = self.steps[end_pos].for_symbol_state;
                                    // println!(
                                    //     "  Loop {l}: gap {:?}, compare loops: {second_start_step} vs. {comp}, end at {}",
                                    //     gap, end_pos
                                    // );
                                    // loop until it is not a repetition any more
                                    // let mut g_add = 0;
                                    let mut repeat_count = 0;
                                    let mut is_match = true;
                                    loop {
                                        for i in 0..gap.1 {
                                            #[cfg(all(debug_assertions, feature = "bb_debug"))]
                                            println!(
                                                "  Loop {loop_no} i:{i}  {}-{}: {},{}: {}=={}, repeat {repeat_count}",
                                                gap.0,
                                                gap.0 + gap.1 - 1,
                                                second_pos + i,
                                                comp_pos + i,
                                                self.steps[second_pos + i].for_symbol_state,
                                                self.steps[comp_pos + i].for_symbol_state
                                            );

                                            if self.steps[second_pos + i].for_symbol_state
                                                != self.steps[comp_pos + i].for_symbol_state
                                            {
                                                // step order does not match
                                                #[cfg(all(debug_assertions, feature = "bb_debug"))]
                                                println!(
                                                    "  Repeat ended after {repeat_count} repititions!"
                                                );

                                                is_match = false;
                                                break;
                                            }
                                            // println!(
                                            //     "  i:{i}  {}-{}: {},{}: {} == {}",
                                            //     gap.0 + g_add,
                                            //     gap.0 + gap.1 + g_add - 1,
                                            //     second_start_step + i,
                                            //     comp + g_add + i,
                                            //     self.steps[second_start_step + i].for_symbol_state,
                                            //     self.steps[comp + g_add + i].for_symbol_state
                                            // );
                                            // if self.steps[second_start_step + i].for_symbol_state
                                            //     != self.steps[comp + g_add + i].for_symbol_state
                                            // {
                                            //     // step order does not match
                                            //     println!("  Loop does not match!");
                                            //     break 'test;
                                            // }
                                        }
                                        if is_match {
                                            comp_pos += gap.1;
                                            repeat_count += 1;
                                        } else {
                                            // second_pos += gap.1;
                                            if repeat_count == 0 {
                                                todo!("loop count 0")
                                            }
                                            break; // break loop and continue with next elements
                                        }
                                        // println!(
                                        //     "  check end expansion at: {}",
                                        //     // comp + gap.0 + gap.1 + g_add
                                        //     comp_pos
                                        // );
                                        // if self.steps[comp_pos].for_symbol_state == end_tr {
                                        //     let mut is_match = true;
                                        //     if gi < gaps.len() - 1 {
                                        //         // check expected steps
                                        //         for i in 0..gaps[gi + 1].0 - gap.0 {
                                        //             if self.steps[second_pos + gap.1 + i]
                                        //                 .for_symbol_state
                                        //                 != self.steps[comp_pos + i].for_symbol_state
                                        //             {
                                        //                 // step order does not match
                                        //                 println!(
                                        //                     "  Loop does not match (continue check)!"
                                        //                 );
                                        //                 is_match = false;
                                        //                 break;
                                        //             }
                                        //         }
                                        //     } else {
                                        //         // check if this is the beginning of the next loop
                                        //         for i in 0..gaps[0].0 {
                                        //             if self.steps[second_pos + gap.1 + i]
                                        //                 .for_symbol_state
                                        //                 != self.steps[comp_pos + i].for_symbol_state
                                        //             {
                                        //                 // step order does not match
                                        //                 println!(
                                        //                     "  Loop does not match (continue check)!"
                                        //                 );
                                        //                 is_match = false;
                                        //                 break;
                                        //             }
                                        //         }
                                        //     }
                                        //     if is_match {
                                        //         second_pos += gap.1;
                                        //         last_gap_0 = gap.0;
                                        //         gap_add += g_add;
                                        //         break;
                                        //     } else {
                                        //         g_add += gap.1;
                                        //     }
                                        // } else {
                                        //     g_add += gap.1;
                                        // }
                                    }
                                    second_pos += gap.1;
                                    last_gap_0 = gap.0;
                                }
                            }
                            // Success, validation absolved
                            return MachineStatus::DecidedEndless(
                                crate::status::EndlessReason::ExpandingLoop,
                            );
                        }
                    } else {
                        todo!("map_id check, why?")
                    }
                    break;
                }
                // // store following steps and their count
                // for (i, &step_id) in self.maps_1d[map_id]
                //     .iter()
                //     .enumerate()
                //     .skip(2)
                //     .rev()
                //     .skip(1)
                // {
                //     let next_step_symbol_state = self.steps[step_id + 1].for_symbol_state;
                //     let mut count = 0;
                //     let mut check_further = false;
                //     for &check_id in self.maps_1d[map_id][0..i].iter() {
                //         if next_step_symbol_state == self.steps[check_id + 1].for_symbol_state {
                //             count += 1;
                //             if count == 4 {
                //                 check_further = true;
                //                 break;
                //             }
                //         }
                //     }
                //     if check_further {
                //         println!();
                //     }
                // }
                // println!();
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
                        println!(
                            "tape shifted {}",
                            crate::tape_utils::U64Ext::to_binary_split_string(&tape_shifted)
                        );
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
                        println!(
                            "tape shifted {}",
                            crate::tape_utils::U64Ext::to_binary_split_string(&tape_shifted)
                        );
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
        }

        // TODO correct status
        // MachineStatus::NoDecision
    }
}

impl Default for DeciderExpandingLoop {
    fn default() -> Self {
        Self {
            steps: Vec::with_capacity(STEP_LIMIT_DECIDER_EXPANDING_LOOP),
            maps_1d: core::array::from_fn(|_| {
                Vec::with_capacity(STEP_LIMIT_DECIDER_EXPANDING_LOOP / 4)
            }),
        }
    }
}

/// Single Step when run, records the state before to identify loops
// TODO remove from_state and from_symbol, only for debugging purposes
// TODO integrate state & symbol in one number and match it with 1D array, so no calc for lookup required, array would have 32 fields
// TODO pub(crate)
struct StepLoop {
    /// Allows quick compare of symbol & state in one step
    /// symbol: bit 0
    /// state: bits 1-4
    for_symbol_state: TransitionType,
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    #[allow(dead_code)]
    pub tape_before: u64,
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    #[allow(dead_code)]
    pub text: [char; 2],
}

impl StepLoop {
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    const FILTER_SYMBOL_PURE: i16 = 0b0000_0001;
    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    const FILTER_STATE: i16 = 0b0001_1110;

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    #[inline]
    pub fn new(
        for_transition: TransitionType,
        for_symbol: TransitionType,
        tape_before: u64,
    ) -> Self {
        Self {
            for_symbol_state: (for_transition & crate::transition_symbol2::FILTER_STATE)
                | for_symbol,
            tape_before,
            #[cfg(all(debug_assertions, feature = "bb_debug"))]
            text: Self::to_chars(for_transition, for_symbol),
        }
    }

    #[cfg(not(all(debug_assertions, feature = "bb_debug")))]
    #[allow(dead_code)]
    #[inline]
    pub fn new(map_id: TransitionType) -> Self {
        Self {
            for_symbol_state: map_id,
        }
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    pub fn is_a0(&self) -> bool {
        self.for_symbol_state & Self::FILTER_STATE == 0b0000_0010
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    pub fn for_state(&self) -> i16 {
        (self.for_symbol_state & Self::FILTER_STATE) >> 1
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    pub fn for_symbol(&self) -> i16 {
        self.for_symbol_state & Self::FILTER_SYMBOL_PURE
    }

    #[cfg(all(debug_assertions, feature = "bb_debug"))]
    fn to_chars(from_state: i16, from_symbol: i16) -> [char; 2] {
        let state = if from_state & crate::transition_symbol2::FILTER_STATE == 0 {
            'Z'
        } else {
            (((from_state & crate::transition_symbol2::FILTER_STATE) >> 2) + 64) as u8 as char
        };

        [state, from_symbol as u8 as char]
    }
}

#[cfg(test)]
mod tests {
    use crate::status::MachineStatus;

    use super::*;

    #[test]
    fn test_decider_expanding_loop_applies_bb4_38250788() {
        let mut decider = DeciderExpandingLoop::new();

        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LB", "1LC"));
        transitions.push(("0LC", "---"));
        transitions.push(("1LD", "1RC"));
        transitions.push(("1RA", "0RA"));
        let machine = Machine::from_string_tuple(38250788, &transitions);
        let status = decider.decider_expanding_loop(&machine);
        // println!("Result: {}", check_result);
        assert_eq!(
            status,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingLoop),
        );
    }

    #[test]
    // TODO make it work
    // TODO probably check the tape_shifted number, it may change here
    fn test_decider_expanding_loop_applies_not_bb5_max() {
        let mut decider = DeciderExpandingLoop::new();

        // BB5 Max, should not apply
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "1LC"));
        transitions.push(("1RC", "1RB"));
        transitions.push(("1RD", "0LE"));
        transitions.push(("1LA", "1LD"));
        transitions.push(("---", "0LA"));
        let machine = Machine::from_string_tuple(0, &transitions);
        // machine.id = 64379691;
        let status = decider.decider_expanding_loop(&machine);
        println!("Result: {}", machine);
        assert_ne!(
            status,
            MachineStatus::DecidedEndless(crate::status::EndlessReason::ExpandingLoop),
        );
    }
}
