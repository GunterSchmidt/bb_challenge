//! # Functionality
//! Output the machine steps while in progress into an HTML file. \
//! The output is limited to the 128-Bit tape_shifted, so not the full tape is seen, but this is usually enough to analyze. \
//! The step line looks like this (here only 64 bits are shown): \
//! Step     1 A0 1RC: 000000000000000000000000_0000000**1**\*00000000_000000000000000000000000 \
//! where the Head is always directly after the '\*'.
//! So here the first step is from table field A0 having transition 1RC, so it writes a 1 on the tape and shifts the head right.
//! This translates into a shift left for the tape if the head is held in a fixed position.
//! The \[1\] is just a bold 1 in html indicating the changed cell. The head now rests at the first 0 after the '*'.
//! The output is limited to WRITE_HTML_STEP_LIMIT (100_000) steps, but can be set higher in config. The full output of BB5_MAX takes
//! a while and creates a 10 GB large html file (showing each step). \
//! In case of the self-ref speed-up not all steps are shown, as the repetitions are omitted. Makes the file much smaller. \
//! Since the step number is used for the config.write_html_step_limit, the file can be very small. \
//! Instead use config.write_html_line_limit which counts the actually written steps. For BB5_MAX only 91,021 of 47,176,870 = 0.02 %
//! are written, which makes it possible to write the full output to an only 19,1 MB large html file.
//!
//! # How this is used
//! - Create a new HtmlWriter.
//! - create_html_file_start is used open the file and write the header. Also creates the directory and css files if they do not exist.
//!
//! # Config
//!
//! ## Features
//! - Enable Feature 'enable_html_reports' to generally allow html output.
//! Even if no output is generated this causes a 10-30% performance degradation.
//! This config is not in the toml file as it would cause to much performance delay.
//!
//! ## config.toml
//! - path: html_out_path (subdirectories will be created automatically depending on decider)
//! - html_tape_shifts: If true, then the head is always in the middle and the tape shifts. Else head moves.
//!
//! ## Program Config
//! Set Config.write_html_file(true) to enable html output for that decider.

use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::{Path, MAIN_SEPARATOR_STR},
    time::Instant,
};

use crate::{
    config::{self, Config, StepTypeBig},
    decider::{decider_hold_long::DeciderHoldLong, Decider},
    machine_binary::MachineBinary,
    machine_info::MachineInfo,
    status::MachineStatus,
    tape::tape_utils::TapeLongPositions,
    transition_binary::TransitionBinary,
};

use crate::decider::DeciderId;

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

#[derive(Debug)]
pub struct HtmlWriter {
    /// limits output to file by the actual written lines (only steps count).
    write_html_line_limit: u32,
    /// current line count
    write_html_line_count: u32,
    write_html_step_start: StepTypeBig,
    write_html_tape_shifted_64_bit: bool,

    n_states: usize,
    /// Main path without sub directory
    html_out_path: String,
    // / Sub-dir. This is mandatory, the option is only to check if it is set.
    // sub_dir: Option<String>,
    /// full path, set_sub_dir to set this path, mandatory.
    path: Option<String>,
    file_name: Option<String>,
    buf_writer: Option<BufWriter<File>>,
}

impl HtmlWriter {
    pub fn new(config: &Config) -> Self {
        Self {
            // decider_id,
            write_html_line_limit: if config.write_html_file() {
                config.write_html_line_limit()
            } else {
                0
            },
            write_html_line_count: 0,
            write_html_step_start: if config.write_html_file() {
                config.write_html_step_start()
            } else {
                0
            },
            write_html_tape_shifted_64_bit: config.write_html_tape_shifted_64_bit(),

            n_states: config.n_states(),
            html_out_path: config.config_toml().html_out_path().to_string(),
            path: None,
            file_name: None,
            buf_writer: None,
        }
    }

    /// Sets the sub directory. This is mandatory.
    /// # Panics
    /// If the sub directory could not be created.
    pub fn init_sub_dir(&mut self, sub_dir: &str) {
        // self.sub_dir = Some(dir.to_string());
        let path = format!(
            "{}{MAIN_SEPARATOR_STR}{sub_dir}_bb{}",
            self.html_out_path, self.n_states
        );
        let msg = format!("CSS files could not be created in {path}.");
        create_css(&path).expect(&msg);

        self.path = Some(path);
    }

    pub fn file_name(&self) -> Option<&String> {
        self.file_name.as_ref()
    }

    /// Returns true if html is enabled and the step_no is < 1000 or > config.write_html_step_start .
    /// step_no must be smaller or equal \
    /// line count must be smaller, so one more can fit
    pub fn is_write_html_in_limit(&self, step_no: StepTypeBig) -> bool {
        self.write_html_line_limit != 0
            && (step_no <= 1000 || step_no >= self.write_html_step_start)
            && self.write_html_line_count < self.write_html_line_limit
    }

