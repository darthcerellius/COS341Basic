use std::io;
use std::process::exit;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use num_integer::div_rem;

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
    input.trim().to_string()
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
    fn execute(&self, registers : &mut Vec<String>, code_list : &Vec<String> , state: usize) -> Result<(usize, Box<dyn StateMachine>),String>;
}

fn fetch_and_execute<T>(
    code: &Option<&String>,
    regular_expression: Regex,
    mut executor: T,
    error_msg: &str
) -> Result<(usize, Box<dyn StateMachine>), String> where T: FnMut(&String, Captures) ->
Result<(usize, Box<dyn StateMachine>), String> {
    match code {
        Some(value) => {
            if regular_expression.is_match(value) {
                executor(value, regular_expression.captures(value).unwrap())
            } else {
                Err(format!("{}: {}\nAborting...", error_msg, value))
            }
        },
        None => Ok((0, get_state(States::QuitState)))
    }
}

#[derive(Copy, Clone)]
pub enum States {
    AssignState,
    ExecuteState,
    GotoState,
    IfState,
    QuitState,
    OutputState,
    MathState
}

struct EndState {}
struct AssignState {}
struct ExecuteState {}
struct IfState{}
struct GotoState{}
struct OutputState{}
struct MathState {}

lazy_static! {
    static ref TRANSITION_FUNCTIONS: [(Regex, States); 5] = [
        (Regex::new(r"let").unwrap(), States::AssignState),
        (Regex::new(r"if").unwrap(), States::IfState), //must go before 'goto'
        (Regex::new(r"goto").unwrap(), States::GotoState),
        (Regex::new(r"quit").unwrap(), States::QuitState),
        (Regex::new(r"output").unwrap(), States::OutputState)
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
        States::MathState => Box::new(MathState{}),
    }
}

impl StateMachine for ExecuteState {
    fn execute(&self, _: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state);
        fetch_and_execute(
            code,
          Regex::new("(.*)").unwrap(),
          |value, _| -> Result<(usize, Box<dyn StateMachine>),String>
              {
                  //Find the correct state to move to
                  for new_state in TRANSITION_FUNCTIONS.iter() {
                      if new_state.0.is_match(value){
                          return Ok((state, get_state(new_state.1)));
                      }
                  }
                  return Err(format!("Unknown instruction: {}\nAborting...", value));
              },
            "Unknown instruction")
    }
}

impl StateMachine for EndState {
    fn execute(&self, _: &mut Vec<String>, _: &Vec<String>, _: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        exit(0);
    }
}

impl StateMachine for GotoState {
    fn execute(&self, _: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state);
        fetch_and_execute(
            code,
            Regex::new(r"goto (\d+)").unwrap(),
            |_, goto_capture| -> Result<(usize, Box<dyn StateMachine>),String>
                {
                    let goto_ptr = goto_capture[1].parse::<usize>().unwrap();
                    if goto_ptr >= code_list.len() {
                        Err(format!("Goto statement points to region out of bounds!\nAborting..."))
                    } else {
                        Ok((goto_ptr, get_state(States::ExecuteState)))
                    }
                },
            "Invalid goto statement")
    }
}

impl StateMachine for IfState {
    fn execute(&self, registers: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state);
        fetch_and_execute(
            code,
            Regex::new(r"if M([0-9]+) (<=?|>=?|=|!=) M(\d+) goto (\d+)").unwrap(),
            |_, captures| {
                let lhs_pos = captures[1].parse::<usize>().unwrap();
                let rhs_pos = captures[3].parse::<usize>().unwrap();
                let code_pos = captures[4].parse::<usize>().unwrap();
                let condition = captures[2].to_string();

                if lhs_pos > registers.len() || rhs_pos > registers.len() {
                    return Err(format!("Memory index out of bounds!\nAborting"));
                }

                let lhs_val = registers.get(lhs_pos).unwrap();
                let rhs_val = registers.get(rhs_pos).unwrap();

                let predicate: fn(&str, &str) -> bool = match condition.as_ref() {
                    ">=" => |lhs, rhs| {return lhs.ge(rhs)},
                    ">" => |lhs, rhs| {return lhs.gt(rhs)},
                    "<=" => |lhs, rhs| {return lhs.le(rhs)},
                    "<" => |lhs, rhs| {return lhs.lt(rhs)},
                    "=" => |lhs, rhs| {return lhs.eq(rhs)},
                    "!=" => |lhs, rhs| {return !lhs.eq(rhs)},
                    _ => |_, _| {return false}
                };

                let goto_pos = if predicate(lhs_val, rhs_val) {
                    code_pos
                } else {
                    state + 1
                };
                Ok((goto_pos, get_state(States::ExecuteState)))
            },
            "Invalid if statement"
        )
    }
}

impl StateMachine for OutputState {
    fn execute(&self, registers: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state);
        fetch_and_execute(
            code,
            Regex::new(r"output M(\d+)").unwrap(),
            |_, output_capture| -> Result<(usize, Box<dyn StateMachine>),String>
                {
                    let mem_pos = output_capture[1].parse::<usize>().unwrap();
                    let data = registers.get(mem_pos);
                    match data {
                        Some(value) => write_output(value.to_string()),
                        None => return Err(format!("Memory index out of bounds!\nAborting..."))
                    }
                    Ok((state + 1, get_state(States::ExecuteState)))
                },
            "Lolwut")
    }
}

