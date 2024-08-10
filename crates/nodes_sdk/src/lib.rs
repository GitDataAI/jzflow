#![feature(duration_constructors)]

pub mod fs_cache;
pub mod metadata;
pub mod mprc;
pub mod multi_sender;

mod sleep_program;

pub use sleep_program::*;

use anyhow::Result;
use tokio::sync::{
    mpsc::Sender,
    oneshot,
};

pub type MessageSender<REQ, RESP> = Sender<(REQ, oneshot::Sender<Result<RESP>>)>;
