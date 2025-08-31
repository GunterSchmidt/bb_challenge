//! This crate provides an argument handler, which may be used to support typical arguments, e.g.
//! '-m 1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA'. See below in the help_string().

use crate::machine_generic::MachineGeneric;

/// This is the return value of the argument handler
#[non_exhaustive]
pub enum ArgValue {
    // TODO machine
    // Machine(Box<Machine>),
    Machine(Box<MachineGeneric>),
    /// Returned when the arg value leads to an action which is performed directly.
    Done,
    None,
    Error(String),
}

pub fn help_string() -> String {
    let mut s = String::new();
    s.push_str("This program accepts the following arguments:\n");
    s.push_str("-b, --name <name>:           Build predefined machine");
    s.push_str("-h, --help:                  This help text\n");
    s.push_str("-m, --machine <transitions>: Run machine, e.g. '-m 1RB1LC_1RC1RB_1RD0LE_1LA1LD_1RZ0LA' or '-m 1RB2LB1RZ_2LA2RB1LB'\n");
    s.push_str("-n, --file-number <number>:  Read machine no (e.g. 42) from bb_challenge file and run it.\n");
    s
}

// TO DO Clap crate
pub fn standard_args(args: &[String]) -> ArgValue {
    // TO DO arg 0 is expected to be the path. This should be more flexible.
    if args.len() <= 1 {
        return ArgValue::None;
    }

    // match on first argument if second is optional
    match args[1].as_str() {
        "-h" | "--help" => {
            println!("{}", help_string());
            return ArgValue::Done;
        }

        _ => {}
    }

    #[allow(clippy::single_match)]
    match args.len() {
        3 => match args[1].as_str() {
            // TODO by name
            // "-b" | "--name" => {
            //     // if let Ok(no) = args[1].parse::<u64>() {}
            //     let machine = Machine::build_machine(args[2].as_str());
            //     match machine {
            //         Some(m) => return ArgValue::Machine(Box::new(m)),
            //         None => {
            //             return ArgValue::Error(format!(
            //                 "No machine with name '{}' found.",
            //                 args[2]
            //             ));
            //         }
            //     }
            // }
            "-m" | "--machine" => {
                let mg = MachineGeneric::try_from_standard_tm_text_format(&args[2]);
                match mg {
                    Ok(machine) => {
                        return ArgValue::Machine(Box::new(machine));
                    }
                    Err(e) => return ArgValue::Error(e.to_string()),
                }
            }

            // TODO
            // "-n" | "--file-number" => {
            //     if let Ok(no) = args[2].parse::<u64>() {
            //         let mut file_path = FILE_PATH_BB5_CHALLENGE_DATA_FILE;
            //         if args.len() > 3 {
            //             file_path = args[3].as_str();
            //         }
            //         // println!("Machine number: {}", no);
            //         match BBFileReader::read_machine_single(no, file_path) {
            //             Ok(machine) => return ArgValue::Machine(Box::new(machine)),
            //             Err(e) => return ArgValue::Error(format!("{e:?}")),
            //         };
            //     } else {
            //         return ArgValue::Error(format!("Invalid machine number: {}", args[2]));
            //     }
            // }

            // No valid argument
            _ => {}
        },

        // false arg count
        _ => {}
    }

    // print help
    println!("Invalid arguments: {:?}\n", &args[1..]);
    println!("{}", help_string());

    ArgValue::None
}

#[cfg(test)]
mod tests {
    use crate::machine_generic::{TransitionGeneric, B};

    use super::*;

    #[test]
    fn test_machine_2x2_6_4() {
        // 2x2-6-4
        let text = "1RB1LB_1LA1RZ";
        let args = vec!["path".to_string(), "-m".to_string(), text.to_string()];
        let r = standard_args(&args);
        let table = match r {
            ArgValue::Machine(t) => t,
            _ => todo!(),
        };
        let check_value = TransitionGeneric::try_from("1RZ").unwrap();
        let transition_b1 = table.transition_for_state_symbol(B, 1);
        println!("{}", table.to_standard_tm_text_format());
        println!("{}", table);
        assert_eq!(check_value, transition_b1);
        let tm_format = table.to_standard_tm_text_format();
        assert_eq!(text, tm_format);
    }
}
