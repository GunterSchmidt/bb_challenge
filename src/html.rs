use std::fs::File;
use std::io::{self, Write};
// use std::path::Path;

const BODY_FONT_FAMILY: &str = "monospace";
const CLASS_HEAD_POSITION: &str = "head_pos";
const CLASS_CHANGED_POSITION: &str = "change_pos";
const CSS_HEAD_POSITION_LIGHT: &str =
    "color: black; background-color: lavenderblush; font-weight: bold;";
const CSS_HEAD_POSITION_DARK: &str = "color: black; background-color: lavenderblush;";
const CSS_CHANGE_POSITION_LIGHT: &str = "color: blue; font-weight: bold;";

pub fn create_html() -> io::Result<()> {
    // Define file names
    let html_file_name = "index.html";
    let light_css_file_name = "light.css";
    let dark_css_file_name = "dark.css";

    // Create and write to HTML file
    let mut html_file = File::create(html_file_name)?;
    write_html_content(&mut html_file, light_css_file_name, dark_css_file_name)?;

    // Create and write to light mode CSS file
    if !file_exists(light_css_file_name) {
        let mut light_css_file = File::create(light_css_file_name)?;
        write_light_css_content(&mut light_css_file)?;
    }

    // Create and write to dark mode CSS file
    if !file_exists(dark_css_file_name) {
        let mut dark_css_file = File::create(dark_css_file_name)?;
        write_dark_css_content(&mut dark_css_file)?;
    }

    println!(
        "Successfully created '{}', '{}', and '{}'",
        html_file_name, light_css_file_name, dark_css_file_name
    );
    println!(
        "Open '{}' in your browser to see the result.",
        html_file_name
    );

    Ok(())
}

fn write_html_content(file: &mut File, light_css: &str, dark_css: &str) -> io::Result<()> {
    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "    <meta charset=\"UTF-8\">")?;
    writeln!(
        file,
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    )?;
    writeln!(file, "    <title>Styled Text</title>")?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{}\" media=\"(prefers-color-scheme: light)\">",
        light_css
    )?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{}\" media=\"(prefers-color-scheme: dark)\">",
        dark_css
    )?;
    writeln!(
        file,
        "    <link rel=\"stylesheet\" href=\"{}\" media=\"not all and (prefers-color-scheme)\">",
        light_css
    )?; // Fallback for browsers not supporting prefers-color-scheme
    writeln!(file, "    <style>")?;
    writeln!(
        file,
        "        body {{ font-family: {BODY_FONT_FAMILY}; font-size: larger;}}"
    )?;
    writeln!(file, "    </style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;

    writeln!(file, "<p>")?;
    for i in 1..=100 {
        writeln!(
            file,
            "{}: 1RB 001<span class=\"{CLASS_HEAD_POSITION}\">2</span><span class=\"{CLASS_CHANGED_POSITION}\">0</span></br>",
            format_int(i, 10),
        )?;
    }
    writeln!(file, "</p>")?;

    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

fn write_light_css_content(file: &mut File) -> io::Result<()> {
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: white;")?;
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
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: black;")?;
    writeln!(file, "    color: white;")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, ".{CLASS_HEAD_POSITION} {{{CSS_HEAD_POSITION_DARK}}}")?;
    Ok(())
}

// check if a file exists
fn file_exists(file_path: &str) -> bool {
    std::path::Path::new(file_path).exists()
}

fn format_int(number: usize, size: usize) -> String {
    let mut s = format!("{:>size$}", number);
    s.replace(" ", "&nbsp;")
}
