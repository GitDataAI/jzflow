pub mod k8s_helper;
pub mod sizefmt;

use core::fmt;

use anyhow::{
    anyhow,
    Result,
};
use tonic::{
    Code,
    Status,
};

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

pub trait StdIntoAnyhowResult<T> {
    fn anyhow(self) -> Result<T>;
}

impl<R, E> StdIntoAnyhowResult<R> for std::result::Result<R, E>
where
    E: fmt::Display,
{
    fn anyhow(self) -> Result<R> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!("{err}")),
        }
    }
}

pub trait AnyhowToGrpc<R> {
    fn to_rpc(self, code: Code) -> std::result::Result<R, Status>;
}

impl<R> AnyhowToGrpc<R> for Result<R> {
    fn to_rpc(self, code: Code) -> std::result::Result<R, Status> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Status::new(code, err.to_string())),
        }
    }
}
