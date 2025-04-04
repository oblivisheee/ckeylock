// filepath: /Users/oblivisheee/Documents/Projects/ckeylock/core/src/response.rs
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    Error,
    NotFound,
    Unauthorized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T: ResponseTrait> {
    pub status: ResponseStatus,
    pub message: String,
    pub data: Option<T>,
}

impl<T: ResponseTrait> Response<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            status: ResponseStatus::Success,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: ResponseStatus::Error,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            status: ResponseStatus::NotFound,
            message: message.to_string(),
            data: None,
        }
    }
}

pub struct SetResponse {
    pub key: Vec<u8>,
}

pub struct GetResponse {
    pub value: Vec<u8>,
}

pub struct DeleteResponse {
    pub key: Vec<u8>,
}
pub struct ListResponse {
    pub keys: Vec<Vec<u8>>,
}
pub struct ExistsResponse {
    pub exists: bool,
}
pub struct CountResponse {
    pub count: usize,
}
pub struct ClearResponse;
pub struct PingResponse;

pub trait ResponseTrait {}
