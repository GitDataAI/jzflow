/*
 * jiaozifs API
 *
 * jiaozifs HTTP API
 *
 * The version of the OpenAPI document: 1.0.0
 *
 * Generated by: https://openapi-generator.tech
 */

use crate::models;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreeNode {
    #[serde(rename = "hash")]
    pub hash: String,
    #[serde(rename = "repository_id")]
    pub repository_id: uuid::Uuid,
    #[serde(rename = "type")]
    pub r#type: i32,
    #[serde(rename = "properties")]
    pub properties: std::collections::HashMap<String, String>,
    #[serde(rename = "sub_objects")]
    pub sub_objects: Vec<models::TreeEntry>,
    #[serde(rename = "created_at")]
    pub created_at: i64,
    #[serde(rename = "updated_at")]
    pub updated_at: i64,
}

impl TreeNode {
    pub fn new(
        hash: String,
        repository_id: uuid::Uuid,
        r#type: i32,
        properties: std::collections::HashMap<String, String>,
        sub_objects: Vec<models::TreeEntry>,
        created_at: i64,
        updated_at: i64,
    ) -> TreeNode {
        TreeNode {
            hash,
            repository_id,
            r#type,
            properties,
            sub_objects,
            created_at,
            updated_at,
        }
    }
}