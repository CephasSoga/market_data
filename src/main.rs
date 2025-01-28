#![allow(warnings)]
pub mod websocket;
pub mod config;
pub  mod auth;
pub mod logging;
pub mod options;


#[tokio::main]
async fn main() {
    websocket::from_base::batch_stock_from_base::example().await;
}