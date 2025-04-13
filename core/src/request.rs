use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Set { key: Vec<u8>, value: Vec<u8> },
    Get { key: Vec<u8> },
    Delete { key: Vec<u8> },
    List,
    Exists { key: Vec<u8> },
    Count,
    BatchGet { keys: Vec<Vec<u8>> },
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestWrapper {
    req: Request,
    id: Vec<u8>,
}

impl RequestWrapper {
    pub fn new(req: Request) -> Self {
        Self {
            req,
            id: uuid::Uuid::new_v4().as_bytes().to_vec(),
        }
    }
    pub fn id(&self) -> Vec<u8> {
        self.id.clone()
    }
    pub fn req(&self) -> &Request {
        &self.req
    }
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
