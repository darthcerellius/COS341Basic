pub mod segment_errors {
    use lazy_static::lazy_static;
    lazy_static! {
    pub static ref ERROR_MESSAGES: [&'static str; 9] = [
            "Ok",
            "No register segment found!",
            "No code segment found!",
            "Malformed register assignment found!",
            "Malformed code assignment found!",
            "Register declaration not in chronological order!",
            "Code lines are not in chronological order!",
            "Malformed register segment found!",
            "Malformed code segment found!",
    ];
}

    pub enum ErrorTypes {
        NoSegment,
        AllOk,
        MalformedAssignment,
        NotChronological,
        MalformedSegment
    }

    pub enum SegmentErrorTypes {
        Variable,
        Code
    }

    pub trait ErrorCodes {
        fn value(&self) -> u32;
    }

    pub struct VariableErrorCodes{
        pub(crate) error: ErrorTypes
    }
    pub struct CodeErrorCode{
        pub(crate) error: ErrorTypes
    }

    impl ErrorCodes for VariableErrorCodes {
        fn value(&self) -> u32 {

            // Error codes for this type are all odd numbers except for the AllOk type
            match (*self).error {
                ErrorTypes::NoSegment => 1,
                ErrorTypes::AllOk => 0,
                ErrorTypes::MalformedAssignment => 3,
                ErrorTypes::NotChronological => 5,
                ErrorTypes::MalformedSegment => 7
            }
        }
    }

    impl ErrorCodes for CodeErrorCode {
        fn value(&self) -> u32 {
            match (*self).error {
                ErrorTypes::NoSegment => 2,
                ErrorTypes::AllOk => 0,
                ErrorTypes::MalformedAssignment => 4,
                ErrorTypes::NotChronological => 6,
                ErrorTypes::MalformedSegment => 8
            }
        }
    }

    pub fn error(seg: &SegmentErrorTypes, code: ErrorTypes) -> Box<dyn ErrorCodes> {
        match seg {
            SegmentErrorTypes::Variable => Box::new(VariableErrorCodes{
                error: code
            }),
            SegmentErrorTypes::Code => Box::new(CodeErrorCode {
                error:code
            })
        }
    }
}
