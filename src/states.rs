use lazy_static::lazy_static;
use regex::Regex;

pub trait StateMachine {
    fn execute(&self, variable_list : &mut Vec<String>, code_list : &Vec<String> , state: usize) -> (usize, Option<Box<dyn StateMachine>>);
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
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        let code = &code_list.get(state);
        match code {
            Some(value) => {
                for new_state in TRANSITION_FUNCTIONS.iter() {
                    if new_state.0.is_match(value){
                        return (state, Some(get_state(new_state.1)))
                    }
                }
                return(state, None)
            },
            None => (state, None)
        }
    }
}

impl StateMachine for EndState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        (0, None)
    }
}

impl StateMachine for GotoState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        (0, None)
    }
}

impl StateMachine for IfState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        (0, None)
    }
}

impl StateMachine for OutputState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        (0, None)
    }
}

impl StateMachine for AssignState {
    fn execute(&self, variable_list: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> (usize, Option<Box<dyn StateMachine>>) {
        let code = &code_list.get(state); // get the line of code

        //ensure that we actually have a line of code to work with
        match code {

            //We have code.
            Some(value) => {

                //Regex used to process the assign statement
                let assign_from_code = Regex::new(r#"let M(\d+) = (([1-9]\d*)|"[a-zA-Z]*")"#).unwrap();
                let assign_from_memory = Regex::new(r"let M(\d+) = M(\d+)").unwrap();

                // Check if assigning from a hardcoded value
                if assign_from_code.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_code.captures(value).unwrap();
                    let memory_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address

                    // Update the given memory address to the new value if it's not out of bounds
                    // and move back to the execute state.
                    if variable_list.len() > *memory_pos {
                        variable_list[*memory_pos] = (&assign_tokens[2]).parse::<String>().unwrap().replace("\"", "");
                        (state + 1, Some(Box::new(ExecuteState {})))
                    } else {
                        (state, None) // Tell the interpreter to exit
                    }

                    // Check if assigning from memory
                } else if assign_from_memory.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_memory.captures(value).unwrap();
                    let lhs_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address for LHS
                    let rhs_pos = &assign_tokens[2].parse::<usize>().unwrap(); // get the memory address for RHS
                    if variable_list.len() > *lhs_pos && variable_list.len() > *rhs_pos {
                        variable_list[*lhs_pos] = (variable_list[*rhs_pos]).parse().unwrap();
                        (state + 1, Some(Box::new(ExecuteState {})))
                    } else {
                        (state, None) // Tell the interpreter to exit
                    }
                    // No valid assign statement
                } else {
                    (state, None) // Tell the interpreter to exit
                }
            },
            None => (state, None) // Tell the interpreter to exit
        }
    }
}

#[cfg(test)]
mod test {
    use super::{AssignState, StateMachine, ExecuteState};

    #[test]
    fn check_that_start_returns_0() {
        let state = ExecuteState{}.execute(&mut vec![], &vec![], 0);
        assert_eq!(0, state.0)
    }

    #[test]
    fn assign_number_to_memory() {
        let mut memory_vec = vec![String::from("0")];
        let code_vec = vec![String::from("let M0 = 5")];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), "5")
    }

    #[test]
    fn assign_string_to_memory() {
        let mut memory_vec = vec![String::from(r#""""#)];
        let code_vec = vec![String::from(r#"let M0 = "hello""#)];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), r#"hello"#)
    }

    #[test]
    fn assign_from_memory_to_memory() {
        let mut memory_vec = vec![String::from("0"), String::from("5")];
        let code_vec = vec![String::from("let M0 = M1")];
        AssignState{}.execute(&mut memory_vec, &code_vec, 0);
        assert_eq!(memory_vec.get(0).unwrap(), "5")
    }
}