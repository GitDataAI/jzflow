use anyhow::{anyhow, Result};
use uuid::Uuid;

pub trait IntoAnyhowResult<T> {
    fn anyhow(self, msg: impl ToString) -> Result<T>;
}

impl<T> IntoAnyhowResult<T> for Option<T> {
    fn anyhow(self, msg: impl ToString) -> Result<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(anyhow!(msg.to_string())),
        }
    }
}
