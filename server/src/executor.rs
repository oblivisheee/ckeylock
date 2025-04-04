use crate::{Error, storage::Storage};
use ckeylock_core::{Request, Response, ResponseData, request::RequestWrapper};
use serde_json::value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;
pub struct Executor {
    command_tx: mpsc::Sender<ExecutorCommands>,
}

impl Executor {
    pub async fn new(storage: Storage) -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut storage = storage;
            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        match cmd{
                            ExecutorCommands::Set { key, value, respond_to } => {
                                let result = storage.set(key, value);
                                if let Err(e) = respond_to.send(result.map_err(|e| e.into())){
                                    error!("Failed to send set response: {:?}", e);
                                }
                            }
                            ExecutorCommands::Get { key, response } => {
                                let result = storage.get(key);
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                    error!("Failed to send get response: {:?}", e);
                                }
                            }
                            ExecutorCommands::Delete { key, response } => {
                                let result = storage.delete(key);
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                    error!("Failed to send delete response: {:?}", e);
                                }
                            }
                            ExecutorCommands::List { response } => {
                                let result = storage.list();
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                    error!("Failed to send list response: {:?}", e);
                                }
                            }
                            ExecutorCommands::Exists { key, response } => {
                                let result = storage.exists(key);
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                    error!("Failed to send exists response: {:?}", e);
                                }
                            }
                            ExecutorCommands::Count { response } => {
                                let result = storage.count();
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                    error!("Failed to send count response: {:?}", e);
                                }
                            }
                            ExecutorCommands::Clear { response } => {
                                let result = storage.clear();
                                if let Err(e) = response.send(result.map_err(|e| e.into())){
                                 error!("Failed to send clear response: {:?}", e);

                                }
                            }
                        }
                    }
                }
            }
        });
        Arc::new(Self { command_tx: tx })
    }

    pub async fn execute(&self, request: RequestWrapper) -> Result<Response, Error> {
        let original_request = request.req().clone();
        match original_request {
            Request::Set { key, value } => {
                let result = self.set(key, value).await?;
                Ok(Response::new(
                    Some(ResponseData::SetResponse { key: result }),
                    "Stored successfully.",
                    request.id(),
                ))
            }
            Request::Get { key } => {
                let value = self.get(key).await?;
                Ok(Response::new(
                    Some(ResponseData::GetResponse { value }),
                    "Retrieved successfully.",
                    request.id(),
                ))
            }
            Request::Delete { key } => {
                let key = self.delete(key.clone()).await?;
                Ok(Response::new(
                    Some(ResponseData::DeleteResponse { key }),
                    "Deleted successfully.",
                    request.id(),
                ))
            }
            Request::List => {
                let result = self.list().await?;
                Ok(Response::new(
                    Some(ResponseData::ListResponse { keys: result }),
                    "Listed successfully.",
                    request.id(),
                ))
            }
            Request::Exists { key } => {
                let result = self.exists(key).await?;
                Ok(Response::new(
                    Some(ResponseData::ExistsResponse { exists: result }),
                    "Existence checked successfully.",
                    request.id(),
                ))
            }
            Request::Count => {
                let value = self.count().await?;
                Ok(Response::new(
                    Some(ResponseData::CountResponse { count: value }),
                    "Counted successfully.",
                    request.id(),
                ))
            }
            Request::Clear => {
                let result = self.clear().await;
                Ok(Response::new(
                    Some(ResponseData::ClearResponse),
                    "Cleared successfully.",
                    request.id(),
                ))
            }
        }
    }
    pub async fn set(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Set {
                key,
                value,
                respond_to: tx,
            })
            .await?;
        rx.await?
    }
    pub async fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Get { key, response: tx })
            .await?;
        rx.await?
    }
    pub async fn delete(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Delete { key, response: tx })
            .await?;
        rx.await?
    }
    pub async fn list(&self) -> Result<Vec<Vec<u8>>, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::List { response: tx })
            .await?;
        rx.await?
    }
    pub async fn exists(&self, key: Vec<u8>) -> Result<bool, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Exists { key, response: tx })
            .await?;
        rx.await?
    }
    pub async fn count(&self) -> Result<usize, Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Count { response: tx })
            .await?;
        rx.await?
    }
    pub async fn clear(&self) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        self.command_tx
            .send(ExecutorCommands::Clear { response: tx })
            .await?;
        rx.await?
    }
}
pub enum ExecutorCommands {
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
        respond_to: oneshot::Sender<Result<Vec<u8>, Error>>,
    },
    Get {
        key: Vec<u8>,
        response: oneshot::Sender<Result<Option<Vec<u8>>, Error>>,
    },
    Delete {
        key: Vec<u8>,
        response: oneshot::Sender<Result<Option<Vec<u8>>, Error>>,
    },
    List {
        response: oneshot::Sender<Result<Vec<Vec<u8>>, Error>>,
    },
    Exists {
        key: Vec<u8>,
        response: oneshot::Sender<Result<bool, Error>>,
    },
    Count {
        response: oneshot::Sender<Result<usize, Error>>,
    },
    Clear {
        response: oneshot::Sender<Result<(), Error>>,
    },
}
