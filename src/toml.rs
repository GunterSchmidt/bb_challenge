//! Very basic functionality to read and write some configuration into a toml configuration file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigToml {
    /// Method to calculate the id.
    #[serde(default = "default_id_calc_forward")]
    id_calc_forward: bool,

    /// file path and file name of bb_challenge file, usually named "all_5_states_undecided_machines_with_global_header"
    #[serde(default = "default_bb_challenge_file")]
    bb_challenge_filename_path: String,

    /// file path and file name of bb_challenge file, usually named "all_5_states_undecided_machines_with_global_header"
    #[serde(default = "default_html_out_path")]
    html_out_path: String,

    /// shifted true: head is always in the middle.
    #[serde(default = "default_html_tape_shifts")]
    html_tape_shifts: bool,
}

impl ConfigToml {
    pub fn read_toml() -> ConfigToml {
        if Path::new(CONFIG_FILE).exists() {
            let config_content = fs::read_to_string(CONFIG_FILE)
                .expect("Config file {CONFIG_FILE} could not be read.");
            let config: ConfigToml = toml::from_str(&config_content)
                .expect("Config file {CONFIG_FILE} could not be parsed.");
            config
        } else {
            println!(
                "Config file {CONFIG_FILE} not found, creating a new one with default values."
            );
            let default_config = ConfigToml::default();
            let toml_string = toml::to_string_pretty(&default_config)
                .expect("Failed to serialize default config");
            let write_result = fs::write(CONFIG_FILE, toml_string);
            if write_result.is_err() {
                println!(
                    "ERROR: Config file {CONFIG_FILE} was not found and could not be written. Using default values, some functionality might not be available."
                );
            }
            default_config
        }
    }

    pub fn id_calc_forward(&self) -> bool {
        self.id_calc_forward
    }

    pub fn bb_challenge_filename_path(&self) -> &str {
        &self.bb_challenge_filename_path
    }

    pub fn html_out_path(&self) -> &str {
        &self.html_out_path
    }

    pub fn html_tape_shifts(&self) -> bool {
        self.html_tape_shifts
    }
}

impl Default for ConfigToml {
    fn default() -> Self {
        ConfigToml {
            id_calc_forward: default_id_calc_forward(),
            bb_challenge_filename_path: default_bb_challenge_file(),
            html_out_path: default_html_out_path(),
            html_tape_shifts: default_html_tape_shifts(),
        }
    }
}

fn default_id_calc_forward() -> bool {
    true
}

fn default_bb_challenge_file() -> String {
    "../res/all_5_states_undecided_machines_with_global_header".to_string()
}

fn default_html_out_path() -> String {
    "../bb_result_html".to_string()
}

fn default_html_tape_shifts() -> bool {
    true
}

pub fn test_toml() {
    let config: ConfigToml = ConfigToml::read_toml();

    println!("id_calc_forward: {}", config.id_calc_forward);
    println!("path: {}", config.bb_challenge_filename_path);
}
