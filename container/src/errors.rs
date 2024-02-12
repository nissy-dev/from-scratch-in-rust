use std::fmt;

#[derive(Debug)]
pub enum ErrorCode {
    ArgumentInvalid(&'static str),
}

impl ErrorCode {
    pub fn get_return_code(&self) -> i32 {
        1
    }
}

#[allow(unreachable_patterns)]
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            ErrorCode::ArgumentInvalid(element) => write!(f, "ArgumentInvalid: {}", element),
            _ => write!(f, "{:?}", self),
        }
    }
}

pub fn exit_with_return_code(res: Result<(), ErrorCode>) {
    match res {
        Ok(_) => {
            log::debug!("Exit without any error, returning 0");
            std::process::exit(0);
        }
        Err(e) => {
            let return_code = e.get_return_code();
            log::error!("Error on exit:\n\t{}\n\tReturning {}", e, return_code);
            std::process::exit(return_code);
        }
    }
}
