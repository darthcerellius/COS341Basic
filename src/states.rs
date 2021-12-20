use std::process::exit;
use std::io;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use num_integer::div_rem;
use rand::Rng;
use std::{thread, time::Duration};
use crate::prog_data::ProgramData;

type NewState = Result<(ProgramData, Box<dyn StateMachine>),String>;

/*
This code exists to provide a means to test that
input, output and exit states work as intended
 */
#[cfg(test)]
static mut IO_BUFFER: String = String::new();
#[cfg(test)]
static mut IS_EXIT: bool = false;

#[cfg(test)]
fn do_exit() {
    unsafe {
        IS_EXIT = true
    }
}

#[cfg(not(test))]
fn do_exit() {
    exit(0);
}

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

/// This trait is used to interpret code data and to be returned by other states.
pub trait StateMachine {
    /// Interprets code data referenced by a state offset.
    /// A new state and offset is returned if the execution was successful, otherwise an error message is returned.
    /// # Arguments
    /// * `registers` - An array of data to use for computation. Data in this array may be mutated by the function
    /// * `code_list` - An array of COS341Basic instructions to execute
    /// * `state` - Offset of the code_list parameter. Tells the function which statement to execute
    ///
    /// # Returns
    /// * `Ok((usize, Box<dyn StateMachine>))` - A tuple containing the new state and offset. The offset may
    ///                                          be the next instruction to execute or an offset specified by a 'goto' command
    /// * `Err(String)` - An error message detailing why the execution failed
    fn execute(&self, data: ProgramData) -> NewState;
}

/// Decodes a line of code using a given regex. If the decoding was successful, the passed in
/// executor function is executed, otherwise the given error message is returned. If no code string
/// is passed, this function returns the quit state.
///
/// # Arguments
/// * `code` - code that the function needs to decode
/// * `regular_expression` - Regex used to decode the code
/// * `executor` - Function to execute if the decoding was successful
/// * `error_msg` - Error to return if the decoding failed
///
/// # Returns
/// * `Ok((usize, Box<dyn StateMachine>))` - A tuple containing the new state and offset. The offset may
///                                          be the next instruction to execute or an offset specified by a 'goto' command
/// * `Err(String)` - An error message detailing why the execution failed
fn decode_and_execute<T>(
    data: ProgramData,
    regular_expression: Regex,
    mut executor: T,
    error_msg: &str
) -> NewState where T: FnMut(ProgramData, &String, Captures) -> NewState {
    let code = data.get_code();
    match code {
        Some(value) => {
            if regular_expression.is_match(&value) {
                executor(data, &value, regular_expression.captures(&value).unwrap())
            } else {
                Err(format!("{}: {}\nAborting...", error_msg, value))
            }
        },
        None => Ok((data, get_state(States::QuitState)))
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

struct EndState {} // Tell the interpreter to quit
struct AssignState {} // Assigns data to registers and gets user input
struct ExecuteState {} // Starting point for code execution
struct IfState{} // Handles conditional branching statements
struct GotoState{} // Handles unconditional jump statements
struct OutputState{} // Outputs data to the user
struct MathState {} // Handle arithmetic statements
struct PushState{} // Push data onto the stack

/*
Array of state types and conditions used by the execute state to
determine which state to transition to.
 */
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
    fn execute(&self, data: ProgramData) -> NewState {
        let code = data.get_code();
        decode_and_execute(
            data,
          Regex::new("(.*)").unwrap(),
          |data, value, _| -> NewState
              {
                  //Find the correct state to move to
                  for new_state in TRANSITION_FUNCTIONS.iter() {
                      if new_state.0.is_match(value){
                          return Ok((data, get_state(new_state.1)));
                      }
                  }
                  return Err(format!("Unknown instruction: {}\nAborting...", value));
              },
            "Unknown instruction")
    }
}

impl StateMachine for EndState {
    fn execute(&self, _: ProgramData) -> NewState {
        do_exit();
        Err(format!("Exit"))
    }
}

impl StateMachine for GotoState {
    fn execute(&self, data: ProgramData) -> NewState {
        decode_and_execute(
            data,
            Regex::new(r"goto (\d+)").unwrap(),
            |mut data, _, goto_capture| -> NewState
                {
                    let goto_ptr = goto_capture[1].parse::<usize>().unwrap();
                    if goto_ptr >= data.code_size() {
                        Err(format!("Goto statement points to region out of bounds!\nAborting..."))
                    } else {
                        data.set_index(goto_ptr);
                        Ok((data, get_state(States::ExecuteState)))
                    }
                },
            "Invalid goto statement")
    }
}

impl StateMachine for IfState {
    fn execute(&self, data: ProgramData) -> NewState {
        decode_and_execute(
            data,
            Regex::new(r"if \$(\w+) (<=?|>=?|=|!=) \$(\w+) goto (\d+)").unwrap(),
            |mut data, _, captures| {
                let lhs_name = captures[1].to_string();
                let rhs_name = captures[3].to_string();
                let code_pos = captures[4].parse::<usize>().unwrap();
                let condition = captures[2].to_string();

                if !data.contains_var(&lhs_name) {
                    return Err(format!("Variable ${} does not exist!\nAborting...", &lhs_name));
                }

                if !data.contains_var(&rhs_name) {
                    return Err(format!("Variable ${} does not exist!\nAborting...", &rhs_name));
                }

                let lhs_val = data.get_var(&lhs_name).unwrap();
                let rhs_val = data.get_var(&rhs_name).unwrap();

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
                    data.get_index() + 1
                };
                data.set_index(goto_pos);
                Ok((data, get_state(States::ExecuteState)))
            },
            "Invalid if statement"
        )
    }
}

