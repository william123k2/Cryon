use std::fmt::Display;
pub type Result<T> = std::result::Result<T, String>;
pub trait Context<T> {
    fn ctx(self, message: &str) -> Result<T>;
}
impl<T, E: Display> Context<T> for std::result::Result<T, E> {
    fn ctx(self, message: &str) -> Result<T> {
        self.map_err(|e| format!("{message}: {e}"))
    }
}
