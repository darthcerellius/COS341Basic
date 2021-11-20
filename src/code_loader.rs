use std::fs;
use regex::Regex;
use crate::errors::segment_errors::{error, ErrorTypes, SegmentErrorTypes};

pub fn load_code_from_file(file_name: String) -> Result<(Vec<String>, Vec<String>), String>{
    let file_data = fs::read_to_string(file_name.clone());
    match file_data {
        Ok(file_string) => {
            const BEGIN_REGISTER_SEGMENT: &str = "BEGIN_REGISTER_SEGMENT\n";
            const END_REGISTER_SEGMENT: &str = "\nEND_REGISTER_SEGMENT";
            const BEGIN_CODE_SEGMENT: &str = "BEGIN_CODE_SEGMENT\n";
            const END_CODE_SEGMENT: &str = "\nEND_CODE_SEGMENT";

            let reg_start_pos = file_string.find(BEGIN_REGISTER_SEGMENT);
            let reg_end_pos = file_string.find(END_REGISTER_SEGMENT);
            let code_start_pos = file_string.find(BEGIN_CODE_SEGMENT);
            let code_end_pos = file_string.find(END_CODE_SEGMENT);

            //Ensure that the code and register segments are present
            if reg_start_pos.is_none() {
                return Err("No register segment found! Aborting...".to_string());
            }
            if reg_end_pos.is_none() {
                return Err("Register segment not fully defined! Aborting...".to_string());
            }
            if code_start_pos.is_none() {
                return Err("No code segment found! Aborting...".to_string());
            }
            if code_end_pos.is_none() {
                return Err("Code segment not fully defined! Aborting...".to_string());
            }

            let mut s_pos = reg_start_pos.unwrap() + BEGIN_REGISTER_SEGMENT.len();
            let mut e_pos = reg_end_pos.unwrap();

            let register_segment = load_variable_segment(file_string[s_pos..e_pos].as_ref());

            if register_segment.is_err() {
                return Err(register_segment.err().unwrap().to_string());
            }

            s_pos = code_start_pos.unwrap() + BEGIN_CODE_SEGMENT.len();
            e_pos = code_end_pos.unwrap();

            let code_segment = load_code_segment(file_string[s_pos..e_pos].as_ref());

            if code_segment.is_err() {
                return Err(code_segment.err().unwrap().to_string());
            }

            Ok((register_segment.unwrap(), code_segment.unwrap()))
        },
        Err(msg) => {
            Err(format!("{}: {}", file_name, msg).to_string())
        }
    }
}

/// Parses a variable string using a provided Regex, extracts the data from the string and returns a Vec
/// containing the data in a 1:1 mapping according to the index of the data in the string
///
/// # Arguments
/// * `segment_error_type` - Tells the function which segment type error codes the function returns should
///                          the parser encounter any error.
/// * `variable_string` - A string that holds variable data in the format 'index value'. Each variable
///                       in this string is separated by '\n' or '\r\n'.
/// # Returns
/// * `Ok(Vec<String>)` - An array holding the declared values.
/// * `Err(u32)` - An error code. This happens when there was an error parsing the variable string.
///
/// # Examples
///
/// ```
/// let expected_result = VariableErrorCodes{
///             error: ErrorTypes::MalformedAssignment
/// };
/// let result = load_segment(SegmentErrorTypes::Variable, segment, split_regex);
/// assert_eq!(result.err().unwrap(), expected_result.value())
/// ```
fn load_segment(segment_error_type: SegmentErrorTypes, variable_string: &str, split_regex: Regex) -> Result<Vec<String>, u32> {

    let mut memory_vec : Vec<String> = Vec::new();
    let mut err = error(&segment_error_type, ErrorTypes::AllOk).value();
    let mut variable_index = 0;

    if variable_string.len() == 0 {
        return Ok(memory_vec)
    }

    if !split_regex.is_match(variable_string.trim()) {
        return Err(error(&segment_error_type, ErrorTypes::MalformedSegment).value());
    }

    let variables = Regex::new(r"(\n|\r\n)").unwrap()
        .split(variable_string)
        .collect::<Vec<&str>>();

    for var in variables {

        if var.len() == 0 {
            continue;
        }

        let item = split_regex.captures(var);

        // ensure that variables have a 'index value' structure
        if item.is_none() {
            err = error(&segment_error_type, ErrorTypes::MalformedAssignment).value();
            break;
        }

        let pos = (&item).as_ref().unwrap().get(1).unwrap();
        let val = item.unwrap().get(2).unwrap();

        /*
        Ensure that the variable indices are created in chronological order, starting from 0.
        These indices have a 1:1 mapping in the resulting array.
         */
        if pos.as_str().parse::<usize>().unwrap() != variable_index {
            err = error(&segment_error_type, ErrorTypes::NotChronological).value();
            break;
        }
        variable_index += 1;
        memory_vec.push(val.as_str().to_string())
    }

    //return the vector if there was no error, otherwise return the error
    match err {
        0 => Ok(memory_vec),
        _ => Err(err)
    }
}

