use tokio::net::TcpStream;
use async_tungstenite::tokio::{TokioAdapter, connect_async};
use async_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use async_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use async_tungstenite::stream::Stream;
use tokio::{sync::mpsc, sync::watch, sync::Mutex, time};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitStream, SplitSink};
use serde::{Serialize, Deserialize};
use std::ops::Deref;
use std::sync::Arc;
use tracing::{info, error};
use crate::config::Config;
use thiserror::Error;
use std::io::{Error, ErrorKind};
use tokio_native_tls::TlsStream;


#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("Failed to connect to WebSocket: {0}")]
    ConnectError(#[source] async_tungstenite::tungstenite::Error),

    #[error("Failed to send WebSocket message: {0}")]
    SendError(#[source] tokio::io::Error),

    #[error("Failed to receive WebSocket message: {0}")]
    ReceiveError(#[source] tokio::io::Error),

    #[error("Failed to serialize JSON message: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Failed to deserialize JSON message: {0}")]
    DeserializationError(#[source] serde_json::Error),


    #[error("Channel send error: {0}")]
    ChannelSendError(mpsc::error::SendError<Update>),

    #[error("Forward connection error: {0}")]
    ForwardConnectionError(String),

    #[error("Other error: {0}")]
    OtherError(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StockUpdate {
    pub s: String,              // Ticker symbol
    pub t: u64,                 // Timestamp
    pub r#type: String,         // Type
    pub ap: Option<f64>,        // Ask price
    pub r#as: Option<u64>,      // Ask size
    pub bp: Option<f64>,        // Bid price
    pub bs: Option<u64>,        // Bid size
    pub lp: Option<f64>,        // Last price
    pub ls: Option<u64>,        // Last size
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CryptoUpdate {
    pub s: String,          // Symbol
    pub t: u64,             // Timestamp
    pub e: String,          // Exchange
    pub r#type: String,     // Type 
    pub bs: f64,            // Bid price
    pub bp: f64,            // Bid size
    pub r#as: f64,          // Ask size
    pub ap: f64             // Ask price

}

#[derive(Serialize, Deserialize, Debug)]
pub struct ForexUpdate {
    pub s: String,      // Symbol
    pub t: u64,         // Timestamp
    pub r#type: String, // Type
    pub ap: f64,        // Ask price
    pub r#as: u64,      // Ask size
    pub bp: f64,        // Bid price
    pub bs: u64         // Bid size
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Update {
    Crypto(CryptoUpdate),
    Forex(ForexUpdate),
    Stock(StockUpdate),
}

#[derive(Debug, Clone)]
struct HandlerArgs {
    fetch_type: Option<FetchType>,
    target_uri: Option<String>,
    tickers: Option<Vec<String>>
}

enum  TargetUri {
    CryptoUri(String),
    ForexUri(String),
    StockUri(String),
}
impl TargetUri {
    fn from_fetch_type(fetch_type: FetchType, config: Arc<Config>) -> Option<String> {
        match  fetch_type {
            FetchType::Crypto => Some(config.websocket.crypto_ws.clone()),
            FetchType::Forex => Some(config.websocket.forex_ws.clone()),
            FetchType::Stock => Some(config.websocket.stock_ws.clone()),
            _ => None,
        }
    }
}