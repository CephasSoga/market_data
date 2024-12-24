#![allow(warnings)]
pub mod websocket;
pub mod config;
pub  mod auth;
pub mod api_calls;


#[tokio::main]
async fn main() {
    websocket::from_base::batch_stock_from_base::example().await;
}