impl StateMachine for OutputState {
    fn execute(&self, data: ProgramData) -> NewState {
        decode_and_execute(
            data,
            Regex::new(r"output \$(\w+)").unwrap(),
            |mut data, _, output_capture| -> NewState
                {
                    let var_name = output_capture[1].to_string();
                    let var_data = data.get_var(&var_name);
                    match var_data {
                        Some(value) => write_output(value.to_string()),
                        None => return Err(format!("Memory index out of bounds!\nAborting..."))
                    };
                    data.next_line();
                    Ok((data, get_state(States::ExecuteState)))
                },
            "Lolwut")
    }
}

impl StateMachine for AssignState {
    fn execute(&self, mut data: ProgramData) -> NewState {
        let code = data.get_code();

        //ensure that we actually have a line of code to work with
        match code {

            //We have code.
            Some(value) => {

                //Regex used to process the assign statement
                let assign_from_code = Regex::new(r#"let \$(\w+) = (0+|([1-9]\d*)|"[a-zA-Z ]*")"#).unwrap();
                let assign_from_memory = Regex::new(r"let \$(\w+) = \$(\w+)").unwrap();
                let assign_from_input = Regex::new(r"let \$(\w+) = input").unwrap();
                let assign_from_operation = Regex::new(r"let \$(\w+) = \$(\w+) ([+\-*/]) \$(\w+)").unwrap();
                let assign_from_stack = Regex::new(r"let \$(\w+) = pop").unwrap();

                // Check if assigning from a hardcoded value
                if assign_from_code.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_code.captures(&value).unwrap();
                    let var_name = assign_tokens[1].to_string(); // get the variable name
                    let var_val = assign_tokens[2].to_string().replace("\"", "");

                    //Set variable and go to the next line
                    data.set_var(var_name, var_val);
                    data.next_line();
                    Ok((data, get_state(States::ExecuteState)))

                    //check if assigning from stack
                } else if assign_from_stack.is_match(&format!("{}", value)) {
                    let stack_value = data.pop();

                    match stack_value {
                        Some(stack_val) => {
                            let assign_tokens = assign_from_stack.captures(&value).unwrap();
                            let var_val = assign_tokens[1].to_string();
                            data.set_var(var_val, stack_val);
                            data.next_line();
                            Ok((data, get_state(States::ExecuteState)))
                        },
                        None => Err(String::from("Stack is empty!\nAborting..."))
                    }
                    // Check if assigning from operation
                } else if assign_from_operation.is_match(&format!("{}", value)) {
                    Ok((data, get_state(States::MathState)))
                } else if assign_from_input.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_input.captures(&value).unwrap();
                    let var_name = assign_tokens[1].to_string(); // get the variable name

                    data.set_var(var_name, get_input());
                    data.next_line();
                    Ok((data, get_state(States::ExecuteState)))
                    // Check if assigning from operation

                } else if assign_from_memory.is_match(&format!("{}", value)) {
                    let assign_tokens = assign_from_memory.captures(&value).unwrap();
                    let lhs_key = assign_tokens[1].to_string(); // get the variable name for LHS
                    let rhs_key = assign_tokens[2].to_string(); // get the variable_name for RHS

                    if !data.contains_var(&rhs_key) {
                        return Err(format!("Variable ${} does not exist!\nAborting...", &rhs_key));
                    }

                    data.set_var_to_var(lhs_key, rhs_key);
                    data.next_line();

                    Ok((data, get_state(States::ExecuteState)))
                    // No valid assign statement
                } else {
                    Err(format!("Invalid assign instruction: {}\nAborting...", value))
                }
            },
            // If we have no code to run, go straight to the exit state
            None => Ok((data, get_state(States::QuitState)))
        }
    }
}

