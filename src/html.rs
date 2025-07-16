//! Functionality to output the machine steps in HTML format. \
//! The output is limited to the 128-Bit tape_shifted, so not the full tape is seen, but this is usually enough to analyze. \
//! The step line looks like this (here only 64 bits are show): \
//! Step     1 A0 1RC: 000000000000000000000000_0000000**1**\*00000000_000000000000000000000000 \
//! where the Head is always directly after the '\*'.
//! So here the first step is from table field A0 having transition 1RC, so it writes a 1 on the tape and shifts the head right.
//! This translates into a shift left for the tape if the head is held in a fixed position.
//! The \[1\] is just a bold 1 in html indicating the changed cell. The head now rests at the first 0 after the '*'.
//! The output is limited to WRITE_HTML_STEP_LIMIT (100_000) steps, but can be set higher in config. The full output of BB5_MAX takes
//! a while and creates a 10 GB large html file (each step). \
//! In case of the self-ref speed-up not all steps are shown, as the repetitions are omitted. Makes the file much smaller. \
//! Since the step number is used for the config.write_html_step_limit, the file can be very small. \
//! Instead use config.write_html_line_limit which counts the actually written steps. For BB5_MAX only 91,021 of 47,176,870 = 0.02 %
//! are written, which makes it possible to write the full output to a 19,1 MB large html file.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, MAIN_SEPARATOR_STR};

use crate::config::{Config, StepTypeBig};
use crate::decider_data_128::DeciderData128;
use crate::machine::Machine;
use crate::status::MachineStatus;
use crate::tape_utils::TapeLongPositions;
use crate::transition_symbol2::TransitionSymbol2;

const CSS_FOLDER: &str = "styles";
const CSS_FILE_LIGHT: &str = "light.css";
const CSS_FILE_DARK: &str = "dark.css";
const BODY_FONT_FAMILY: &str = "monospace";
pub const CLASS_HEAD_POSITION: &str = "head_pos";
pub const CLASS_CHANGED_POSITION: &str = "change_pos";
// const CLASS_TABLE: &str = "table_transition";
const CSS_HEAD_POSITION_LIGHT: &str = "background-color: lavenderblush; font-weight: bold;"; // color: black;
const CSS_HEAD_POSITION_DARK: &str = "background-color: lavenderblush;"; // color: black;
const CSS_CHANGE_POSITION_LIGHT: &str = "font-weight: bold;"; // color: blue;
const CSS_CHANGE_POSITION_DARK: &str = "font-weight: bold;"; // color: yellow;
const CSS_STEP: &str = ".p_step {
    padding: 0;
    margin: 0;
}";
const CSS_HTML: &str = "html {
    background-color: white;
    height: 100%;
    margin: 0px;
    padding: 0px;
}";
const CSS_TABLE_LIGHT: &str = "table,
th,
td {
    border: 1px solid black;
    border-collapse: collapse;
    padding: 3px;
    margin-left: 10px;
}";
const CSS_TABLE_DARK: &str = "table,
th,
td {
    border: 1px solid white;
    border-collapse: collapse;
    padding: 3px;
    margin-left: 10px;
}";

/// Returns a String with the number of blanks specified, which does not compress in html ("\&nbsp;\&nbsp;").
pub fn blanks(num_blanks: usize) -> String {
    "&nbsp;".repeat(num_blanks)
}

/// Creates the css files if they do not exist.
pub fn create_css(path: &str) -> io::Result<()> {
    // Define file names
    let css_path = Path::new(path).join(CSS_FOLDER);
    if !css_path.exists() {
        std::fs::create_dir_all(&css_path)?;
    }
    let light_css = css_path.join("light.css");
    let dark_css = css_path.join("dark.css");

    // Create and write to light mode CSS file
    if !light_css.exists() {
        let mut light_css_file = File::create(&light_css)?;
        write_light_css_content(&mut light_css_file)?;
    }

    // Create and write to dark mode CSS file
    if !dark_css.exists() {
        let mut dark_css_file = File::create(&dark_css)?;
        write_dark_css_content(&mut dark_css_file)?;
    }

    Ok(())
}