fn load_variable_segment(segment: &str) -> Result<Vec<String>, u32> {
    let var_regex = Regex::new(r#"^(\d+) (\w+)"#).unwrap();
    load_segment(SegmentErrorTypes::Variable, segment, var_regex)
}

fn load_code_segment(segment: &str) -> Result<Vec<String>, u32> {
    let var_regex = Regex::new(r#"^(\d+) (.+)"#).unwrap();
    load_segment(SegmentErrorTypes::Code, segment, var_regex)
}

#[cfg(test)]
mod test {
    use crate::code_loader::{load_code_segment, load_variable_segment};
    use crate::errors::segment_errors::{CodeErrorCode, ErrorCodes, ErrorTypes, VariableErrorCodes};
    use super::*;

    #[test]
    fn test_malformed_variable_segment() {
        let result = load_variable_segment("1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::MalformedSegment
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_malformed_assignment() {
        let result = load_variable_segment("0 0\n1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::MalformedAssignment
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_memory_array_one_item() {
        let result = load_variable_segment("0 5\n");
        assert_eq!(result.ok().unwrap(), vec![String::from("5")])
    }

    #[test]
    fn test_memory_array_many_items() {
        let item_string = "0 5\n1 6\n2 hello\n3 0";
        let expected_array = item_string.split("\n").
            map(|x| x.split(" ").collect::<Vec<&str>>()[1].to_string())
            .collect::<Vec<String>>();
        let result = load_variable_segment(item_string);
        assert_eq!(result.ok().unwrap(), expected_array)
    }

    #[test]
    fn test_out_of_chronological_order_one_item() {
        let result = load_variable_segment("1 1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::NotChronological
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_out_of_chronological_order_first_item() {
        let result = load_variable_segment("1 0\n2 1\n3 1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::NotChronological
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_out_of_chronological_order_middle_item() {
        let result = load_variable_segment("0 0\n2 1\n3 1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::NotChronological
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_out_of_chronological_order_last_item() {
        let result = load_variable_segment("0 0\n1 1\n3 1");
        let expected_result = VariableErrorCodes{
            error: ErrorTypes::NotChronological
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn test_with_empty_string() {
        let result = load_variable_segment("");
        let expected_vector:Vec<String> = Vec::new();
        assert_eq!(result.ok().unwrap(), expected_vector)
    }

    #[test]
    fn test_with_multiple_newlines() {
        let result = load_variable_segment("\n\r\n\n0 5\n\n\n1 2");
        assert_eq!(result.ok().unwrap(), vec![String::from("5"), String::from("2")])
    }

    #[test]
    fn test_loading_code() {
        let result = load_code_segment("0 let M0 = 3");
        assert_eq!(result.ok().unwrap(), vec![String::from("let M0 = 3")])
    }

    #[test]
    fn test_missing_space_between_index_and_code() {
        let result = load_code_segment("0let M0 = 3");
        let expected_result = CodeErrorCode{
            error: ErrorTypes::MalformedSegment
        };
        assert_eq!(result.err().unwrap(), expected_result.value())
    }

    #[test]
    fn get_error_from_file_not_found() {
        let result = load_code_from_file("notfound.txt".to_string());

        let error_string = "notfound.txt: No such file or directory (os error 2)";

        assert_eq!(result.err().unwrap(), error_string)
    }

    #[test]
    fn load_small_test_file() {

        let result = load_code_from_file("testfiles/test1.txt".to_string());

        assert_eq!(result.as_ref().ok().unwrap().0, vec!["5"]);
        assert_eq!(result.as_ref().ok().unwrap().1, vec!["quit"]);
    }

    #[test]
    fn load_large_test_file() {
        let reg_vec = vec![
            String::from("0"),
            String::from("5"),
            String::from("10"),
            String::from("hello"),
            String::from("world"),
            String::from("66"),
        ];
        let code_vec = vec![
            String::from("let M0 = 5"),
            String::from("if M0 < M1 goto C3"),
            String::from("goto C6"),
            String::from("output M3"),
            String::from("output M4"),
            String::from("quit"),
            String::from("let M5 = M5 + M0"),
            String::from("output M5"),
            String::from("quit"),
        ];

        let result = load_code_from_file("testfiles/test2.txt".to_string());

        assert_eq!(result.as_ref().ok().unwrap().0, reg_vec);
        assert_eq!(result.as_ref().ok().unwrap().1, code_vec);
    }

    #[test]
    fn load_code_with_missing_segments() {
        let mut result = load_code_from_file("testfiles/test3.txt".to_string());

        assert_eq!(result.as_ref().err().unwrap(), "No register segment found! Aborting...");

        result = load_code_from_file("testfiles/test4.txt".to_string());

        assert_eq!(result.as_ref().err().unwrap(), "No code segment found! Aborting...");
    }

    #[test]
    fn load_code_with_swapped_segment() {

        let result = load_code_from_file("testfiles/test5.txt".to_string());

        assert_eq!(result.as_ref().ok().unwrap().0, vec!["5"]);
        assert_eq!(result.as_ref().ok().unwrap().1, vec!["quit"]);
    }

    #[test]
    fn load_code_with_register_error() {

        let result = load_code_from_file("testfiles/test6.txt".to_string());

        assert_eq!(result.as_ref().err().unwrap(), "5");
    }

    #[test]
    fn load_code_with_code_error() {
        let result = load_code_from_file("testfiles/test7.txt".to_string());

        assert_eq!(result.as_ref().err().unwrap(), "8");
    }
}