use std::str::FromStr;

use ckeylock_core::response::ErrorResponse;
use ckeylock_core::{Request, RequestWrapper, Response};
use futures_util::{SinkExt, StreamExt};
use std::cell::RefCell;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{ClientRequestBuilder, http::Uri, protocol::Message},
};

pub struct CKeyLockAPI {
    bind: String,
    password: Option<String>,
}
impl CKeyLockAPI {
    /// Creates a new instance of `CKeyLockAPI` with the given bind address.
    /// The bind address should be in the format "host:port".
    pub fn new(bind: &str, password: Option<&str>) -> Self {
        CKeyLockAPI {
            bind: bind.to_owned(),
            password: password.map(|p| p.to_owned()),
        }
    }

    pub async fn connect(&self) -> Result<CKeyLockConnection, Error> {
        let url = format!("ws://{}", self.bind);
        let request = match &self.password {
            Some(password) => ClientRequestBuilder::new(Uri::from_str(&url)?)
                .with_header("Authorization", password)
                .into_client_request()?,
            None => url.into_client_request()?,
        };
        let (ws_stream, _) = connect_async(request).await?;

        Ok(CKeyLockConnection {
            ws_stream: RefCell::new(ws_stream),
        })
    }
}

pub struct CKeyLockConnection {
    ws_stream: RefCell<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}
impl CKeyLockConnection {
    async fn send_request(&self, request: Request) -> Result<Response, Error> {
        let wrapper = RequestWrapper::new(request);
        let mut ws_stream = self.ws_stream.borrow_mut();
        ws_stream
            .send(request_into_message(wrapper.clone()))
            .await?;
        while let Some(msg) = ws_stream.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Ok(response) = serde_json::from_str::<Response>(&text) {
                    if response.reqid() == wrapper.id() {
                        return Ok(response);
                    }
                } else if let Ok(err_response) = serde_json::from_str::<ErrorResponse>(&text) {
                    if err_response.reqid == wrapper.id() {
                        return Err(Error::Custom(err_response.message));
                    }
                }
            }
        }
        Err(Error::Custom(
            "Response with matching ID not found".to_string(),
        ))
    }
    pub async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let res = self.send_request(Request::Set { key, value }).await?;
        if let Some(ckeylock_core::ResponseData::SetResponse { key }) = res.data() {
            Ok(key.to_vec())
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let res = self.send_request(Request::Get { key }).await?;
        if let Some(ckeylock_core::ResponseData::GetResponse { value }) = res.data() {
            Ok(value.as_ref().map(|v| v.to_vec()))
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn delete(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let res = self.send_request(Request::Delete { key }).await?;
        if let Some(ckeylock_core::ResponseData::DeleteResponse { key }) = res.data() {
            Ok(key.as_ref().map(|v| v.to_vec()))
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn list(&self) -> Result<Vec<Vec<u8>>, Error> {
        let res = self.send_request(Request::List).await?;
        if let Some(ckeylock_core::ResponseData::ListResponse { keys }) = res.data() {
            Ok(keys.clone())
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn exists(&self, key: Vec<u8>) -> Result<bool, Error> {
        let res = self.send_request(Request::Exists { key }).await?;
        if let Some(ckeylock_core::ResponseData::ExistsResponse { exists }) = res.data() {
            Ok(*exists)
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn count(&self) -> Result<usize, Error> {
        let res = self.send_request(Request::Count).await?;
        if let Some(ckeylock_core::ResponseData::CountResponse { count }) = res.data() {
            Ok(*count)
        } else {
            Err(Error::WrongResponseFormat)
        }
    }

    pub async fn clear(&self) -> Result<(), Error> {
        let res = self.send_request(Request::Clear).await?;
        if let Some(ckeylock_core::ResponseData::ClearResponse) = res.data() {
            Ok(())
        } else {
            Err(Error::WrongResponseFormat)
        }
    }
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut ws_stream = self.ws_stream.borrow_mut();
        Ok(ws_stream.close(None).await?)
    }
}

fn request_into_message(req: ckeylock_core::RequestWrapper) -> Message {
    Message::Text(req.to_string().into())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("WebSocket error: {0}")]
    WsError(#[from] WsError),
    #[error("Wrong response format")]
    WrongResponseFormat,
    #[error("Failed to parse uri: {0}")]
    UriParseError(#[from] tokio_tungstenite::tungstenite::http::uri::InvalidUri),
    #[error("{0}")]
    Custom(String),
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set() {
        let api = CKeyLockAPI::new(
            "127.0.0.1:8080",
            Some(
                "4f69e5532544b557dcee8dd077318f353af4ebcbed3280c8a9ceaa337ed45d283f9a7c9faebfb0c476a92e07a47c28bc603a93c74c090d41b3d625ea35902f35",
            ),
        );
        let mut connection = api.connect().await.unwrap();

        let key = b"popa".to_vec();
        let value = b"pisya".to_vec();

        let result = connection.set(key.clone(), value.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), key);
    }

    #[tokio::test]
    async fn test_get() {
        let api = CKeyLockAPI::new("127.0.0.1:8080", Some("helloworld"));
        let mut connection = api.connect().await.unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        connection.set(key.clone(), value.clone()).await.unwrap();
        let result = connection.get(key.clone()).await;
        assert!(result.is_ok());
        let unwrapped_value = result.unwrap();
        assert_eq!(unwrapped_value, Some(value));
        println!("Value: {:?}", unwrapped_value);
    }

    #[tokio::test]
    async fn test_delete() {
        let api = CKeyLockAPI::new("127.0.0.1:8080", Some("helloworld"));
        let mut connection = api.connect().await.unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        connection.set(key.clone(), value.clone()).await.unwrap();
        let result = connection.delete(key.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(key));
    }

    #[tokio::test]
    async fn test_list() {
        let api = CKeyLockAPI::new("127.0.0.1:8080", Some("helloworld"));
        let mut connection = api.connect().await.unwrap();

        let key1 = b"test_key1".to_vec();
        let key2 = b"test_key2".to_vec();
        let value = b"test_value".to_vec();

        connection.set(key1.clone(), value.clone()).await.unwrap();
        connection.set(key2.clone(), value.clone()).await.unwrap();

        let result = connection.list().await;
        assert!(result.is_ok());
        let keys = result.unwrap();
        assert!(keys.contains(&key1));
        assert!(keys.contains(&key2));
    }
    #[tokio::test]
    pub async fn req() {
        let api = CKeyLockAPI::new("127.0.0.1:8080", Some("helloworld"));
        let mut connection = api.connect().await.unwrap();
        let key = b"popa".to_vec();
        let res = connection.get(key.clone()).await.unwrap();
        println!("Response: {:?}", res.unwrap());
    }
}
