use regex::Regex;
use crate::errors::segment_errors::{error, VariableErrorCodes, CodeErrorCode, ErrorTypes, ErrorCodes, SegmentErrorTypes};

// fn load_code_from_file(file_name: String) -> Result<(Vec<String>, Vec<String>), Box<dyn ErrorCodes>>{
//     Err(Err)
// }

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
/// let var_array = load_variable_segment("0 1\n1 2")
/// let expected_array = vec![String::from("1"), String::from("2")];
/// assert_eq!(var_array.ok().unwrap(), expected_array);
/// ```
fn load_segment(segment_error_type: SegmentErrorTypes, variable_string: &str, split_regex: Regex) -> Result<Vec<String>, u32> {

    let mut memory_vec : Vec<String> = Vec::new();
    let mut err = error(&segment_error_type, ErrorTypes::AllOk).value();
    let mut variable_index = 0;

    if variable_string.len() == 0 {
        return Ok(memory_vec)
    }

    if !split_regex.is_match(variable_string) {
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
    let var_regex = Regex::new(r#"(\d+) (\w+)"#).unwrap();
    load_segment(SegmentErrorTypes::Variable, segment, var_regex)
}

#[cfg(test)]
mod test {
    use crate::code_loader::load_variable_segment;
    use crate::errors::segment_errors::{ErrorCodes, ErrorTypes, VariableErrorCodes};

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
}