    /// Checks if config.write_html_file was set to true and if the path is set
    pub fn is_write_html_file(&self) -> bool {
        self.write_html_line_limit != 0 && self.path.is_some()
    }

    pub fn path(&self) -> Option<&String> {
        self.path.as_ref()
    }

    /// Reset line count when HtmlWriter is reused.
    pub fn reset_write_html_line_count(&mut self) {
        self.write_html_line_count = 0;
    }

    pub fn create_html_file_start(
        &mut self,
        decider_id: &DeciderId,
        machine: &MachineBinary,
    ) -> io::Result<()> {
        let mi = MachineInfo::from(machine);
        self.create_html_file_start_m_info(decider_id, &mi)
    }
    /// Writes to html header and the start of the body to the file. \
    /// Sets file_name in self.
    /// # Arguments
    /// - decider_id: For file name and description
    /// - machine to write, uses tm_standard_name for file name
    ///
    /// Example: ("data", "hold", m) creates /data/hold_1RB0RC_1RA0RA_0RB1RC.html
    pub fn create_html_file_start_m_info(
        &mut self,
        decider_id: &DeciderId,
        machine: &MachineInfo,
    ) -> io::Result<()> {
        match &self.path {
            Some(path) => {
                if !std::fs::exists(path)? {
                    std::fs::create_dir_all(path)?;
                }
                let file_name = decider_id.name.replace(" ", "_").to_lowercase()
                    + "_"
                    + machine.file_name().as_str()
                    + ".html";
                let p = Path::new(&path).join(&file_name);
                let mut file = File::create(&p)?;
                write_html_header(&mut file, &machine.to_standard_tm_text_format())?;
                writeln!(file, "<body>")?;
                let m_id = if machine.has_id() {
                    format!(" Id: {}", machine.id())
                } else {
                    String::new()
                };
                // write header, machine, e.g.
                // BB4 Decider Cycler Machine Id: 32538705 0RC1LC_---1RC_1LD1RB_1RA0RA
                writeln!(
                    file,
                    "  <h2>BB{} {} Machine{m_id} {}</h2>",
                    machine.n_states(),
                    decider_id.name,
                    machine.to_standard_tm_text_format()
                )?;
                // Machine transitions as table
                writeln!(file, "{}", machine.machine().to_table_html_string(true))?;

                // write self-referencing
                if machine.has_self_referencing_transition() {
                    let ts = machine.machine().get_self_referencing_transitions();
                    let mut s = vec![String::from("Self-referencing transitions:")];
                    for t in ts.iter() {
                        s.push(format!("{} {t}", t.self_ref_array_id_to_field_name()));
                    }

                    let text = s.join("</br>");
                    writeln!(file, "<p>{text}</p>")?;
                    writeln!(file,"<p>Note: This machine has self-referencing transitions (e.g. Field A1: 1RA) \
                which leads to repeatedly calling itself in case of tape head reads 1. This is used to speed up the \
                decider by jumping over these repeated steps.</p>")?;
                }

                // start with an opening <p> tag, as the lines end with </br>, but this actually leads to nested paragraphs.
                // writeln!(file, "  <p>")?;

                self.buf_writer = Some(BufWriter::new(file));
                self.file_name = Some(file_name);
                self.write_html_line_count = 0;

                Ok(())
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No path defined, possibly sub directory not set.",
            )),
        }
    }

    pub fn rename_file_to_status_self(&mut self, status: &MachineStatus) {
        rename_file_to_status(
            self.path.as_ref().unwrap(),
            self.file_name.as_ref().unwrap(),
            status,
        );
    }

    pub fn write_html_file_end(&mut self, step_no: StepTypeBig, status: &MachineStatus) {
        // no if let as borrow checker would complain
        if self.buf_writer.is_some() {
            use num_format::ToFormattedString;
            let locale = config::user_locale();
            if self.write_html_line_count >= self.write_html_line_limit {
                self.write_html_p(
                    format!(
                        "HTML Line Limit ({}) reached, total lines: {}.",
                        self.write_html_line_count.to_formatted_string(&locale),
                        step_no.to_formatted_string(&locale)
                    )
                    .as_str(),
                );
            } else if self.write_html_line_count < step_no {
                let p =
                    ((self.write_html_line_count as f64 / step_no as f64) * 1000.0).round() / 10.0;
                self.write_html_p(
                    format!(
                        "Steps executed (single step or step jump): {} of {} = {p} %.",
                        self.write_html_line_count.to_formatted_string(&locale),
                        step_no.to_formatted_string(&locale)
                    )
                    .as_str(),
                );
            }
            // if self.step_no >= self.write_html_step_limit {
            //     self.write_html_p(
            //         format!(
            //             "HTML Step Limit ({}) reached, total steps: {}.",
            //             self.write_html_step_limit.to_formatted_string(&locale),
            //             self.step_no.to_formatted_string(&locale)
            //         )
            //         .as_str(),
            //     );
            // }
            let text = format!("{}", status);
            self.write_html_p(&text);
            if let Some(buf_writer) = self.buf_writer.as_mut() {
                crate::html::write_file_end(buf_writer).expect("Html file could not be written")
            }

            // dbg!(
            //     self.file_name.as_ref().unwrap(),
            //     self.path.as_ref().unwrap()
            // );
            // Rename file to status
            self.rename_file_to_status_self(status);
        }
    }

    pub fn write_html_p(&mut self, text: &str) {
        if let Some(buf_writer) = self.buf_writer.as_mut() {
            write_html_p(buf_writer, text);
        }
    }

    /// Write a single step to the html file.
    /// # Panics
    /// If file cannot be written. Unlikely as the file is already open for write.
    pub fn write_step_html(&mut self, step_data: &StepHtml) {
        if self.is_write_html_in_limit(step_data.step_no) {
            step_data.write_step_html(self.buf_writer.as_mut().unwrap());
            self.write_html_line_count += 1;
        }
    }

    pub fn write_html_tape_shifted_64_bit(&self) -> bool {
        self.write_html_tape_shifted_64_bit
    }
}

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

