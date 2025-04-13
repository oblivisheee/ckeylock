use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    Error,
    NotFound,
    Unauthorized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    message: String,
    data: Option<ResponseData>,
    reqid: Vec<u8>,
}

impl Response {
    pub fn new(data: Option<ResponseData>, message: &str, reqid: Vec<u8>) -> Self {
        Self {
            message: message.to_string(),
            data,
            reqid,
        }
    }
    pub fn data(&self) -> Option<&ResponseData> {
        self.data.as_ref()
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn reqid(&self) -> Vec<u8> {
        self.reqid.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
    pub reqid: Vec<u8>,
}
impl ErrorResponse {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseData {
    SetResponse { key: Vec<u8> },
    GetResponse { value: Option<Vec<u8>> },
    DeleteResponse { key: Option<Vec<u8>> },
    ListResponse { keys: Vec<Vec<u8>> },
    ExistsResponse { exists: bool },
    CountResponse { count: usize },
    BatchGetResponse { values: Vec<Option<Vec<u8>>> },
    ClearResponse,
}
