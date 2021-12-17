use std::fs;
use regex::Regex;
use crate::errors::segment_errors::{error, ERROR_MESSAGES, ErrorTypes, SegmentErrorTypes};

/// Loads COS341Basic data from a file and creates two vectors, one for the register data and
/// the other for the code data. If an error is encountered while loading program data, a
/// message detailing the error is returned.
///
/// # Arguments
/// * `file_path` - Path of the file to load
///
/// # Returns
/// * `Ok((Vec<String>, Vec<String>))` - a tuple containing the register and code vectors
/// * `Err(String)` - a message detailing any error that occurred while loading the program
pub fn load_code_from_file(file_path: String) -> Result<Vec<String>, String>{
    let file_data = fs::read_to_string(file_path.clone());
    match file_data {
        Ok(file_string) => {
            let code_vec = if file_string.len() > 0 {
                let code_segment = load_code_segment(file_string.as_str());

                if code_segment.is_err() {
                    return Err(ERROR_MESSAGES[code_segment.err().unwrap() as usize].parse().unwrap());
                }
                code_segment.unwrap()
            } else {
                Vec::new()
            };

            Ok(code_vec)
        },
        Err(msg) => {
            Err(format!("{}: {}", file_path, msg).to_string())
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

    //return empty array if no registers are declared
    if variable_string.len() == 0 {
        return Ok(memory_vec)
    }

    //Register segment was not declared correctly
    if !split_regex.is_match(variable_string.trim()) {
        return Err(error(&segment_error_type, ErrorTypes::MalformedSegment).value());
    }

    //split the string by lines
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

///Uses the load_segment function to load code data into memory.
/// # Arguments
///  * - `segment` - String slice containing code data
/// # Returns
/// * Ok(Vec<String>) - An array containing code data for the interpreter to execute.
/// * Err(u32) - An error code. This happens when there was an error parsing the code string.
fn load_code_segment(segment: &str) -> Result<Vec<String>, u32> {
    let var_regex = Regex::new(r#"^(\d+) (.+)"#).unwrap();
    load_segment(SegmentErrorTypes::Code, segment, var_regex)
}

#[cfg(test)]
mod test {
    use crate::code_loader::load_code_segment;
    use crate::errors::segment_errors::{CodeErrorCode, ErrorCodes, ErrorTypes, VariableErrorCodes};
    use super::*;

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

        assert_eq!(result.ok().unwrap(), vec![String::from("quit")]);
    }

    #[test]
    fn load_large_test_file() {
        let code_vec = vec![
            String::from("let $a = 5"),
            String::from("let $b = 5"),
            String::from("let $c = 5"),
            String::from("let $d = 5"),
            String::from("let $e = 5"),
            String::from("if $a < $b goto 7"),
            String::from("goto 9"),
            String::from("output $c"),
            String::from("output $d"),
            String::from("quit"),
            String::from("let $e = $e + $a"),
            String::from("output $e"),
            String::from("quit"),
        ];

        let result = load_code_from_file("testfiles/test2.txt".to_string());

        assert_eq!(result.ok().unwrap(), code_vec);
    }

    #[test]
    fn load_code_with_code_error() {
        let result = load_code_from_file("testfiles/test3.txt".to_string());

        assert_eq!(result.as_ref().err().unwrap(), ERROR_MESSAGES[8]);
    }

    #[test]
    fn load_empty_file() {
        let result = load_code_from_file("testfiles/test4.txt".to_string());

        let test: Vec<String> = Vec::new();

        assert_eq!(result.ok().unwrap(), test);
    }
}