/// Formats an Integer right aligned
pub fn format_right_aligned_int_html(number: usize, size: usize) -> String {
    let s = format!("{number:>size$}");
    s.replace(" ", "&nbsp;")
}

/// Creates the folder path of the html file and the css files in the folder if not already existing.
/// # Returns
/// - the path for the html files, like '/result/<sub_path>_bb5', e.g. '/result/cycler_bb5' \
/// - None if write_html_file in [Config] is set to false.
/// # Panics
/// If the path could not be created.
pub fn get_html_path(sub_path: &str, config: &Config) -> Option<String> {
    // if config.write_html_file() {
    let path = format!(
        "{}{MAIN_SEPARATOR_STR}{sub_path}_bb{}",
        config.config_toml().html_out_path(),
        config.n_states()
    );
    let msg = format!("CSS files could not be created in {path}.");
    create_css(&path).expect(&msg);
    Some(path)
    // } else {
    //     None
    // }
}

/// Rename file depending on status, Decided or Undecided will be added to the file name.
/// #Panics
/// If file could not be renamed.
pub fn rename_file_to_status(file_path: &str, file_name: &str, machine_status: &MachineStatus) {
    let old_path = format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, file_name);
    let new_path = match machine_status {
        MachineStatus::NoDecision => todo!(),
        MachineStatus::EliminatedPreDecider(_) => todo!(),
        MachineStatus::Undecided(_, _, _) => {
            // rename file
            let f_name_new = "undecided_".to_string() + file_name;
            Some(format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, f_name_new))
        }
        MachineStatus::DecidedHalts(steps) => {
            // rename file
            let f_name_new = format!("decided_halt_{steps}_{}", file_name);
            Some(format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, f_name_new))
        }
        MachineStatus::DecidedNonHalt(_) => {
            // rename file
            let f_name_new = format!("decided_non_halt_{}", file_name);
            Some(format!("{}{}{}", file_path, MAIN_SEPARATOR_STR, f_name_new))
        }
        _ => {
            // rename file
            // dbg!(machine_status);
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

pub fn write_file_end(buf_writer: &mut BufWriter<File>) -> io::Result<()> {
    writeln!(buf_writer, "</body>")?;
    writeln!(buf_writer, "</html>")?;
    buf_writer.flush().expect("Could not flush");
    Ok(())
}

// #[deprecated]
// pub fn write_step_html_128(
//     buf_writer: &mut BufWriter<File>,
//     step_no: StepTypeBig,
//     tr_field_id: usize,
//     transition: TransitionBinary,
//     tape_shifted: u128,
//     pos_middle: i64,
// ) {
//     let data = StepHtml {
//         step_no,
//         tr_field_id,
//         transition,
//         tape_shifted,
//         is_u128_tape: true,
//         pos_middle,
//         tape_long_positions: None,
//     };
//     data.write_step_html(buf_writer);
// }

/// Writes a text into an open Html file.
/// # Panics
/// Panics on file write error. At this point it is unlikely to occur.
pub fn write_html(buf_writer: &mut BufWriter<File>, text: &str) {
    writeln!(buf_writer, "{text}",).expect("Html write error");
    #[cfg(feature = "bb_debug")]
    buf_writer.flush().expect("Could not flush");
}

/// Writes a paragraphed text into an open Html file.
/// # Panics
/// Panics on file write error. At this point it is unlikely to occur.
pub fn write_html_p(buf_writer: &mut BufWriter<File>, text: &str) {
    writeln!(buf_writer, "<p>{text}</p>",).expect("Html write error");
    #[cfg(feature = "bb_debug")]
    buf_writer.flush().expect("Could not flush");
}

/// Writes a batch of machines to html
pub fn write_machines_to_html(
    machine_infos: &[MachineInfo],
    description: &str,
    config: &Config,
    limit_num_files: usize,
    as_64_bit: bool,
) {
    println!(
        "Writing {} '{description}' machines to html...",
        machine_infos.len()
    );
    let config = Config::builder_from_config(config)
        .write_html_file(true)
        .write_html_tape_shifted_64_bit(as_64_bit)
        // .step_limit_cycler(100_000)
        // .step_limit_bouncer(100_000)
        // .step_limit_hold(100_000)
        // .write_html_line_limit(25_000)
        .build();
    if config.write_html_file() {
        let mut last_progress_info = Instant::now();
        for (i, m_info) in machine_infos.iter().take(limit_num_files).enumerate() {
            // let machine = Machine::from(m_info);
            // write hold (because self ref)
            DeciderHoldLong::decide_single_machine(&m_info.machine(), &config);
            // write bouncer (because single step)
            crate::decider::decider_bouncer_128::DeciderBouncer128::decide_single_machine(
                &m_info.machine(),
                &config,
            );
            // write cycler (because single step)
            crate::decider::decider_cycler::DeciderCycler::decide_single_machine(
                &m_info.machine(),
                &config,
            );
            let dur = Instant::now() - last_progress_info;
            if dur.as_millis() > 5000 {
                println!("progress: {} / {}", i + 1, machine_infos.len());
                last_progress_info = Instant::now();
            }
        }
        println!("done.");
    }
}

/// All data required to write a step to the html file and write functionality. \
/// This serves two purposes:
/// - Always show the tape with the head in the middle, regardless which underlying tape storage.
/// - Show identical data as far as possible.
#[derive(Debug, Clone, Copy)]
pub struct StepHtml {
    /// Current step no, starting at 1.
    pub step_no: StepTypeBig,
    /// Table field which lead to the current transition.
    pub tr_field_id: usize,
    /// Current transition
    pub transition: TransitionBinary,
    /// Tape after the transition was executed. Displayed as 128-Bit with head as bit 63.
    pub tape_shifted: u128,
    /// if false the lower 64 bit will be used. This can also be used to only print the middle part if the tape is shifted before by 32 bit.
    pub is_u128_tape: bool,
    /// current pos_middle of tape shifted, it is not the real delta to pos_start
    pub pos_middle: i64,
    /// current tape_long if available or necessary
    pub tape_long_positions: Option<TapeLongPositions>,
}

impl StepHtml {
    /// Write a single step to the html file. \
    /// # Panics
    /// If file cannot be written. Unlikely as the file is already open for write.
    pub fn write_step_html(&self, buf_writer: &mut BufWriter<File>) {
        write_html(buf_writer, &self.step_to_html_fmt());
    }

    /// Formats the line
    pub fn step_to_html_fmt(&self) -> String {
        let binary = if self.is_u128_tape {
            crate::tape::tape_utils::U128Ext::to_binary_split_html_string(
                &self.tape_shifted,
                &self.transition,
            )
        } else {
            crate::tape::tape_utils::U64Ext::to_binary_split_html_string(
                &(self.tape_shifted as u64),
                &self.transition,
            )
        };
        let tl_pos = if let Some(tp) = &self.tape_long_positions {
            format!(
                " TL P {} {}..{}",
                Self::format_right_aligned_int_html(tp.tl_pos as isize, 3),
                tp.tl_low_bound,
                tp.tl_high_bound
            )
        } else {
            String::new()
        };
        format!(
            "<p class=\"p_step\">Step {} {} {}: {binary} P: {}{}</p>",
            Self::format_right_aligned_int_html(self.step_no as isize, 5),
            MachineBinary::array_id_to_field_name(self.tr_field_id),
            self.transition,
            Self::format_right_aligned_int_html(self.pos_middle as isize, 3),
            tl_pos
        )
    }

    /// Formats an Integer right aligned
    pub fn format_right_aligned_int_html(number: isize, size: usize) -> String {
        let s = format!("{number:>size$}");
        s.replace(" ", "&nbsp;")
    }
}
