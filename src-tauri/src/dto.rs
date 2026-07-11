use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct AuthSuccess {
    pub username: String,
}
