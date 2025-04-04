# CKeyLock

CKeyLock is a secure and efficient tool written in Rust for managing and storing your cryptographic keys.

## Features

- **Secure Storage**: Safeguard your cryptographic keys with robust encryption.
- **Rust-Powered**: Built with Rust, ensuring high performance and memory safety.
- **User-Friendly**: Simple and intuitive interface for seamless key management.

## Getting Started

To get started with CKeyLock, follow the instructions below:

1. Clone the repository:
    ```bash
    git clone https://github.com/oblivisheee/ckeylock.git
    ```
2. Navigate to the project directory:
    ```bash
    cd ckeylock
    ```
3. Build the project:
    ```bash
    cargo build --release -p ckeylock
    ```
4. Create a config with name `Ckeylock.toml`. For example
    ```toml
    bind = "127.0.0.1:8080"
    password = "helloworld"
    dump_path = "dump-clok.bin"
    dump_password = "helloworld"
    ```
4. Run the application:
    ```bash
    ./target/release/ckeylock
    ```

## API

To use CKeyLock in your project follow these steps:
1. Create a new project.
    ```bash
    cargo init
    ```
2. Add API lib.
    ```bash
    cargo add ckeylock-api
    ```
3. Initialize connection.
    ```rust
    let api = CKeyLockAPI::new("127.0.0.1:8080", Some("helloworld"));
    let mut connection = api.connect().await.unwrap();
    ```
4. Use!

## Contributing

We welcome contributions! Feel free to submit issues or pull requests to help improve CKeyLock.

## License

CKeyLock is licensed under the [MIT License](LICENSE).

---
Start securing your cryptographic keys with CKeyLock today!