use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::marker::Send;

pub trait GID = Eq + Clone + Copy + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>;
