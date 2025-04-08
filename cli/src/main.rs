use ckeylock_api::CKeyLockAPI;
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(name = "ckeylock-cli")]
#[command(
    about = "CKeyLock CLI",
    long_about = "CKeyLock CLI is a tool to interact with CKeyLock storage via API.\nMake sure you have the CKEYLOCK_BIND and CKEYLOCK_PASSWORD variables set."
)]
#[command(version = "0.1.0", author = "oblivisheee")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Clone)]
enum Commands {
    Set {
        #[arg(long, short)]
        key: String,
        #[arg(long, short)]
        value: String,
    },
    Get {
        #[arg(long, short)]
        key: String,
    },
    Delete {
        #[arg(long, short)]
        key: String,
    },
    List,
    Exists {
        #[arg(long, short)]
        key: String,
    },
    Count,
    Clear,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let binds = retrieve_binds_from_env();
    let (bind, password) = match binds {
        Some((bind, password)) => (bind, password),
        None => {
            eprintln!("Please set CKEYLOCK_BIND and CKEYLOCK_PASSWORD environment variables.");
            return;
        }
    };
    let api = CKeyLockAPI::new(&bind, password.as_deref());
    let connection = api.connect().await.unwrap_or_else(|e| {
        eprintln!("Failed to connect to server: {}", e);
        std::process::exit(1);
    });
    match cli.command {
        Commands::Set { key, value } => {
            let result = connection
                .set(key.as_bytes().to_vec(), value.as_bytes().to_vec())
                .await;
            match result {
                Ok(_) => println!("Key set successfully."),
                Err(e) => eprintln!("Failed to set key: {}", e),
            }
        }
        Commands::Get { key } => {
            let result = connection.get(key.as_bytes().to_vec()).await;
            match result {
                Ok(value) => {
                    if let Some(value) = value {
                        println!("Value: {:?}", String::from_utf8_lossy(&value));
                    } else {
                        println!("Key not found.");
                    }
                }
                Err(e) => eprintln!("Failed to get key: {}", e),
            }
        }
        Commands::Delete { key } => {
            let result = connection.delete(key.as_bytes().to_vec()).await;
            match result {
                Ok(_) => println!("Key deleted successfully."),
                Err(e) => eprintln!("Failed to delete key: {}", e),
            }
        }
        Commands::List => {
            let result = connection.list().await;
            match result {
                Ok(keys) => println!("Keys: {:?}", keys),
                Err(e) => eprintln!("Failed to list keys: {}", e),
            }
        }
        Commands::Exists { key } => {
            let result = connection.exists(key.as_bytes().to_vec()).await;
            match result {
                Ok(exists) => println!("Key exists: {}", exists),
                Err(e) => eprintln!("Failed to check if key exists: {}", e),
            }
        }
        Commands::Count => {
            let result = connection.count().await;
            match result {
                Ok(count) => println!("Key count: {}", count),
                Err(e) => eprintln!("Failed to count keys: {}", e),
            }
        }
        Commands::Clear => {
            let result = connection.clear().await;
            match result {
                Ok(_) => println!("All keys cleared."),
                Err(e) => eprintln!("Failed to clear keys: {}", e),
            }
        }
    }
}

fn retrieve_binds_from_env() -> Option<(String, Option<String>)> {
    let bind = std::env::var("CKEYLOCK_BIND").ok()?;
    let password = std::env::var("CKEYLOCK_PASSWORD").ok();
    Some((bind, password))
}