/// Writes to html header and the start of the body to the file.
/// # Arguments
/// - path of the html file
/// - file_name_prefix
/// - machine to write, uses tm_standard_name for file name
///
/// Example: ("data", "hold", m) creates /data/hold_1RB0RC_1RA0RA_0RB1RC.html
///
/// # Returns
/// (File, FileName)
pub fn create_html_file_start(
    path: &str,
    decider_name: &str,
    machine: &Machine,
) -> io::Result<(File, String)> {
    if !std::fs::exists(path).expect("File System Error") {
        std::fs::create_dir_all(path)?;
    }
    let file_name = decider_name.to_owned() + "_" + machine.file_name().as_str() + ".html";
    let p = Path::new(path).join(&file_name);
    let mut file = File::create(&p)?;
    write_html_header(&mut file, &machine.to_standard_tm_text_format())?;
    writeln!(file, "<body>")?;
    let m_id = if machine.id() == 0 {
        String::new()
    } else {
        format!(" Id: {}", machine.id())
    };
    // write header, machine, e.g.
    // BB4 Decider Cycler Machine Id: 32538705 0RC1LC_---1RC_1LD1RB_1RA0RA
    writeln!(
        file,
        "  <h2>BB{} {decider_name} Machine{m_id} {}</h2>",
        machine.n_states(),
        machine.to_standard_tm_text_format()
    )?;
    // Machine transitions as table
    writeln!(
        file,
        "{}",
        machine.transition_table().to_table_html_string(true)
    )?;
    // start with an opening <p> tag, as the lines end with </br>, but this actually leads to nested paragraphs.
    // writeln!(file, "  <p>")?;

    Ok((file, file_name))
}

/// Formats an Integer right aligned
pub fn format_right_aligned_int_html(number: usize, size: usize) -> String {
    let s = format!("{number:>size$}");
    s.replace(" ", "&nbsp;")
}

/// Creates the folder path of the html file and the css files in the folder if not already existing.
/// # Returns
/// - the path for the html files, usually '/result/<sub_path>_bb5', e.g. '/result/cycler_bb5' \
/// - None if write_html_file in [Config] is set to false.
pub fn get_html_path(sub_path: &str, config: &Config) -> Option<String> {
    if config.write_html_file() {
        let path = format!(
            "{}{MAIN_SEPARATOR_STR}{sub_path}_bb{}",
            Config::get_result_path(),
            config.n_states()
        );
        let msg = format!("CSS files could not be created in {path}.");
        create_css(&path).expect(&msg);
        Some(path)
    } else {
        None
    }
}

/// Rename file depending on status, Decided or Undecided will be added to the file name.
/// Panics if file cannot be renamed
pub fn rename_file_to_status(file_path: &str, file_name: &str, machine_status: &MachineStatus) {
    // -> io::Result<()>
    let old_path = format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, file_name);
    // let mut new_path: Option<String> = None;
    let new_path = match machine_status {
        MachineStatus::NoDecision => todo!(),
        MachineStatus::EliminatedPreDecider(_) => todo!(),
        MachineStatus::Undecided(_, _, _) => {
            // rename file
            let f_name_new = "undecided_".to_string() + file_name;
            Some(format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, f_name_new))
        }
        _ => {
            // rename file
            let f_name_new = "decided_".to_string() + file_name;
            Some(format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, f_name_new))
        }
    };
    if let Some(new_path) = new_path {
        let r = std::fs::exists(&old_path);
        match r {
            Ok(exists) => {
                if !exists {
                    panic!("File {old_path} not found!");
                }
            }
            Err(e) => panic!("File Error: {e}"),
        }
        std::fs::rename(&old_path, &new_path)
            .unwrap_or_else(|_| panic!("Could not rename file: {old_path}"));
    }
}

/// Writes the \<head\> section of the file.
pub fn write_html_header(file: &mut File, title: &str) -> io::Result<()> {
    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "    <meta charset=\"UTF-8\">")?;
    writeln!(
        file,
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    )?;
    writeln!(file, "    <title>{title}</title>")?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{CSS_FOLDER}/{CSS_FILE_LIGHT}\" media=\"(prefers-color-scheme: light)\">",
    )?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{CSS_FOLDER}/{CSS_FILE_DARK}\" media=\"(prefers-color-scheme: dark)\">",
    )?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{CSS_FOLDER}/{CSS_FILE_LIGHT}\" media=\"not all and (prefers-color-scheme)\">",
    )?; // Fallback for browsers not supporting prefers-color-scheme
    writeln!(file, "    <style>")?;
    writeln!(
        file,
        "        body {{ font-family: {BODY_FONT_FAMILY}; font-size: larger;}}"
    )?;
    writeln!(file, "    </style>")?;
    writeln!(file, "</head>")?;
    Ok(())
}

fn write_light_css_content(file: &mut File) -> io::Result<()> {
    writeln!(file, "{CSS_HTML}")?;
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: white;")?;
    writeln!(file, "    min-height: 100vh;")?;
    writeln!(file, "    margin: 0;")?;
    writeln!(file, "    padding: 10px;")?;
    writeln!(file, "    color: black;")?;
    writeln!(file, "}}")?;
    writeln!(file, "{CSS_TABLE_LIGHT}")?;
    writeln!(file, "{CSS_STEP}")?;
    writeln!(file)?;
    writeln!(
        file,
        ".{CLASS_HEAD_POSITION} {{\n    {CSS_HEAD_POSITION_LIGHT}\n}}"
    )?;
    writeln!(
        file,
        ".{CLASS_CHANGED_POSITION} {{\n    {CSS_CHANGE_POSITION_LIGHT}\n}}"
    )?;
    Ok(())
}

