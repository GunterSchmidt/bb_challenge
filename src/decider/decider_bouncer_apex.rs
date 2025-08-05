//! This is a simple decider bouncer with speed-up logic.\
//! However, since most is already done by predecider and cycler, not many are left for the bouncer.
//! Of those, about 90% are caught in a few steps, so the speed-up is not measurable.
//! More importantly, this logic does not catch all bouncers and some are not caught due to tape size limitations.

use std::fmt::Display;

#[cfg(all(debug_assertions, feature = "bb_debug_cycler"))]
use crate::tape_utils::U128Ext;
use crate::{
    config::Config,
    decider::{
        self,
        decider_data_apex::DeciderDataApex,
        decider_result::{BatchData, ResultUnitEndReason},
        Decider,
    },
    machine::Machine,
    status::{EndlessReason, MachineStatus},
    tape::tape_utils::{U64Ext, TAPE_SIZE_BIT_U128},
};

// #[cfg(debug_assertions)]
// const DEBUG_EXTRA: bool = false;

/// Initial capacity for step recorder. Not so relevant.
const MAX_INIT_CAPACITY: usize = 10_000;

// TODO Use long tape, or tape_shifted left & right bound could be introduced.
#[derive(Debug)]
pub struct DeciderBouncerApex {
    data: DeciderDataApex,
    /// Store all steps to do comparisons (test if a cycle is repeating)
    /// All even are lower bits, all odd upper bits
    apex_left: Vec<ApexStep>,
    apex_right: Vec<ApexStep>,
    max_right: i64,
    max_right_ids: Vec<usize>,
    min_right: i64,
    min_right_ids: Vec<usize>,
    is_rhythmic_right: bool,
    max_left: i64,
    max_left_ids: Vec<usize>,
    min_left: i64,
    min_left_ids: Vec<usize>,
    is_rhythmic_left: bool,
    // / Stores the step ids (2 = 3rd step) for each field in the transition table. \
    // / (basically e.g. all steps for e.g. field 'B0' steps: 1 if A0 points to B, as step 1 then has state B and head symbol 0.)
    // TODO performance: extra differentiation for 0/1 at head position? The idea is, that the field cannot be identical if head read is different
    // maps_1d: [Vec<usize>; 2 * (MAX_STATES + 1)],
    is_self_ref: bool,
    // last_pos_head: i64,
    // last_tr: TransitionSymbol2,
}

impl DeciderBouncerApex {
    /// Creates a new bouncer. Only uses step_limit_bouncer from config.
    pub fn new(config: &Config) -> Self {
        let cap = (config.step_limit_bouncer() as usize).min(MAX_INIT_CAPACITY);
        let mut decider = Self {
            data: DeciderDataApex::new(config),

            apex_right: Vec::with_capacity(cap),
            max_right: 1,
            max_right_ids: Vec::new(),
            min_right: -1,
            min_right_ids: Vec::new(),
            is_rhythmic_right: false,

            apex_left: Vec::with_capacity(cap),
            max_left: 1,
            max_left_ids: Vec::new(),
            min_left: -1,
            min_left_ids: Vec::new(),
            is_rhythmic_left: false,
            is_self_ref: false,
        };
        decider.data.step_limit = config.step_limit_bouncer();

        #[cfg(feature = "bb_enable_html_reports")]
        {
            decider
                .data
                .set_path_option(crate::html::get_html_path("bouncer", config));
        }

        decider
    }

    // // This assumes the last apex pos equals current pos.
    // fn get_ids_for_pos_identical(&self, check_left: bool, pos: i64) -> Option<Vec<usize>> {
    //     // check all pos are identical
    //     let apexes = if check_left {
    //         &self.apex_left
    //     } else {
    //         &self.apex_right
    //     };
    //     let mut v = Vec::new();
    //     for (i, _apex) in apexes
    //         .iter()
    //         .enumerate()
    //         .filter(|(_i, a)| a.pos_head == pos)
    //     {
    //         v.push(i);
    //     }

