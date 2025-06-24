// use std::fmt::Display;
//
// use crate::MAX_STATES;

#[derive(Debug)]
pub struct GenericError {
    message: String,
}

// impl GenericError {
//     pub fn new(message: String) -> Self {
//         Self { message }
//     }
// }

impl std::error::Error for GenericError {}

// Implement std::convert::From for AppError; from io::Error
impl From<std::io::Error> for GenericError {
    fn from(error: std::io::Error) -> Self {
        GenericError {
            message: error.to_string(),
        }
    }
}

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
