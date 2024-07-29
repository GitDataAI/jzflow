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

/// S3AuthInfo : S3AuthInfo holds S3-style authentication.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct S3AuthInfo {
    #[serde(rename = "credentials")]
    pub credentials: Box<models::Credential>,
}

impl S3AuthInfo {
    /// S3AuthInfo holds S3-style authentication.
    pub fn new(credentials: models::Credential) -> S3AuthInfo {
        S3AuthInfo {
            credentials: Box::new(credentials),
        }
    }
}