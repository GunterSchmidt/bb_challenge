use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::machine::Machine;
use crate::transition_symbol2::TransitionSymbol2;

const CSS_FOLDER: &str = "styles";
const CSS_FILE_LIGHT: &str = "light.css";
const CSS_FILE_DARK: &str = "dark.css";
const BODY_FONT_FAMILY: &str = "monospace";
pub const CLASS_HEAD_POSITION: &str = "head_pos";
pub const CLASS_CHANGED_POSITION: &str = "change_pos";
const CSS_HEAD_POSITION_LIGHT: &str = "background-color: lavenderblush; font-weight: bold;"; // color: black;
const CSS_HEAD_POSITION_DARK: &str = "background-color: lavenderblush;"; // color: black;
const CSS_CHANGE_POSITION_LIGHT: &str = "font-weight: bold;"; // color: blue;
const CSS_CHANGE_POSITION_DARK: &str = "font-weight: bold;"; // color: yellow;
const CSS_HTML: &str = "html {
    background-color: white;
    height: 100%;
    margin: 0px;
    padding: 0px;
}";

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

// pub fn create_html_header(path: &str) -> io::Result<()> {
//     // Create and write to HTML file
//     let mut html_file = File::create(&path)?;
//     write_html_header(&mut html_file)?;
//     Ok(())
// }

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

pub fn format_int_html(number: usize, size: usize) -> String {
    let s = format!("{number:>size$}");
    s.replace(" ", "&nbsp;")
}

pub fn blanks(num_blanks: usize) -> String {
    "&nbsp;".repeat(num_blanks)
}

/// Writes to html header and the start of the body to the disk.
/// ## Arguments
/// * path of the html file
/// * file_name_prefix
/// * machine to write, uses tm_standard_name for file name
/// * Example: ("data", "hold", m) creates /data/hold_1RB0RC_1RA0RA_0RB1RC.html
///
/// Returns (File, FileName)
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
    writeln!(
        file,
        "  <h2>BB{} {decider_name} Machine{m_id} {}</h2>",
        machine.n_states(),
        machine.to_standard_tm_text_format()
    )?;
    writeln!(file, "  <p>")?;

    Ok((file, file_name))
}

pub fn write_file_end(file: &mut File) -> io::Result<()> {
    writeln!(file, "  </p>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

pub fn write_step_html_64(
    file: &mut File,
    step_no: usize,
    tr_field_id: usize,
    transition: &TransitionSymbol2,
    tape_shifted: u64,
) {
    writeln!(
        file,
        "Step {} {} {transition}: {}</br>",
        format_int_html(step_no, 5),
        TransitionSymbol2::field_id_to_string(tr_field_id),
        crate::tape_utils::U64Ext::to_binary_split_html_string(&tape_shifted, transition),
    )
    .expect("Html write error");
}

pub fn write_step_html_128(
    file: &mut File,
    step_no: usize,
    tr_field_id: usize,
    transition: &TransitionSymbol2,
    tape_shifted: u128,
) {
    writeln!(
        file,
        "Step {} {} {transition}: {}</br>",
        format_int_html(step_no, 5),
        TransitionSymbol2::field_id_to_string(tr_field_id),
        crate::tape_utils::U128Ext::to_binary_split_html_string(&tape_shifted, transition),
    )
    .expect("Html write error");
}

pub fn write_html_p(file: &mut File, text: &str) {
    writeln!(file, "<p>{text}</p>",).expect("Html write error");
}