impl StateMachine for MathState {
    fn execute(&self, data: ProgramData) -> NewState {
        decode_and_execute(
            data,
            Regex::new(r"\$(\w+) = \$(\w+) ([+\-*/]) \$(\w+)").unwrap(),
            |mut data, _, captures| {
                let lhs_name = captures[2].to_string();
                let rhs_name = captures[4].to_string();
                let assign_name = captures[1].to_string();
                let operation = captures[3].to_string();

                if !data.contains_var(&lhs_name) {
                    return Err(format!("Variable ${} does not exist!\nAborting...", &lhs_name));
                }

                if !data.contains_var(&rhs_name) {
                    return Err(format!("Variable ${} does not exist!\nAborting...", &rhs_name));
                }

                let lhs_val = data.get_var(&lhs_name).unwrap().parse::<i128>();
                let rhs_val = data.get_var(&rhs_name).unwrap().parse::<i128>();

                if lhs_val.is_err() {
                    return Err(format!("${} is not a numeric value!\nAborting...", &lhs_name));
                }
                if rhs_val.is_err() {
                    return Err(format!("${} is not a numeric value!\nAborting...", &rhs_name));
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
                    let division: Vec<&str> = result.split(r".").collect();
                    data.set_var(assign_name, division[0].to_string());
                    data.push(division[1].to_string()); // push remainder onto the stack
                } else {
                    data.set_var(assign_name, result);
                }

                data.next_line();

                Ok((data, get_state(States::ExecuteState)))
            },
            "Lolwut"
        )
    }
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, LinkedList};
    use std::thread;
    use std::time::Duration;
    use rand::Rng;
    use crate::{get_state, States};
    use crate::prog_data::ProgramData;
    use crate::states::{EndState, GotoState, IO_BUFFER, IS_EXIT, MathState};
    use super::{AssignState, StateMachine, ExecuteState, OutputState, IfState};

    #[test]
    fn check_that_start_returns_0() {
        let data = ProgramData::new(Vec::new(), HashMap::new(), LinkedList::new(), 0);
        let state = get_state(States::ExecuteState).execute(data);
        assert_eq!(0, state.ok().unwrap().0.get_index())
    }

    #[test]
    fn execute_state_calls_assign_state() {
        let mut state = get_state(States::ExecuteState);
        let mut data = ProgramData::new(
            vec![String::from("let $a = 5")],
            HashMap::from([(String::from("a"), String::from("0"))]),
            LinkedList::new(),
            0
        );
        let mut result = state.execute(data).unwrap();
        state = result.1;
        data = result.0;
        result = state.execute(data).unwrap();
        assert_eq!(*(result.0.get_var(&String::from("a")).unwrap()), String::from("5"))
    }

    #[test]
    fn assign_number_to_variable() {
        let mut data = ProgramData::new(
            vec![String::from("let $a = 5")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let result = AssignState{}.execute(data);
        let res = result.ok().unwrap()
            .0.get_var(&String::from("a"))
            .unwrap().to_string();
        assert_eq!(res.as_str(), "5")
    }

    #[test]
    fn assign_string_to_variable() {
        let mut data = ProgramData::new(
            vec![String::from(r#"let $a = "hello""#)],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let result = AssignState{}.execute(data);
        let res = result.ok().unwrap()
            .0.get_var(&String::from("a"))
            .unwrap().to_string();
        assert_eq!(res.as_str(), "hello")
    }

    #[test]
    fn assign_from_variable_to_variable() {
        let mut data = ProgramData::new(
            vec![String::from("let $b = 5"), String::from("let $a = $b")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $b
        let mut result = AssignState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign the value of $b to $a
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = result.0.get_var(&String::from("a"))
            .unwrap().to_string();
        assert_eq!(res.as_str(), "5")
    }

    #[test]
    fn assign_register_to_input() {
        unsafe {
            IO_BUFFER = String::from("hello")
        }
        let mut data = ProgramData::new(
            vec![String::from(r#"let $a = input"#)],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let result = AssignState{}.execute(data);
        let res = result.ok().unwrap()
            .0.get_var(&String::from("a"))
            .unwrap().to_string();
        assert_eq!(res.as_str(), "hello")
    }

    #[test]
    fn output_int_register() {

        //Save the static global variable to ensure other test data is saved
        let mut old_str = String::new();
        unsafe {
            old_str = IO_BUFFER.clone();
        }
        let mut data = ProgramData::new(
            vec![String::from("let $a = 5"), String::from("output $a")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Execute next instruction
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Output $a
        data = result.0;
        result = result.1.execute(data).unwrap();

        let mut output_str = String::new();
        unsafe {
            output_str = IO_BUFFER.clone();
            IO_BUFFER = old_str;
        }
        assert_eq!("5", output_str);
    }

    #[test]
    fn output_str_variable() {

        //Hack to stop static global variable for being accessed by multiple tests simultaneously
        let sleep_time = rand::thread_rng().gen_range(100..500);
        thread::sleep(Duration::from_millis(sleep_time));

        let mut data = ProgramData::new(
            vec![String::from("let $a = \"meme\""), String::from("output $a")],
            HashMap::new(),
            LinkedList::new(),
            0
        );

        //Save the static global variable to ensure other test data is saved
        let mut old_str = String::new();
        unsafe {
            old_str = IO_BUFFER.clone();
        }
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Execute next instruction
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Output $a
        data = result.0;
        result = result.1.execute(data).unwrap();
        let mut output_str = String::new();
        unsafe {
            output_str = IO_BUFFER.clone();
            IO_BUFFER = old_str;
        }
        println!("{}", output_str);
        assert_eq!("meme", output_str);
    }

    #[test]
    fn goto_valid_block() {
        let mut data = ProgramData::new(
            vec![String::from("goto 2"), String::from("quit"), String::from("quit")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let res = GotoState{}.execute(data);
        assert_eq!(res.unwrap().0.get_index(), 2)
    }

    #[test]
    fn goto_invalid_block() {
        let mut data = ProgramData::new(
            vec![String::from("goto 4"), String::from("quit"), String::from("quit")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let res = GotoState{}.execute(data);
        assert_eq!(res.err().unwrap(), "Goto statement points to region out of bounds!\nAborting...")
    }

    #[test]
    fn if_tests_true() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 0"),
                String::from("let $b = 1"),
                String::from("if $a < $b goto 2"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.unwrap().0.get_index(), 2)
    }

    #[test]
    fn if_tests_false() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 1"),
                String::from("let $b = 0"),
                String::from("if $a < $b goto 2"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.unwrap().0.get_index(), 3)
    }

    #[test]
    fn math_add() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 1"),
                String::from("let $b = 2"),
                String::from("let $c = $a + $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should perform addition)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;

        assert_eq!(data.get_var(&String::from("c")).unwrap().as_str(), "3")
    }

    #[test]
    fn math_sub() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 1"),
                String::from("let $b = 1"),
                String::from("let $c = $a - $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should perform subtraction)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;

        assert_eq!(data.get_var(&String::from("c")).unwrap().as_str(), "0")
    }

    #[test]
    fn math_mult() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 4"),
                String::from("let $b = 2"),
                String::from("let $c = $a * $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should perform multiplication)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;

        assert_eq!(data.get_var(&String::from("c")).unwrap().as_str(), "8")
    }

    #[test]
    fn math_div() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 5"),
                String::from("let $b = 2"),
                String::from("let $c = $a / $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should perform division)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;

        assert_eq!(data.get_var(&String::from("c")).unwrap().as_str(), "2");
        assert_eq!(data.pop().unwrap().as_str(), "1")
    }

    #[test]
    fn math_lhs_not_number() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = \"me\""),
                String::from("let $b = 2"),
                String::from("let $c = $a / $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should return error)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data).err();
        assert_eq!(res.unwrap(), "$a is not a numeric value!\nAborting...")
    }

    #[test]
    fn math_rhs_not_number() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 2"),
                String::from("let $b = \"me\""),
                String::from("let $c = $a / $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $c (should return error)
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data).err();
        assert_eq!(res.unwrap(), "$b is not a numeric value!\nAborting...")
    }

    #[test]
    fn end_state_quits_program() {
        let mut data = ProgramData::new(
            vec![
                String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Quit program
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Exit");
        unsafe {
            assert_eq!(IS_EXIT, true)
        }
    }

    #[test]
    fn execute_invalid_instruction() {
        let mut data = ProgramData::new(
            vec![
                String::from("go to 0")
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Run invalid instruction
        let res = ExecuteState{}.execute(data);
        assert_eq!(res.err().unwrap(), "Unknown instruction: go to 0\nAborting...")
    }

    #[test]
    fn assign_from_invalid_variable() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = $b"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Try assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Variable $b does not exist!\nAborting...")
    }

    #[test]
    fn goto_invalid_goto() {
        let mut data = ProgramData::new(
            vec![
                String::from("goto e"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Try assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Invalid goto statement: goto e\nAborting...")
    }

    #[test]
    fn assign_invalid_assign_rhs() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = e"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Try assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Invalid assign instruction: let $a = e\nAborting...")
    }

    #[test]
    fn assign_invalid_assign_lhs() {
        let mut data = ProgramData::new(
            vec![
                String::from("let e = $a"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Try assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Invalid assign instruction: let e = $a\nAborting...")
    }

    #[test]
    fn if_invalid_lhs_variable() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 0"),
                String::from("if $a < $b goto 3"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.err().unwrap(), "Variable $b does not exist!\nAborting...")
    }

    #[test]
    fn if_invalid_rhs_variable() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $b = 0"),
                String::from("if $a < $b goto 3"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.err().unwrap(), "Variable $a does not exist!\nAborting...")
    }

    #[test]
    fn if_invalid_lhs_code() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $b = 0"),
                String::from("if e < $b goto 3"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.err().unwrap(), "Invalid if statement: if e < $b goto 3\nAborting...")
    }

    #[test]
    fn if_invalid_rhs_code() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 0"),
                String::from("if $a < e goto 3"),
                String::from("quit"), String::from("quit"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        let res = IfState{}.execute(result.0);
        assert_eq!(res.err().unwrap(), "Invalid if statement: if $a < e goto 3\nAborting...")
    }

    #[test]
    fn if_test_predicates() {
        let mut data = ProgramData::new(
            vec![
                String::from("let $a = 0"),
                String::from("let $b = 1"),
                String::from("if $a != $b goto 5"),
                String::from("if $a < $b goto 5"),
                String::from("if $a <= $b goto 5"),
                String::from("if $a > $b goto 5"),
                String::from("if $a >= $b goto 5"),
                String::from("if $a = $b goto 5"),
            ],
            HashMap::new(),
            LinkedList::new(),
            0
        );

        //Assign $a
        let mut result = ExecuteState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        //Assign $b
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();
        data = result.0;

        let mut res = IfState{}.execute(data).unwrap();
        data = res.0;
        assert_eq!(data.get_index(), 5);

        data.set_index(3);
        res = IfState{}.execute(data).unwrap();
        data = res.0;
        assert_eq!(data.get_index(), 5);

        data.set_index(3);
        res = IfState{}.execute(data).unwrap();
        data = res.0;
        assert_eq!(data.get_index(), 5);

        data.set_index(4);
        res = IfState{}.execute(data).unwrap();
        data = res.0;
        assert_eq!(data.get_index(), 5);

        data.set_index(5);
        res = IfState{}.execute(data).unwrap();
        data = res.0;
        assert_eq!(data.get_index(), 6);

        data.set_index(6);
        res = IfState{}.execute(data).unwrap();
        assert_eq!(res.0.get_index(), 7);
    }

    #[test]
    fn assign_reach_end_of_code() {
        let mut data = ProgramData::new(
            vec![String::from("let $a = 5")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let mut result = AssignState{}.execute(data).unwrap();
        data = result.0;
        result = result.1.execute(data).unwrap();

        data = result.0;
        let res = result.1.execute(data);

        assert_eq!(res.err().unwrap(), "Exit")
    }

    #[test]
    fn assign_from_stack() {
        let mut data = ProgramData::new(
            vec![String::from("let $a = pop")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        data.push(String::from("test"));
        let mut result = AssignState{}.execute(data).unwrap();
        data = result.0;
        assert_eq!(data.get_var(&String::from("a")).unwrap(), "test")
    }

    #[test]
    fn assign_empty_stack() {
        let mut data = ProgramData::new(
            vec![String::from("let $a = pop")],
            HashMap::new(),
            LinkedList::new(),
            0
        );
        let mut result = AssignState{}.execute(data);
        assert_eq!(result.err().unwrap(), "Stack is empty!\nAborting...")
    }
}