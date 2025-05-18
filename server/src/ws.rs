use crate::{Error, executor::Executor};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::{
    handshake::server::{ErrorResponse, Request, Response},
    protocol::Message,
};
use tracing::{debug, error, info, warn};

pub struct WsServer;

impl WsServer {
    pub async fn new(
        bind: &str,
        password: Option<String>,
        executor: Arc<Executor>,
        concurrent_limit: Option<usize>,
    ) -> Result<Self, WsServerError> {
        info!("Starting WebSocket server on {}", bind);
        let listener = TcpListener::bind(bind).await?;
        while let Ok((stream, addr)) = listener.accept().await {
            info!("New connection from {}", addr);
            let password = password.clone();
            let executor = executor.clone();
            tokio::spawn(async move {
                let callback = |req: &Request,
                                mut res: Response|
                 -> Result<Response, ErrorResponse> {
                    debug!("Handling WebSocket handshake request");
                    if let Some(header_value) = req.headers().get("Authorization") {
                        let header_value = header_value.to_str().unwrap();
                        if let Some(password) = &password {
                            if header_value == password {
                                debug!("Authorization successful");
                                res.headers_mut()
                                    .insert("Authorization", header_value.parse().unwrap());
                            } else {
                                warn!("Authorization failed: invalid password");
                                res.headers_mut()
                                    .insert("WWW-Authenticate", "Basic".parse().unwrap());
                                res.headers_mut()
                                    .insert("401 Unauthorized", "Unauthorized".parse().unwrap());
                                return Err(ErrorResponse::new(Some(
                                    WsServerError::Unauthorized.to_string(),
                                )));
                            }
                        } else {
                            warn!("Authorization failed: password required but not provided");
                            res.headers_mut()
                                .insert("WWW-Authenticate", "Basic".parse().unwrap());
                            res.headers_mut()
                                .insert("401 Unauthorized", "Unauthorized".parse().unwrap());
                            return Err(ErrorResponse::new(Some(
                                WsServerError::Unauthorized.to_string(),
                            )));
                        }
                    } else {
                        if password.is_some() {
                            warn!("Authorization failed: missing Authorization header");
                            return Err(ErrorResponse::new(Some(
                                WsServerError::Unauthorized.to_string(),
                            )));
                        }
                    }
                    debug!("WebSocket handshake successful");
                    Ok(res)
                };
                match accept_hdr_async(stream, callback).await {
                    Ok(stream) => {
                        info!("WebSocket connection established");
                        let (write, read) = stream.split();
                        let write = Arc::new(tokio::sync::Mutex::new(write));
                        let executor = Arc::clone(&executor);

                        read.for_each_concurrent(concurrent_limit, {
                            let write = Arc::clone(&write);
                            let executor = Arc::clone(&executor);
                            move |msg| {
                                let write = Arc::clone(&write);
                                let executor = Arc::clone(&executor);
                                async move {
                                    let message = match msg {
                                        Ok(m) => m,
                                        Err(e) => {
                                            error!("WebSocket error: {:?}", e);
                                            return;
                                        }
                                    };
                                    match message {
                                        Message::Text(text) => {
                                            debug!("Received text message.");
                                            let request = match serde_json::from_str::<
                                                ckeylock_core::RequestWrapper,
                                            >(
                                                &text
                                            ) {
                                                Ok(request) => request,
                                                Err(e) => {
                                                    error!("Failed to parse request: {:?}", e);
                                                    let mut write = write.lock().await;
                                                    if let Err(e) = write
                                                        .send(Message::Text(e.to_string().into()))
                                                        .await
                                                    {
                                                        error!(
                                                            "Failed to send error response: {:?}",
                                                            e
                                                        );
                                                    }
                                                    return;
                                                }
                                            };
                                            let response = executor.execute(request.clone()).await;
                                            let mut write = write.lock().await;
                                            match response {
                                                Ok(response) => {
                                                    debug!("Request executed successfully");
                                                    if let Err(e) = write
                                                        .send(response_into_message(response))
                                                        .await
                                                    {
                                                        error!("Failed to send response: {:?}", e);
                                                    }
                                                }
                                                Err(e) => {
                                                    error!("Request execution failed: {:?}", e);
                                                    if let Err(e) = write
                                                        .send(error_into_message(e, request.id()))
                                                        .await
                                                    {
                                                        error!(
                                                            "Failed to send error response: {:?}",
                                                            e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Message::Ping(ping) => {
                                            debug!("Received ping, sending pong");
                                            let mut write = write.lock().await;
                                            if let Err(e) = write.send(Message::Pong(ping)).await {
                                                error!("Failed to send pong: {:?}", e);
                                            }
                                        }
                                        Message::Close(close) => {
                                            debug!("Received close message: {:?}", close);
                                            let mut write = write.lock().await;
                                            if let Err(e) = write.send(Message::Close(close)).await
                                            {
                                                error!("Failed to send close message: {:?}", e);
                                            }
                                        }
                                        _ => {
                                            debug!("Received unsupported message type");
                                        }
                                    }
                                }
                            }
                        })
                        .await;
                    }
                    Err(e) => {
                        error!("Error during WebSocket handshake: {:?}", e);
                    }
                }
            });
        }
        Ok(Self)
    }
}

fn response_into_message(res: ckeylock_core::Response) -> Message {
    Message::Text(res.to_string().into())
}
fn error_into_message(err: Error, reqid: Vec<u8>) -> Message {
    Message::Text(
        ckeylock_core::response::ErrorResponse {
            message: err.to_string(),
            reqid,
        }
        .to_string()
        .into(),
    )
}

#[derive(Debug, thiserror::Error)]
pub enum WsServerError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
