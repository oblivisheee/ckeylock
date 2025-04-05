mod conf;
mod crypto;
mod executor;
mod storage;
mod ws;

use clap::Parser;
use conf::Config;
use crypto::hash;
use storage::Storage;
use ws::WsServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = CKEYLOCK_CONFIG_PATH)]
    pub config: String,
}

const CKEYLOCK_CONFIG_PATH: &str = "Ckeylock.toml";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_level(true)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let args = Args::parse();
    let conf = Config::from_toml(&args.config).unwrap_or_else(|e| {
        panic!("Failed to load config: {}", e.to_string());
    });
    let key = hash(conf.dump_password.as_bytes());
    let aes = crypto::AES::new(&key);
    let storage = Storage::new(conf.dump_path, aes).unwrap_or_else(|e| {
        panic!("Failed to initialize storage: {}", e.to_string());
    });
    let executor = executor::Executor::new(storage).await;
    WsServer::new(&conf.bind, conf.password, executor)
        .await
        .unwrap_or_else(|e| {
            panic!("Failed to start WebSocket server: {}", e.to_string());
        });
}
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Config error: {0}")]
    ConfigError(#[from] conf::ConfigError),
    #[error("Server error: {0}")]
    ServerError(#[from] ws::WsServerError),
    #[error("Storage error: {0}")]
    StorageError(#[from] storage::StorageError),
    #[error("Tokio mpsc send error: {0}")]
    TokioSendError(#[from] tokio::sync::mpsc::error::SendError<executor::ExecutorCommands>),
    #[error("Oneshot recv error: {0}")]
    OneshotRecvError(#[from] oneshot::RecvError),
}
