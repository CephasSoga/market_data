#![allow(warnings)]

pub mod commodity;
pub mod request;
pub mod auth;
pub mod crypto;
pub mod etf;
pub mod financial;
pub mod forex;
pub mod market;
pub mod index;
pub mod mutualfund;
pub mod search;
pub mod stock;
pub mod config;
pub mod websocket;

use crate::market::Market;

#[tokio::main]
async fn main() {
    index::example().await;
}
