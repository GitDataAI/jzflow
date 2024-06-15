use serde::{Deserialize, Serialize};
use std::default::Default;
use std::hash::Hash;
use std::marker::Send;

pub trait GID =
    Eq + Clone + Default + Copy + Hash + Send + Sync + Serialize + for<'de> Deserialize<'de>;
