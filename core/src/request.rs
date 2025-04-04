pub struct SetRequest {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub struct GetRequest {
    pub key: Vec<u8>,
}

pub struct DeleteRequest {
    pub key: Vec<u8>,
}

pub struct ListRequest;

pub struct ExistsRequest {
    pub key: Vec<u8>,
}

pub struct CountRequest;

pub struct ClearRequest;
