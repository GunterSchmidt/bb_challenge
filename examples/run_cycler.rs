//! This example shows how to used the Cycler for a machine. \
//! It creates the HTML file for the detailed description of [TapeLongShifted].
/// It requires the feature "enable_html_reports" to be enabled. \
use bb_challenge::{
    config::Config,
    decider::{decider_cycler::DeciderCycler, decider_halt_long::DeciderHaltLong, Decider},
    machine_binary::MachineId,
};

fn main() {
    bb_challenge_cycler_undecided_to_html();
    bb_challenge_cycler_loop_4_3();
}

/// This runs the cycler on the bb_challenge file and creates a HTML file showing all steps. \
/// 1RB---_0RC0RA_1RD0LE_0LC1RC_1LC0RA
pub fn bb_challenge_cycler_undecided_to_html() {
    let machine = MachineId::try_from("1RB---_0RC0RA_1RD0LE_0LC1RC_1LC0RA").unwrap();
    let config = Config::builder(machine.n_states())
        .write_html_file(true)
        .write_html_line_limit(25_000)
        .step_limit_decider_cycler(50_000)
        .build();
    let status = DeciderCycler::decide_single_machine(&machine, &config);
    let id = machine.id_or_normalized_id();
    println!("Machine {id}: {}", status);

    // let status_bouncer = bb_challenge::decider::decider_bouncer_128::DeciderBouncer128::decide_single_machine(&machine, &config);
    // println!("Machine {id}: {}", status_bouncer);
}

pub fn bb_challenge_cycler_loop_4_3() {
    let machine_left = MachineId::try_from("1RB---_0RC0LE_1LD0LA_1LB1RB_1LC1RC").unwrap();
    let machine_right = MachineId::try_from("1RB---_1LB1LC_0RD0RC_1LE1RE_1LA0LE").unwrap();
    let config = Config::builder(machine_left.n_states())
        .write_html_file(true)
        .write_html_line_limit(25_000)
        .step_limit_decider_cycler(50_000)
        .build();
    let status_left = DeciderCycler::decide_single_machine(&machine_left, &config);
    let status_right = DeciderCycler::decide_single_machine(&machine_right, &config);
    let status_right_hold = DeciderHaltLong::decide_single_machine(&machine_right, &config);
    println!("Machine left: {}", status_left);
    println!("Machine right: {}", status_right);
    println!("Machine right hold: {}", status_right_hold);

    // let status_bouncer = bb_challenge::decider::decider_bouncer_128::DeciderBouncer128::decide_single_machine(&machine, &config);
    // println!("Machine {id}: {}", status_bouncer);
}
