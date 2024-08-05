use serde::Serialize;

use crate::core::db::DBConfig;

#[derive(Clone, Serialize)]
pub struct MongoConfig {
    pub mongo_url: String,
}

impl MongoConfig {
    pub fn new(mongo_url: String) -> Self {
        MongoConfig { mongo_url }
    }
}

impl DBConfig for MongoConfig {
    fn connection_string(&self) -> &str {
        return &self.mongo_url;
    }
}