fn write_dark_css_content(file: &mut File) -> io::Result<()> {
    writeln!(file, "{CSS_HTML}")?;
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: black;")?;
    writeln!(file, "    min-height: 100vh;")?;
    writeln!(file, "    margin: 0;")?;
    writeln!(file, "    padding: 10px;")?;
    writeln!(file, "    color: white;")?;
    writeln!(file, "}}")?;
    writeln!(file, "{CSS_TABLE_DARK}")?;
    writeln!(file, "{CSS_STEP}")?;
    writeln!(file)?;
    writeln!(
        file,
        ".{CLASS_HEAD_POSITION} {{\n    {CSS_HEAD_POSITION_DARK    }\n}}"
    )?;
    writeln!(
        file,
        ".{CLASS_CHANGED_POSITION} {{\n    {CSS_CHANGE_POSITION_DARK}\n}}"
    )?;
    Ok(())
}

pub fn write_file_end(file: &mut File) -> io::Result<()> {
    // writeln!(file, "  </p>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

/// Deprecated
pub fn write_step_html_128(
    file: &mut File,
    step_no: StepTypeBig,
    tr_field_id: usize,
    transition: TransitionSymbol2,
    tape_shifted: u128,
    pos_middle: usize,
) {
    let data = StepHtml {
        step_no,
        tr_field_id,
        transition,
        tape_shifted,
        is_u128_tape: true,
        pos_middle,
        tape_long_positions: None,
    };
    data.write_step_html(file);
}

/// Writes a text into an open Html file.
/// # Panics
/// Panics on file write error. At this point it is unlikely to occur.
pub fn write_html(file: &mut File, text: &str) {
    writeln!(file, "{text}",).expect("Html write error");
}

/// Writes a paragraphed text into an open Html file.
/// # Panics
/// Panics on file write error. At this point it is unlikely to occur.
pub fn write_html_p(file: &mut File, text: &str) {
    writeln!(file, "<p>{text}</p>",).expect("Html write error");
}

/// All data required to write a step to the html file and write functionality.
pub struct StepHtml {
    /// Current step no, starting at 1.
    pub step_no: StepTypeBig,
    /// Table field which lead to the current transition.
    pub tr_field_id: usize,
    /// Current transition
    pub transition: TransitionSymbol2,
    /// tape after the transition was executed
    pub tape_shifted: u128,
    /// if false the lower 64 bit will be used. This can also be used to only print the middle part if the tape is shifted before by 32 bit.
    pub is_u128_tape: bool,
    /// current pos_middle
    pub pos_middle: usize,
    /// current tape_long if available or necessary
    pub tape_long_positions: Option<TapeLongPositions>,
}

impl StepHtml {
    /// Write a single step to the html file.
    /// # Panics
    /// If file cannot be written. Unlikely as the file is already open for write.
    pub fn write_step_html(&self, file: &mut File) {
        write_html(file, &self.step_to_html());
    }

    /// Formats the line
    fn step_to_html(&self) -> String {
        let binary = if self.is_u128_tape {
            crate::tape_utils::U128Ext::to_binary_split_html_string(
                &self.tape_shifted,
                &self.transition,
            )
        } else {
            crate::tape_utils::U64Ext::to_binary_split_html_string(
                &(self.tape_shifted as u64),
                &self.transition,
            )
        };
        let tl_pos = if let Some(tp) = &self.tape_long_positions {
            format!(
                " TL P{} {}..{}",
                tp.tl_pos, tp.tl_low_bound, tp.tl_high_bound
            )
        } else {
            String::new()
        };
        format!(
            "<p class=\"p_step\">Step {} {} {}: {binary} P: {}{}</p>",
            Self::format_right_aligned_int_html(self.step_no, 5),
            TransitionSymbol2::field_id_to_string(self.tr_field_id),
            self.transition,
            self.pos_middle,
            tl_pos
        )
    }

    /// Formats an Integer right aligned
    pub fn format_right_aligned_int_html(number: StepTypeBig, size: usize) -> String {
        let s = format!("{number:>size$}");
        s.replace(" ", "&nbsp;")
    }
}

impl From<&DeciderData128> for StepHtml {
    fn from(data: &DeciderData128) -> Self {
        Self {
            step_no: data.step_no,
            tr_field_id: data.tr_field,
            transition: data.tr,
            tape_shifted: data.tape_shifted(),
            is_u128_tape: true,
            pos_middle: data.tl.pos_middle(),
            tape_long_positions: Some(data.tape_long_positions()),
        }
    }
}
