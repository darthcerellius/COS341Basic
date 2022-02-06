extern crate lazy_static;

mod code_loader;
mod errors;
mod states;
mod prog_data;

use std::collections::{HashMap, LinkedList};
use std::process::exit;
use crate::prog_data::ProgramData;
use crate::states::{States, get_state};

fn main() {
    let program_file = std::env::args().nth(1);
    match program_file {
        Some(data) => {
            let program = code_loader::load_code_from_file(data);
            match program {
                Ok(code_data) => {

                    let mut prog_data = ProgramData::new(
                        code_data,
                        HashMap::new(),
                        LinkedList::new(),
                        0
                    );

                    let mut state = get_state(States::ExecuteState).execute(prog_data);
                    loop {
                        if state.as_ref().is_err() {
                            eprintln!("{}", state.err().unwrap());
                            exit(-1);
                        }
                        let result = state.ok().unwrap();
                        let state_function = result.1;
                        prog_data = result.0;
                        state = state_function.execute(prog_data);
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

