use std::env;
use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::prelude::*;
mod configs;
mod handlers;
mod models;
mod error;
mod services;

#[handler]
async fn hello() -> &'static str {
    "Hello World"
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    
    tracing_subscriber::fmt().init();

    let domain: String = env::var("SAN_DOMAIN").map_err(|_|panic!("Missing SAN_DOMAIN env var!")).unwrap();
    let ip: String = env::var("SAN_IP").map_err(|_|panic!("Missing SAN_DOMAIN env var!")).unwrap();
    
    let (cert, key) = configs::provider_cert::gen_tls(domain, ip).await;
    
    let router = configs::provider_router::build();
    
    let config = RustlsConfig::new(Keycert::new().cert(cert.as_slice()).key(key.as_slice()));
    
    let listener = TcpListener::new(("0.0.0.0", 8698)).rustls(config.clone());
    
    let acceptor = QuinnListener::new(
            config.build_quinn_config().unwrap(), 
            ("0.0.0.0", 8698)
        )
        .join(listener)
        .bind()
        .await;
    
    Server::new(acceptor).serve(router).await;
}