use std::fmt::format;
use std::io;
use std::process::exit;
use lazy_static::lazy_static;
use regex::Regex;

#[cfg(test)]
static mut IO_BUFFER: String = String::new();

#[cfg(test)]
fn get_input() -> String {
    let mut ret_str = String::new();
    unsafe {
        ret_str = IO_BUFFER.clone();
    }
    ret_str
}

#[cfg(not(test))]
fn get_input() -> String {
    let mut input = String::new();
    let input_result = io::stdin().read_line(&mut input);
    match input_result {
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error reading input!\nAborting...");
            exit(-1);
        }
    }
    input
}

#[cfg(test)]
fn write_output(out_string: String) {
    unsafe {
        IO_BUFFER = out_string;
    }
}

#[cfg(not(test))]
fn write_output(out_string: String) {
    println!("{}", out_string);
}

pub trait StateMachine {
    fn execute(&self, variable_list : &mut Vec<String>, code_list : &Vec<String> , state: usize) -> Result<(usize, Box<dyn StateMachine>),String>;
}

#[derive(Copy, Clone)]
pub enum States {
    AssignState,
    ExecuteState,
    GotoState,
    IfState,
    QuitState,
    OutputState
}

struct EndState {}
struct AssignState {}
struct ExecuteState {}
struct IfState{}
struct GotoState{}
struct OutputState{}

lazy_static! {
    static ref TRANSITION_FUNCTIONS: [(Regex, States); 5] = [
        (Regex::new(r"let").unwrap(), States::AssignState),
        (Regex::new(r"goto").unwrap(), States::GotoState),
        (Regex::new("if").unwrap(), States::IfState),
        (Regex::new("quit").unwrap(), States::QuitState),
        (Regex::new("output").unwrap(), States::OutputState)
    ];
}

/// Returns the desired state based on the provided state type
/// # Arguments
/// * state_type - Determine the type of state to return
/// # Returns
/// Box<dyn StateMachine> - A state that implements the 'StateMachine' trait.
///
/// # Examples
/// ```
/// let state = get_state(States::EndState);
/// state.execute(vec![], vec![], 0);
/// ```
pub fn get_state(state_type: States) -> Box<dyn StateMachine> {
    match state_type {
        States::AssignState => Box::new(AssignState{}),
        States::GotoState => Box::new(GotoState{}),
        States::IfState => Box::new(IfState{}),
        States::QuitState => Box::new(EndState{}),
        States::OutputState => Box::new(OutputState{}),
        States::ExecuteState => Box::new(ExecuteState{}),
    }
}

impl StateMachine for ExecuteState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state);
        match code {
            Some(value) => {

                //Find the correct state to move to
                for new_state in TRANSITION_FUNCTIONS.iter() {
                    if new_state.0.is_match(value){
                        return Ok((state, get_state(new_state.1)));
                    }
                }
                return Err(format!("Unknown instruction: {}\nAborting...", value));
            },
            // If we have no code to run, go straight to the exit state
            None => Ok((state, get_state(States::QuitState)))
        }
    }
}

impl StateMachine for EndState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        Ok((state, get_state(States::QuitState)))
    }
}

impl StateMachine for GotoState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let goto_regex = Regex::new(r"goto (\d+)").unwrap();
        let code = &code_list.get(state);
        match code {
            Some(value) => {
                if goto_regex.is_match(value) {
                    let goto_capture = goto_regex.captures(value).unwrap();
                    let goto_ptr = goto_capture[1].parse::<usize>().unwrap();
                    if goto_ptr >= code_list.len() {
                        Err(format!("Goto statement points to region out of bounds!\nAborting..."))
                    } else {
                        Ok((goto_ptr, get_state(States::ExecuteState)))
                    }
                } else {
                    Err(format!("Invalid goto statement: {}\nAborting...", value))
                }
            },
            None => Ok((state, get_state(States::QuitState)))
        }
    }
}

impl StateMachine for IfState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        Ok((state, get_state(States::QuitState)))
    }
}

impl StateMachine for OutputState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        Ok((state, get_state(States::QuitState)))
    }
}

