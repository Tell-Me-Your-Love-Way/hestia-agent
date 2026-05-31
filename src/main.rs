use std::env;

use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::prelude::*;
mod configs;
mod handlers;
mod models;
mod error;
mod services;
// Handler function responding with "Hello World" for HTTP/3 requests
#[handler]
async fn hello() -> &'static str {
    "Hello World"
}

#[tokio::main]
async fn main() {
    dotenv::dotenv();
    // Initialize logging system
    tracing_subscriber::fmt().init();

    let domain: String = env::var("SAN_DOMAIN").map_err(|_|panic!("Missing SAN_DOMAIN env var!")).unwrap();
    let ip: String = env::var("SAN_IP").map_err(|_|panic!("Missing SAN_DOMAIN env var!")).unwrap();

    // Load TLS certificate and private key from embedded PEM files
    let (cert, key) = configs::provider_cert::gen_tls(domain, ip).await;

    // Create router with single endpoint
    let router = configs::provider_router::build();

    // Configure TLS settings using Rustls
    let config = RustlsConfig::new(Keycert::new().cert(cert.as_slice()).key(key.as_slice()));

    // Create TCP listener with TLS encryption on port 8698
    let listener = TcpListener::new(("0.0.0.0", 8698)).rustls(config.clone());

    // Create QUIC listener and combine with TCP listener
    let acceptor = QuinnListener::new(
            config.build_quinn_config().unwrap(), 
            ("0.0.0.0", 8698)
        )
        .join(listener)
        .bind()
        .await;

    // Start server supporting both HTTP/3 (QUIC) and HTTPS (TCP)
    Server::new(acceptor).serve(router).await;
}