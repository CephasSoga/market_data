#![allow(warnings)]
pub mod websocket;
pub mod config;
pub  mod auth;
pub mod logging;
pub mod options;
pub mod request_parser;

use std::sync::Arc;

use websocket::WsHandler;

use crate::config::Config;
use crate::websocket::WebSocketProxyServer;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    let mut ws = WebSocketProxyServer::new(
        Arc::new(config),
        "0.0.0.0:8080"
    );

    ws.run().await.expect("Failed to run Ws.")
}