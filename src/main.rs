mod code_loader;
mod errors;
mod states;

use std::process::exit;
use crate::states::{States, get_state};

fn main() {
    let program_file = std::env::args().nth(1);
    match program_file {
        Some(data) => {
            let program = code_loader::load_code_from_file(data);
            match program {
                Ok((mut reg_data, code_data)) => {
                    let mut state = get_state(States::ExecuteState).execute(&mut reg_data, &code_data, 0);
                    loop {
                        let state_function = state.1;
                        match state_function {
                            Some(ref new_state) => state = new_state.execute(&mut reg_data, &code_data, state.0),
                            None => exit(0)
                        }
                    }
                },
                Err(error_msg) => {
                    eprintln!("{}", error_msg);
                    exit(-1);
                }
            }
        },
        None => {
            eprintln!("No program file specified! Aborting...");
            exit(-1);
        }
    };
}

