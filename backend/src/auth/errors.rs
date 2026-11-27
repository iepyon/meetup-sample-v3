#[derive(Debug)]
pub enum RegisterError {
    DuplicateEmail { email: String },
    Internal,
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    Internal,
}
