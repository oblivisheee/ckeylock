use std::str::FromStr;

use ckeylock_core::response::ErrorResponse;
use ckeylock_core::{Request, RequestWrapper, Response};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
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
                .into_client_request()
                .map_err(|e| Error::Custom(format!("Failed to build client request: {}", e)))?,
            None => url
                .into_client_request()
                .map_err(|e| Error::Custom(format!("Failed to build client request: {}", e)))?,
        };
        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| Error::Custom(format!("Failed to connect to WebSocket: {}", e)))?;

        Ok(CKeyLockConnection {
            inner: CkeyLockConnectionInner::new(ws_stream).into(),
        })
    }
}

pub struct CKeyLockConnection {
    inner: Arc<CkeyLockConnectionInner>,
}

impl CKeyLockConnection {
    async fn send_request(&self, request: Request) -> Result<Response, Error> {
        let request = RequestWrapper::new(request);

        self.inner
            .send(request_into_message(request.clone()))
            .await?;

        while let Some(msg) = self.inner.lock().await.next().await {
            let msg =
                msg.map_err(|e| Error::Custom(format!("Failed to receive message: {}", e)))?;
            if let Some(parsed_response) = self.parse_response(&msg, request.id()) {
                return parsed_response;
            }
        }
        Err(Error::Custom(
            "Response with matching ID not found".to_string(),
        ))
    }

    fn parse_response(&self, msg: &Message, req_id: Vec<u8>) -> Option<Result<Response, Error>> {
        if let Message::Text(text) = msg {
            if let Ok(response) = serde_json::from_str::<Response>(text) {
                if response.reqid() == req_id {
                    return Some(Ok(response));
                }
            } else if let Ok(err_response) = serde_json::from_str::<ErrorResponse>(text) {
                if err_response.reqid == req_id {
                    return Some(Err(Error::Custom(format!(
                        "Error response received: {}",
                        err_response.message
                    ))));
                }
            }
        }
        None
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
        self.inner
            .lock()
            .await
            .close(None)
            .await
            .map_err(|e| Box::new(Error::Custom(format!("Failed to close WebSocket: {}", e))) as _)
    }
}

fn request_into_message(req: ckeylock_core::RequestWrapper) -> Message {
    Message::Text(req.to_string().into())
}

pub struct CkeyLockConnectionInner(Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>);

impl CkeyLockConnectionInner {
    pub fn new(ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        CkeyLockConnectionInner(Mutex::new(ws_stream))
    }

    pub async fn send(&self, msg: Message) -> Result<(), Error> {
        self.0
            .lock()
            .await
            .send(msg)
            .await
            .map_err(|e| Error::Custom(format!("Failed to send message: {}", e)))
    }
    pub async fn lock(
        &self,
    ) -> tokio::sync::MutexGuard<'_, WebSocketStream<MaybeTlsStream<TcpStream>>> {
        self.0.lock().await
    }
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
        let api = CKeyLockAPI::new("127.0.0.1:5830", Some("helloworld"));
        let connection = api.connect().await.unwrap();

        let key = b"popa".to_vec();
        let value = b"pizdec".to_vec();

        let result = connection.set(key.clone(), value.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), key);
    }

    #[tokio::test]
    async fn test_get() {
        let api = CKeyLockAPI::new("127.0.0.1:5830", Some("helloworld"));
        let connection = api.connect().await.unwrap();

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
        let api = CKeyLockAPI::new("127.0.0.1:5830", Some("helloworld"));
        let connection = api.connect().await.unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        connection.set(key.clone(), value.clone()).await.unwrap();
        let result = connection.delete(key.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(key));
    }

    #[tokio::test]
    async fn test_list() {
        let api = CKeyLockAPI::new("127.0.0.1:5830", Some("helloworld"));
        let connection = api.connect().await.unwrap();

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
}
