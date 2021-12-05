extern crate lazy_static;

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
                        if state.as_ref().is_err() {
                            eprintln!("{}", state.err().unwrap());
                            exit(-1);
                        }
                        let state_function = &state.as_ref().ok().unwrap().1;
                        let new_state = state.as_ref().ok().unwrap().0;
                        state = state_function.execute(&mut reg_data, &code_data, new_state);
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

