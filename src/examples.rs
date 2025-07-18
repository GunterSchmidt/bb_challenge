use crate::{config::Config, decider::Decider, decider_cycler::DeciderCycler, machine::Machine};

/// This example runs the cycler on the bb_challenge file machine id 30605 and creates a HTML file showing all steps. \
/// It requires feature "bb_enable_html_reports" to be enabled.
pub fn bb_challenge_id_30605_cycler_to_html() {
    let config_single = Config::builder(4)
        .write_html_file(true)
        .write_html_line_limit(25_000)
        .step_limit_cycler(50_000)
        .build();
    let id = 30605;
    let machine =
        Machine::from_standard_tm_text_format(id, "1RB0RZ_0RC0RA_1RD0LE_0LC1RC_1LC0RA").unwrap();
    // let ms = bb_challenge::decider_bouncer_v2::DeciderBouncerV2::decide_single_machine(
    //     &machine,
    //     &config_single,
    // );
    let ms = DeciderCycler::decide_single_machine(&machine, &config_single);
    println!("Machine {id}: {}", ms);
}