impl StateMachine for AssignState {
    fn execute(&self, registers: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>),String> {
        let code = &code_list.get(state); // get the line of code

        //ensure that we actually have a line of code to work with
        match code {

            //We have code.
            Some(value) => {

                //Regex used to process the assign statement
                let assign_from_code = Regex::new(r#"let M(\d+) = (([1-9]\d*)|"[a-zA-Z ]*")"#).unwrap();
                let assign_from_memory = Regex::new(r"let M(\d+) = M(\d+)").unwrap();
                let assign_from_input = Regex::new(r"let M(\d+) = input").unwrap();
                let assign_from_operation = Regex::new(r"let M(\d+) = M(\d+) ([+\-*/]) M(\d+)").unwrap();

                // Check if assigning from a hardcoded value
                if assign_from_code.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_code.captures(value).unwrap();
                    let memory_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address

                    // Update the given memory address to the new value if it's not out of bounds
                    // and move back to the execute state.
                    if registers.len() > *memory_pos {
                        registers[*memory_pos] = (&assign_tokens[2]).parse::<String>().unwrap().replace("\"", "");
                        Ok((state + 1, get_state(States::ExecuteState)))
                    } else {
                        Err(format!("Accessing register that is not allocated: {}\nAborting", *memory_pos))
                    }

                    // Check if assigning from operation
                } else if assign_from_operation.is_match(&format!("{}", value)) {
                    Ok((state, get_state(States::MathState)))
                } else if assign_from_input.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_input.captures(value).unwrap();
                    let memory_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address

                    // Update the given memory address to the new value if it's not out of bounds
                    // and move back to the execute state.
                    if registers.len() > *memory_pos {
                        registers[*memory_pos] = get_input();
                        Ok((state + 1, get_state(States::ExecuteState)))
                    } else {
                        Err(format!("Accessing register that is not allocated: {}\nAborting", *memory_pos))
                    }
                    // Check if assigning from operation
                } else if assign_from_memory.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_memory.captures(value).unwrap();
                    let lhs_pos = &assign_tokens[1].parse::<usize>().unwrap(); // get the memory address for LHS
                    let rhs_pos = &assign_tokens[2].parse::<usize>().unwrap(); // get the memory address for RHS

                    if registers.len() < *lhs_pos {
                        return Err(format!("Accessing register that is not allocated: {}\nAborting", *lhs_pos));
                    }

                    if registers.len() < *rhs_pos {
                        return Err(format!("Accessing register that is not allocated: {}\nAborting", *rhs_pos));
                    }

                    registers[*lhs_pos] = (registers[*rhs_pos]).parse().unwrap();
                    Ok((state + 1, get_state(States::ExecuteState)))
                    // No valid assign statement
                } else {
                    Err(format!("Invalid assign instruction: {}\nAborting...", value))
                }
            },
            // If we have no code to run, go straight to the exit state
            None => Ok((state, get_state(States::QuitState)))
        }
    }
}

impl StateMachine for MathState {
    fn execute(&self, registers: &mut Vec<String>, code_list: &Vec<String>, state: usize) -> Result<(usize, Box<dyn StateMachine>), String> {
        let code = &code_list.get(state);
        fetch_and_execute(
            code,
            Regex::new(r"M(\d+) = M(\d+) ([+\-*/]) M(\d+)").unwrap(),
            |_, captures| {
                let lhs_pos = captures[2].parse::<usize>().unwrap();
                let rhs_pos = captures[4].parse::<usize>().unwrap();
                let assign_pos = captures[1].parse::<usize>().unwrap();
                let operation = captures[3].to_string();

                if lhs_pos > registers.len() || rhs_pos > registers.len() {
                    return Err(format!("Memory index out of bounds!\nAborting"));
                }

                let lhs_val = registers.get(lhs_pos).unwrap().parse::<i128>();
                let rhs_val = registers.get(rhs_pos).unwrap().parse::<i128>();

                if lhs_val.is_err() {
                    return Err(format!("LHS register is not a number!\nAborting..."));
                }
                if rhs_val.is_err() {
                    return Err(format!("RHS register is not a number!\nAborting..."));
                }
                let result = match operation.as_ref() {
                    "*" => format!("{}", lhs_val.unwrap() * rhs_val.unwrap()),
                    "/" => {
                        let result = div_rem(lhs_val.unwrap(), rhs_val.unwrap());
                        format!("{}.{}", result.0, result.1)
                    },
                    "+" => format!("{}", lhs_val.unwrap() + rhs_val.unwrap()),
                    "-" => format!("{}", lhs_val.unwrap() - rhs_val.unwrap()),
                    _ =>  panic!()
                };

                //assign the quotient and division to two registers
                if result.contains(".") {

                    //ensure that we can write the remainder to a register
                    if assign_pos >= registers.len() {
                        return Err(format!("Division statement cannot write to register not allocated!\nAborting..."));
                    }
                    let division: Vec<&str> = result.split(r".").collect();
                    registers[assign_pos] = division[0].to_string();
                    registers[assign_pos+1] = division[1].to_string();
                } else {
                    registers[assign_pos] = result;
                }

                Ok((state+1, get_state(States::ExecuteState)))
            },
            "Lolwut"
        )
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