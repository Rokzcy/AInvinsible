[package]
name = "stealth-rotator"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
tokio-tungstenite = "0.20"
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"
tokio-stream = "0.1"
tracing = "0.1"
tracing-subscriber = "0.3"
