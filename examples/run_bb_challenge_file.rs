//! This example shows how to evaluate the bb_challenge file. \
//! It creates the HTML file for the detailed description of [TapeLongShifted].
//!
//! Run this with 'cargo run --example run_bb_challenge_file -- release'. Should run around 2 seconds, without --release a minute.
//!
//! For this to work, the bb_challenge file must be available. Check the config.toml file in bb_challenge folder.
//! Set the 'bb_challenge_filename_path' with the correct full or relative path.

use bb_challenge::{
    config::{Config, CoreUsage},
    data_provider::bb_file_reader,
    decider::{DeciderConfig, DeciderStandard},
    html,
};

// This can be changed to evaluate a different section.
const FIRST_MACHINE_ID: u64 = 10_000_000;
const NUM_MACHINES: u64 = 5_000;

fn main() {
    evaluate_bb_challenge_file();
}

fn evaluate_bb_challenge_file() {
    let n_states = 5;
    let config = Config::builder(n_states)
        .file_id_range(FIRST_MACHINE_ID..FIRST_MACHINE_ID + NUM_MACHINES)
        // Turn this on, if you want all machines output as HTML files.
        // .write_html_file(true)
        // Turn this on, if you only want the undecided machines as HTML files, run with different deciders.
        .limit_machines_undecided(200)
        .build();
    let (config_1, config_2) = DeciderConfig::standard_config(&config);
    // println!("Config 1: {config_1}");
    // println!("Config 2: {config_2}");
    let decider_configs = DeciderStandard::standard_decider_for_config(&config_1, &config_2);

    // do not use hold decider (just for runtime purposes in this example)
    let decider_last = 3;
    assert_eq!(5, config_1.n_states());
    // run bb_challenge file
    let result = bb_file_reader::run_deciders_bb_challenge_file(
        &decider_configs[0..decider_last],
        CoreUsage::SingleCoreEnumeratorMultiCoreDecider,
    );

    let mut names = Vec::new();
    for d in decider_configs[0..decider_last].iter() {
        names.push(d.decider_id().name);
    }
    println!();
    println!("Decider: {}", names.join(", "));
    println!("Config 1: {config_1}");
    if decider_last > 2 {
        println!("Config 2: {config_2}");
    }
    println!("\n{}", result.to_string_with_duration());

    // write undecided to html
    if let Some(m_undecided) = result.machines_undecided() {
        let config = Config::builder_from_config(&config_1)
            .step_limit_decider_cycler(100_000)
            .step_limit_decider_bouncer(100_000)
            .step_limit_decider_halt(100_000)
            .write_html_line_limit(25_000)
            .build();
        html::write_machines_to_html(&m_undecided, "undecided", &config, 1000, false);
    }
}
