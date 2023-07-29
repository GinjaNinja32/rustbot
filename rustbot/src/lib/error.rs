use std::borrow::Cow;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug)]
pub struct UserError {
    msg: Cow<'static, str>,
}
impl UserError {
    pub fn new<T: Into<Cow<'static, str>>>(msg: T) -> Self {
        Self { msg: msg.into() }
    }
}
impl std::error::Error for UserError {}
impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.msg)
    }
}
