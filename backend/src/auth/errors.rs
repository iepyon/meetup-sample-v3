#[derive(Debug)]
pub enum RegisterError {
    DuplicateEmail,
    Internal,
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    Internal,
}
