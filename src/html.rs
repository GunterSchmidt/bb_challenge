use std::fs::File;
use std::io::{self, Write};
// use std::path::Path;

pub fn create_html() -> io::Result<()> {
    // Define file names
    let html_file_name = "index.html";
    let light_css_file_name = "light.css";
    let dark_css_file_name = "dark.css";

    // Create and write to HTML file
    let mut html_file = File::create(html_file_name)?;
    write_html_content(&mut html_file, light_css_file_name, dark_css_file_name)?;

    // Create and write to light mode CSS file
    let mut light_css_file = File::create(light_css_file_name)?;
    write_light_css_content(&mut light_css_file)?;

    // Create and write to dark mode CSS file
    let mut dark_css_file = File::create(dark_css_file_name)?;
    write_dark_css_content(&mut dark_css_file)?;

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
    writeln!(file, "        body {{ font-family: sans-serif; }}")?;
    writeln!(
        file,
        "        .blue-two {{ color: blue; background-color: gray; padding: 2px 0; }}"
    )?;
    writeln!(file, "    </style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;

    for i in 1..=100 {
        writeln!(
            file,
            "    <p>{}: 1RB 001<span class=\"blue-two\">2</span>0</p>",
            i
        )?;
    }

    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;
    Ok(())
}

fn write_light_css_content(file: &mut File) -> io::Result<()> {
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: white;")?;
    writeln!(file, "    color: black;")?;
    writeln!(file, "}}")?;
    Ok(())
}

fn write_dark_css_content(file: &mut File) -> io::Result<()> {
    writeln!(file, "body {{")?;
    writeln!(file, "    background-color: black;")?;
    writeln!(file, "    color: white;")?;
    writeln!(file, "}}")?;
    Ok(())
}