impl StateMachine for AssignState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state); // get the line of code

        //ensure that we actually have a line of code to work with
        match code {

            //We have code.
            Some(value) => {

                //Regex used to process the assign statement
                let assign_from_code = Regex::new(r#"let M(\d+) = (([1-9]\d*)|"[a-zA-Z]*")"#).unwrap();
                let assign_from_memory = Regex::new(r"let M(\d+) = M(\d+)").unwrap();
                let assign_from_input = Regex::new(r"let M(\d+) = input").unwrap();

                // Check if assigning from a hardcoded value
                if assign_from_code.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_code.captures(value).unwrap();
                    let memory_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address

                    // Update the given memory address to the new value if it's not out of bounds
                    // and move back to the execute state.
                    if variable_list.len() > *memory_pos {
                        variable_list[*memory_pos] = (&assign_tokens[2]).parse::<String>().unwrap().replace("\"", "");
                        Ok((state + 1, get_state(States::ExecuteState)))
                    } else {
                        Err(format!("Accessing register that is not allocated: {}\nAborting", *memory_pos))
                    }

                    // Check if assigning from memory
                } else if assign_from_memory.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_memory.captures(value).unwrap();
                    let lhs_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address for LHS
                    let rhs_pos = &assign_tokens[2].parse::<usize>().unwrap(); // get the memory address for RHS

                    if variable_list.len() < *lhs_pos {
                        return Err(format!("Accessing register that is not allocated: {}\nAborting", *lhs_pos));
                    }

                    if variable_list.len() < *rhs_pos {
                        return Err(format!("Accessing register that is not allocated: {}\nAborting", *rhs_pos));
                    }

                    variable_list[*lhs_pos] = (variable_list[*rhs_pos]).parse().unwrap();
                    Ok((state + 1, get_state(States::ExecuteState)))

                    // No valid assign statement
                } else if assign_from_input.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_input.captures(value).unwrap();
                    let memory_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address

                    // Update the given memory address to the new value if it's not out of bounds
                    // and move back to the execute state.
                    if variable_list.len() > *memory_pos {
                        variable_list[*memory_pos] = get_input();
                        Ok((state + 1, get_state(States::ExecuteState)))
                    } else {
                        Err(format!("Accessing register that is not allocated: {}\nAborting", *memory_pos))
                    }
                } else {
                    Err(format!("Invalid assign instruction: {}\nAborting...", value))
                }
            },
            // If we have no code to run, go straight to the exit state
            None => Ok((state, get_state(States::QuitState)))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{get_state, States};
    use crate::states::IO_BUFFER;
    use super::{AssignState, StateMachine, ExecuteState};

    #[test]
    fn check_that_start_returns_0() {
        let state = get_state(States::ExecuteState).execute(&mut vec![], &vec![], 0);
        assert_eq!(0, state.ok().unwrap().0)
    }

    #[test]
    fn execute_state_calls_assign_state() {
        let state = get_state(States::ExecuteState);
        let mut register_vec = vec![String::from("0")];
        let code_vec = vec![String::from("let M0 = 5")];
        let result = state.execute(&mut register_vec, &code_vec, 0);
        result.as_ref().ok().unwrap().1.execute(&mut register_vec, &code_vec, result.as_ref().ok().unwrap().0);
        assert_eq!(register_vec, vec![String::from("5")])
    }

    #[test]
    fn assign_number_to_register() {
        let mut memory_vec = vec![String::from("0")];
        let code_vec = vec![String::from("let M0 = 5")];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), "5")
    }

    #[test]
    fn assign_string_to_register() {
        let mut memory_vec = vec![String::from(r#""""#)];
        let code_vec = vec![String::from(r#"let M0 = "hello""#)];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), r#"hello"#)
    }

    #[test]
    fn assign_from_register_to_register() {
        let mut memory_vec = vec![String::from("0"), String::from("5")];
        let code_vec = vec![String::from("let M0 = M1")];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), "5")
    }

    #[test]
    fn assign_register_to_input() {
        unsafe {
            IO_BUFFER = String::from("hello")
        }
        let mut register_vec = vec![String::from("0")];
        let code_vec = vec![String::from("let M0 = input")];
        AssignState{}.execute(&mut register_vec, &code_vec, 0);
        assert_eq!(register_vec.get(0).unwrap(), "hello")
    }
}