    //     // too many entries, filter first after other apex
    //     if v.len() >= 20 {
    //         // TODO possibly easier to have two functions for left and right
    //         let a_other;
    //         let max_min_ids_other;
    //         let is_rhythmic_other = if check_left {
    //             a_other = &self.apex_right;
    //             max_min_ids_other = &self.max_right_ids;
    //             self.is_rhythmic_right
    //         } else {
    //             a_other = &self.apex_left;
    //             max_min_ids_other = &self.min_left_ids;
    //             self.is_rhythmic_left
    //         };
    //         if is_rhythmic_other
    //             && max_min_ids_other.len() >= 5
    //             && apexes.last().unwrap().step_no > ap
    //         {
    //             // check enough entries

    //             println!()
    //         }
    //     }
    //     // Only return Some if at least 4 elements are available for comparison
    //     if v.len() >= 4 {
    //         Some(v)
    //     } else {
    //         None
    //     }
    // }

    // This assumes the last apex pos equals current pos.
    fn get_ids_for_pos_identical_left(&self, pos: i64) -> Option<Vec<usize>> {
        // check all pos are identical
        let mut v = Vec::new();
        for (i, _apex) in self
            .apex_left
            .iter()
            .enumerate()
            .filter(|(_i, a)| a.pos_head == pos)
        {
            v.push(i);
        }

        // too many entries, filter first after other apex
        if v.len() >= 10 {
            if self.is_rhythmic_right
                && self.max_right_ids.len() >= 5
                // check enough entries
                && self.apex_left.last().unwrap().step_no
                    > self.apex_right[*self.max_right_ids.last().unwrap()].step_no
            {
                let mut ids = Vec::new();
                let start = self.max_right_ids.len() - 5;
                let mut i = 0;
                for &r_id in self.max_right_ids[start..].iter() {
                    let step_other = self.apex_right[r_id].step_no;
                    while i < self.apex_left.len() {
                        if self.apex_left[i].step_no > step_other {
                            ids.push(i);
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                }
                if ids.len() == 5 {
                    // check correct relation
                    for i in 0..4 {
                        if ids[i] > self.max_right_ids[start + i + 1] {
                            todo!()
                        }
                    }
                    return Some(ids);
                }
            }
        }
        // Only return Some if at least 4 elements are available for comparison
        if v.len() >= 4 {
            Some(v)
        } else {
            None
        }
    }
    // This assumes the last apex pos equals current pos.
    fn get_ids_for_pos_identical_right(&self, pos: i64) -> Option<Vec<usize>> {
        // check all pos are identical
        let mut v = Vec::new();
        for (i, _apex) in self
            .apex_right
            .iter()
            .enumerate()
            .filter(|(_i, a)| a.pos_head == pos)
        {
            v.push(i);
        }

        // too many entries, filter first after other apex
        if v.len() >= 10 {
            if self.is_rhythmic_left
                && self.min_left_ids.len() >= 5
                // check enough entries
                && self.apex_right.last().unwrap().step_no
                    > self.apex_left[*self.min_left_ids.last().unwrap()].step_no
            {
                let mut ids = Vec::new();
                let start = self.min_left_ids.len() - 5;
                let mut i = 0;
                for &r_id in self.min_left_ids[start..].iter() {
                    let step_other = self.apex_left[r_id].step_no;
                    while i < self.apex_right.len() {
                        if self.apex_right[i].step_no > step_other {
                            ids.push(i);
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                }
                if ids.len() == 5 {
                    // check correct relation
                    for i in 0..4 {
                        if ids[i] > self.min_left_ids[start + i + 1] {
                            todo!()
                        }
                    }
                    return Some(ids);
                }
            }
        }

        // Only return Some if at least 4 elements are available for comparison
        if v.len() >= 4 {
            Some(v)
        } else {
            None
        }
    }
}

impl Decider for DeciderBouncerApex {
    fn decider_id() -> &'static decider::DeciderId {
        // &DECIDER_BOUNCER_ID
        &decider::DeciderId {
            id: 22,
            name: "Decider Bouncer Apex",
        }
    }

    fn decide_machine(&mut self, machine: &Machine) -> MachineStatus {
        // initialize decider
        self.data.clear();
        self.apex_left.clear();
        self.apex_right.clear();
        // skip 1
        self.max_right = 1;
        self.max_right_ids.clear();
        self.min_right = -1;
        self.min_right_ids.clear();
        self.is_rhythmic_right = false;
        self.max_left = 1;
        self.max_left_ids.clear();
        self.min_left = -1;
        self.min_left_ids.clear();
        self.is_rhythmic_left = false;
        // self.last_pos_head = 0;

        self.data.transition_table = *machine.transition_table();
        self.is_self_ref = self.data.transition_table.has_self_referencing_transition();
        // let mut last_left_empty_step_no = 0;
        // let mut last_right_empty_step_no = 0;
        // let mut is_bouncing_right = false;

        #[cfg(feature = "bb_enable_html_reports")]
        self.data.write_html_file_start(Self::decider_id(), machine);

        // loop over transitions to write tape
        let mut last_right_apex_step_no = 0;
        let mut last_left_apex_step_no = 0;
        loop {
            // TODO use last
            self.data.next_transition();

            // check if done
            if self.data.is_done() {
                break;
            }

            // if self.is_self_ref {
            //     if !self.data.update_tape_self_ref_speed_up() {
            //         break;
            //     }
            // } else
            // #[cfg(feature = "bb_no_self_ref")]
            if !self.data.update_tape_single_step() {
                break;
            }

            // TODO tape long used
            if self.data.tape.is_tape_long_extended() {
                self.data.status = MachineStatus::Undecided(
                    crate::status::UndecidedReason::TapeSizeLimit,
                    self.data.step_no,
                    TAPE_SIZE_BIT_U128,
                );
                break;
            }

            // peak next to identify apex
            // only take expanding peaks
            let pos = self.data.tape.pos_head();
            // let tr_curr = self.data.tr;
            // let curr = self.data.get_current_symbol();
            let next_tr_field = self.data.tr.state_x2() + self.data.get_current_symbol();
            let next_tr = self.data.transition_table.transition(next_tr_field);
            // self.last_pos_head = pos;
            // self.last_tr = tr_curr;
            if self.data.tr.direction() != next_tr.direction() {
                #[cfg(feature = "bb_debug")]
                println!(
                    "  Dir Change to {}, P: {}",
                    next_tr.direction_to_char(),
                    pos
                );
                let (tape_after_high, tape_after_low) = self.data.tape.tape_short_split();
                let apex = ApexStep {
                    step_no: self.data.step_no as i64,
                    pos_head: self.data.tape.pos_head(),
                    tape_after_low,
                    tape_after_high,
                };
                if next_tr.is_dir_left() {
                    // only take first apex after apex of other side
                    if apex.step_no > last_right_apex_step_no
                        && last_left_apex_step_no <= last_right_apex_step_no
                    {
                        last_left_apex_step_no = apex.step_no;
                        if !self.is_rhythmic_right {
                            self.apex_right.push(apex);
                            if pos > self.max_right {
                                // This is the normal case where the side is expanding.
                                self.max_right = pos;
                                self.max_right_ids.push(self.apex_right.len() - 1);
                                if self.max_right_ids.len() >= 4 {
                                    self.is_rhythmic_right =
                                        is_delta_rhythmic(&self.apex_right, &self.max_right_ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing right for expanding right: {}",
                                        self.is_rhythmic_right
                                    );
                                }
                            } else if pos < self.min_right {
                                self.min_right = pos;
                                self.min_right_ids.push(self.apex_right.len() - 1);
                                if self.min_right_ids.len() >= 4 {
                                    self.is_rhythmic_right =
                                        is_delta_rhythmic(&self.apex_right, &self.min_right_ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing right for wandering left: {}",
                                        self.is_rhythmic_right
                                    );
                                }
                            } else if self.apex_right.len() > 5 && self.max_right_ids.len() < 2 {
                                // In this case, the side is not expanding,
                                let apex_ids = self.get_ids_for_pos_identical_right(pos);
                                // check all pos are identical
                                if let Some(ids) = apex_ids {
                                    self.is_rhythmic_right =
                                        is_delta_rhythmic(&self.apex_right, &ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing right for pos identical: {}",
                                        self.is_rhythmic_right
                                    );
                                }
                            }
                        }
                    }
                } else {
                    if apex.step_no > last_left_apex_step_no
                        && last_right_apex_step_no < last_left_apex_step_no
                    {
                        last_right_apex_step_no = apex.step_no;
                        if !self.is_rhythmic_left {
                            self.apex_left.push(apex);
                            if pos < self.min_left {
                                self.min_left = pos;
                                self.min_left_ids.push(self.apex_left.len() - 1);
                                if self.min_left_ids.len() >= 4 {
                                    self.is_rhythmic_left =
                                        is_delta_rhythmic(&self.apex_left, &self.min_left_ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing left for expanding left: {}",
                                        self.is_rhythmic_left
                                    );
                                }
                            } else if pos > self.max_left {
                                // For cases where the side is wandering of to the other side.
                                self.max_left = pos;
                                self.max_left_ids.push(self.apex_left.len() - 1);
                                if self.max_left_ids.len() >= 4 {
                                    self.is_rhythmic_left =
                                        is_delta_rhythmic(&self.apex_left, &self.max_left_ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing left for wandering right: {}",
                                        self.is_rhythmic_left
                                    );
                                }
                            } else if self.apex_left.len() > 5 && self.min_left_ids.len() < 2 {
                                // In this case, the side is not expanding,
                                let apex_ids = self.get_ids_for_pos_identical_left(pos);
                                // check all pos are identical
                                if let Some(ids) = apex_ids {
                                    self.is_rhythmic_left =
                                        is_delta_rhythmic(&self.apex_left, &ids);
                                    #[cfg(feature = "bb_debug")]
                                    println!(
                                        "  Checked bouncing left for pos identical: {}",
                                        self.is_rhythmic_left
                                    );
                                }
                            }
                        }
                    }
                }
                if self.is_rhythmic_left && self.is_rhythmic_right {
                    #[cfg(feature = "bb_debug")]
                    println!("Found a bouncer!");
                    self.data.status =
                        MachineStatus::DecidedEndless(EndlessReason::Bouncer(self.data.step_no));
                    break;
                }
            }
            #[cfg(feature = "bb_debug")]
            if self.data.step_no % 100 == 0 {
                println!();
            }
        }

        #[cfg(feature = "bb_enable_html_reports")]
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

    fn decide_single_machine(machine: &Machine, config: &Config) -> MachineStatus {
        let mut d = Self::new(config);
        d.decide_machine(machine)
    }

    fn decider_run_batch(batch_data: &mut BatchData) -> ResultUnitEndReason {
        let decider = Self::new(batch_data.config);
        decider::decider_generic_run_batch_v2(decider, batch_data)
    }
}

/// ids must be >= 4
fn is_delta_rhythmic(apexes: &[ApexStep], apex_ids: &[usize]) -> bool {
    // compare step distance: same or same growth
    assert!(apex_ids.len() >= 4);
    if is_delta_rhythmic_sub(apexes, apex_ids, 1) {
        return true;
    };

    // if last > 3 {
    //     let id_0 = apex_ids[last - 4];
    //     let delta_0 = apexes[id_1].step_no - apexes[id_0].step_no;
    //     let delta_diff_0_1 = delta_1 - delta_0;
    //     if delta_diff_2_3 - delta_diff_1_2 == delta_diff_1_2 - delta_diff_0_1 {
    //         println!();
    //         todo!();
    //     }
    // }
    if apex_ids.len() >= 7 {
        // check every second
        // let ids = [
        //     apex_ids[last - 6],
        //     apex_ids[last - 4],
        //     apex_ids[last - 2],
        //     apex_ids[last],
        // ];
        if is_delta_rhythmic_sub(apexes, &apex_ids, 2) {
            return true;
        };
    }

    false
}

fn is_delta_rhythmic_sub(apexes: &[ApexStep], apex_ids: &[usize], id_delta: usize) -> bool {
    let last = apex_ids.len() - 1;
    let ids = [
        apex_ids[last - id_delta * 3],
        apex_ids[last - id_delta * 2],
        apex_ids[last - id_delta],
        apex_ids[last],
    ];
    // compare step distance: same or same growth
    let delta_1 = apexes[ids[1]].step_no - apexes[ids[0]].step_no;
    let delta_2 = apexes[ids[2]].step_no - apexes[ids[1]].step_no;
    let delta_3 = apexes[ids[3]].step_no - apexes[ids[2]].step_no;
    let delta_diff_1_2 = delta_2 - delta_1;
    let delta_diff_2_3 = delta_3 - delta_2;

    // #[cfg(feature = "bb_debug")]
    // if apexes.len() > 10 {
    //     //  || apexes.last().unwrap().pos_head != 0 {
    //     println!()
    // }

    let mut compare = delta_diff_1_2 == delta_diff_2_3;
    // check if the delta is a multiple of the other
    if !compare
        && delta_diff_1_2 != 0
        && last >= id_delta * 4
        && delta_diff_2_3 % delta_diff_1_2 == 0
    {
        let id_0 = apex_ids[last - id_delta * 4];
        let delta_0 = apexes[ids[0]].step_no - apexes[id_0].step_no;
        let delta_diff_0_1 = delta_1 - delta_0;
        if delta_diff_0_1 != 0 && delta_diff_1_2 % delta_diff_0_1 == 0 {
            compare = delta_diff_2_3 / delta_diff_1_2 == delta_diff_1_2 / delta_diff_0_1;
            #[cfg(feature = "bb_debug")]
            if compare {
                println!(
                    "  Multiple found: {delta_diff_2_3}, {delta_diff_1_2} ({delta_diff_0_1}): {} == {}",
                    delta_diff_2_3 / delta_diff_1_2,
                    delta_diff_1_2 / delta_diff_0_1
                );
            }
        }
    }

    if compare {
        // compare bit changes
        let tape_list = [
            apexes[ids[0]].tape_after_low,
            apexes[ids[1]].tape_after_low,
            apexes[ids[2]].tape_after_low,
            apexes[ids[3]].tape_after_low,
        ];
        let changes_low = Changes::new(tape_list);
        if changes_low.is_change_identical_4_elements() {
            let tape_list = [
                apexes[ids[0]].tape_after_high,
                apexes[ids[1]].tape_after_high,
                apexes[ids[2]].tape_after_high,
                apexes[ids[3]].tape_after_high,
            ];
            let changes_high = Changes::new(tape_list);
            if changes_high.is_change_identical_4_elements() {
                return true;
            }
        }
    }

    false
}

/// Function to test single machine
pub fn test_decider(transition_tm_format: &str) {
    // let config = Config::new_default(5);
    let machine = Machine::from_standard_tm_text_format(0, transition_tm_format).unwrap();
    let config = Config::builder(machine.n_states())
        .write_html_file(true)
        .write_html_step_start(792_199_000)
        .write_html_line_limit(500_000)
        .step_limit_bouncer(800_000_000)
        .build();
    let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
    println!("{}", check_result);
    // assert_eq!(check_result, MachineStatus::DecidedHolds(47176870));
}

// #[derive(Debug, Default)]
// struct ApexData {
//     steps: Vec<ApexStep>,
//     max: i64,
//     max_ids: Vec<usize>,
//     min: i64,
//     min_ids: Vec<usize>,
//     is_rhythmic: bool,
// }

/// This struct only stores the tape if either the left or right side of the tape is 0.
/// Every even entry is left side empty, odd right side empty.
/// Since only consecutive entries are checked, the step_no is not relevant.
// TODO step_no could be interesting to check if a rhythm is there (e.g. prev. distance + 2)
#[derive(Debug)]
struct ApexStep {
    /// only for debugging purposes
    // #[cfg(debug_assertions)]
    step_no: i64,
    pos_head: i64,
    /// tape after transition was executed
    tape_after_low: u64,
    tape_after_high: u64,
}

#[derive(Debug)]
struct Changes {
    // tapes: [u64; 4],
    changed: [Changed; 3],
}

impl Changes {
    fn new(tape_list: [u64; 4]) -> Self {
        // let mut tapes = [0u64; 4];
        // tapes.copy_from_slice(&tape_list[0..4]);
        let changed = [
            Changed::new(tape_list[1], tape_list[0]),
            Changed::new(tape_list[2], tape_list[1]),
            Changed::new(tape_list[3], tape_list[2]),
        ];
        Self {
            // tapes: tape_list,
            changed,
        }
    }

    /// Same change on three comparisons (4 elements)
    fn is_change_identical_4_elements(&self) -> bool {
        let is_changed_pos_delta_ok =
            self.changed[1].pos - self.changed[0].pos == self.changed[2].pos - self.changed[1].pos;
        if is_changed_pos_delta_ok {
            let is_change_added_bits_ok = self.changed[0].change_moved
                == self.changed[1].change_moved
                && self.changed[1].change_moved == self.changed[2].change_moved;
            if !is_change_added_bits_ok {
                // check grow left and right
                #[cfg(feature = "bb_debug")]
                for c in &self.changed {
                    println!("{c}");
                }
                // remove identical right bits
                let mut changed_left = self.changed;
                while changed_left[0].change_moved & 1 == changed_left[1].change_moved & 1
                    && changed_left[1].change_moved & 1 == changed_left[2].change_moved & 1
                {
                    for c in changed_left.iter_mut() {
                        c.change_moved >>= 1;
                        c.pos = c.change_moved.leading_zeros() as i32;
                    }
                }
                #[cfg(feature = "bb_debug")]
                for c in &changed_left {
                    println!("{c}");
                }
                // delta of c.pos must be identical as parent is identical, or not?
                let d = changed_left[0].pos - changed_left[1].pos;
                if d == 1 {
                    // same inserted?
                    if changed_left[1].change_moved & 1 == changed_left[2].change_moved & 1
                        && changed_left[1].change_moved & 1
                            == (changed_left[2].change_moved >> 1) & 1
                    {
                        changed_left[1].change_moved >>= 1;
                        changed_left[2].change_moved >>= 2;

                        // identical change
                        if changed_left[0].change_moved == changed_left[1].change_moved
                            && changed_left[1].change_moved == changed_left[2].change_moved
                        {
                            return true;
                        }
                    }
                }
                // TODO check if other deltas need to be evaluated, not too difficult
                //  else {
                //     todo!()
                // }
            }
            return is_change_added_bits_ok;
        }

        false
    }
}

/// stores the changed bits between two consecutive relevant steps
#[derive(Debug, Clone, Copy)]
struct Changed {
    /// trailing zeros in original, shows number of added bits when rows are compared
    pos: i32,
    /// changed bits, moved to the right, ideally the changes are then identical
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
        #[cfg(feature = "bb_debug")]
        {
            use crate::tape::tape_utils::U64Ext;

            println!(
                " OLD {}, CHG {}",
                older_tape.to_binary_split_string(),
                changed.to_binary_split_string()
            );
            println!(
                " NEW {}, MVD {}",
                newer_tape.to_binary_split_string(),
                (changed >> trailing_zeros).to_binary_split_string()
            );
            // println!(" CHG {}", changed.to_binary_split_string());
            // println!(
            //     " MVD {}",
            //     (changed >> trailing_zeros).to_binary_split_string()
            // );
        }
        Self {
            pos: trailing_zeros as i32,
            change_moved: changed >> trailing_zeros,
        }
    }

    // Same change on three comparisons (4 elements)
    //     fn is_change_identical_4_elements(changed: &[Self]) -> bool {
    //         assert_eq!(3, changed.len());
    //         let is_changed_pos_delta_ok =
    //             changed[1].pos - changed[0].pos == changed[2].pos - changed[1].pos;
    //         if is_changed_pos_delta_ok {
    //             let is_change_added_bits_ok = changed[0].change_moved == changed[1].change_moved
    //                 && changed[1].change_moved == changed[2].change_moved;
    //             if !is_change_added_bits_ok {
    //                 // check grow left and right
    //                 let pos_delta = (changed[1].pos - changed[0].pos).abs();
    //                 // check even positive number
    //                 if pos_delta & 1 == 0 {
    //                     let half = pos_delta / 2;
    //                     for c in changed {
    //                         println!("{c}");
    //                     }
    //                     let changed_1 = changed[1].change_moved << half;
    //                     let changed_2 = changed[2].change_moved << (half * 2);
    //                     let right_ok = changed[0].change_moved == changed_1 && changed_1 == changed_2;
    //                     if right_ok {
    //                         println!()
    //                     }
    //
    //                     // if pos_delta == 2 {
    //                     // } else {
    //                     //     todo!("need work on other possibilities")
    //                     // }
    //                 }
    //             }
    //             return is_change_added_bits_ok;
    //         }
    //
    //         false
    //     }

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

    fn is_bouncer(machine: &Machine) -> bool {
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_bouncer(5000)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        if check_result.is_bouncer() {
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
    fn is_bouncer_bb4_2793430() {
        // BB4 every 2nd step
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "0LD"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("---", "1RA"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(2793430, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb4_11337065() {
        let machine =
            Machine::from_standard_tm_text_format(11337065, "1RB0LB_1LA0LC_---1RD_0RA0RA").unwrap();
        assert!(is_bouncer(&machine));
    }

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
    fn is_bouncer_bb4_19125173() {
        let machine =
            Machine::from_standard_tm_text_format(19125173, "1RB1RA_1LC---_1RD1LC_0RA0RA").unwrap();
        assert!(is_bouncer(&machine));
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
    fn is_bouncer_bb4_39509465() {
        let machine =
            Machine::from_standard_tm_text_format(39509465, "1RB---_1LC0RA_0LD0LB_1RA0RA").unwrap();
        assert!(is_bouncer(&machine));
    }

    // Step   7: 000000000000000000000000_00000010←00000000
    // Step  19: 000000000000000000000000_00010010←00000000
    // Step  37: 000000000000000000000000_10010010←00000000
    // Step  61: 000000000000000000000100_10010010←00000000
    #[test]
    fn is_bouncer_bb4_47640088() {
        let machine =
            Machine::from_standard_tm_text_format(47640088, "0RB0LB_0LC0RD_1LA---_1RA0RA").unwrap();
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_1RB1LC_0RCZZZ_1LD1RC_0RC0RA() {
        let machine =
            Machine::from_standard_tm_text_format(0, "1RB1LC_0RC---_1LD1RC_0RC0RA").unwrap();
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_84080() {
        // BB3 84080 (high bound check)
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LA", "0RA"));

        let machine = Machine::from_string_tuple(84080, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_112641() {
        // BB3 112641
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "0LB"));
        transitions.push(("1LA", "---"));
        transitions.push(("1LA", "0RA"));

        let machine = Machine::from_string_tuple(112641, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_569564() {
        // BB3 569564
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "0LA"));
        transitions.push(("1LA", "---"));
        transitions.push(("0LB", "1RA"));
        let machine = Machine::from_string_tuple(569564, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_584567() {
        // BB3 584567 collects too many for one side, matches only after other side apex
        let machine =
            Machine::from_standard_tm_text_format(584567, "1RC---_0RA0LB_1LB1RA").unwrap();
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_1265977() {
        // BB3 1265977 collects too many for one side, matches only after other side apex
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "---"));
        transitions.push(("0LA", "0RB"));
        transitions.push(("1RB", "1LA"));
        let machine = Machine::from_string_tuple(1265977, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_1970063() {
        // BB3 1970063 step_delta iterates same delta +-
        let machine =
            Machine::from_standard_tm_text_format(1970063, "0RB0LA_1RC---_1LA1RB").unwrap();
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_3044529() {
        // BB3 3044529 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LB", "---"));
        transitions.push(("1RC", "1LB"));
        transitions.push(("0LA", "0RC"));
        let machine = Machine::from_string_tuple(3044529, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_3554911() {
        // BB3 3554911 A0 always same low_bound and pos = MIDDLE_BIT
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RB", "---"));
        transitions.push(("1LC", "1RB"));
        transitions.push(("0RA", "0LC"));
        let machine = Machine::from_string_tuple(3554911, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_6317243() {
        // BB4 Start out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("1RD", "0LC"));
        transitions.push(("1LB", "0RB"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(6317243, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_13318557() {
        // BB4 Start High bound out of sync
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1RC", "---"));
        transitions.push(("0LD", "1LB"));
        transitions.push(("0LB", "1RC"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(13318557, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_15783962() {
        // BB4 ascending shift with gap and linear growing distance between head pos
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LB", "1RD"));
        transitions.push(("1LC", "---"));
        transitions.push(("1RA", "1LC"));
        transitions.push(("0RA", "0RA"));
        let machine = Machine::from_string_tuple(15783962, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_32538705() {
        // BB4 sinus, but not with A0
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0RC", "1LC"));
        transitions.push(("---", "1RC"));
        transitions.push(("1LD", "1RB"));
        transitions.push(("1RA", "0RA"));
        let machine = Machine::from_string_tuple(32538705, &transitions);
        assert!(is_bouncer(&machine));
    }

    #[test]
    fn is_bouncer_bb3_41399() {
        // BB3 41399 (this is a cycler, but it actually expands endless with 0)
        let machine = Machine::from_standard_tm_text_format(41399, "1LB---_0RC1RB_1RA0RA").unwrap();
        assert!(is_bouncer(&machine));
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
    fn to_check_is_not_bouncer_bb4_45935166() {
        // BB4 delta of delta rhythm 22, 14, 20 repeats; requires 128-bit tape
        // TODO here the comparison would be to find an apex in the apex-list first,
        // check all pos, find the apexes, here for right
        // Step   4:  2, then down and up: 000000000000000000000000_00000101←00000000
        // Step  28:  4, then down and up: 000000000000000000000000_01110101←00000000
        // Step  74:  6, then down and up: 000000000000000000001101_01110101←00000000
        // Step 134:  8, then down and up: 000000000000000001011101_01110101←00000000
        // Step 214: 10, then down and up: 000000000000011101011101_01110101←00000000
        // Step 316: 12, then down and up: 000000001101011101011101_01110101←00000000
        // additionally only every third needs to be compared

        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("0LC", "1LA"));
        transitions.push(("0RD", "---"));
        transitions.push(("1RB", "1LD"));
        transitions.push(("1RA", "0RA"));
        let machine = Machine::from_string_tuple(45935166, &transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_bouncer(2000)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        assert_eq!(
            check_result,
            MachineStatus::Undecided(UndecidedReason::StepLimit, 2000, 128)
        );
    }

    // TODO interesting machine, endless, but need other check
    #[test]
    fn to_check_is_not_bouncer_bb4_64379691() {
        // BB4 every steps repeating, but with growing amount of identical steps
        let mut transitions: Vec<(&str, &str)> = Vec::new();
        transitions.push(("1LC", "1RA"));
        transitions.push(("---", "1RD"));
        transitions.push(("1RB", "1LC"));
        transitions.push(("0LA", "0RA"));
        let machine = Machine::from_string_tuple(64379691, &transitions);
        // let config = Config::new_default(machine.n_states());
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_bouncer(5000)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
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
        let machine = Machine::from_string_tuple(68106631, &transitions);
        // // let config = Config::new_default(machine.n_states());
        // let config = Config::builder(machine.n_states())
        // .write_html_file(true)
        // .step_limit_bouncer(5000)
        // .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        if let MachineStatus::Undecided(UndecidedReason::TapeSizeLimit, _, _) = check_result {
        } else {
            panic!("{check_result}");
        }
    }

    #[test]
    fn is_not_decider_bouncer_bb3_max_651320() {
        // BB3 Max
        let machine = Machine::build_machine("BB3_MAX").unwrap();
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        assert_eq!(check_result, MachineStatus::DecidedHolds(21));
    }

    #[test]
    fn is_not_bouncer_bb4_max_322636617() {
        // BB4 Max
        let machine = Machine::build_machine("BB4_MAX").unwrap();
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        assert_eq!(check_result, MachineStatus::DecidedHolds(107));
    }

    #[test]
    fn is_not_decider_bouncer_bb5_max() {
        // BB5 Max
        let machine = Machine::build_machine("BB5_MAX").unwrap();
        let config = Config::builder(machine.n_states())
            .write_html_file(true)
            .step_limit_bouncer(10000)
            .build();
        let check_result = DeciderBouncerApex::decide_single_machine(&machine, &config);
        // println!("Result: {}", check_result);
        let ok = if let MachineStatus::Undecided(_, _, _) = check_result {
            true
        } else {
            println!("Result: {}", check_result);
            false
        };
        assert!(ok);
    }
}
