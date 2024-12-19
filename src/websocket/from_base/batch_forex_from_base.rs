use crate::websocket::from_base::base_batch::{Update, GenericHandler, WebSocketHandler, WebSocketError};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use tokio::{sync::mpsc, sync::watch, sync::Mutex, time};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tracing::{info, error};
use futures::stream::{SplitSink, SplitStream};
use crate::config::Config;
use thiserror::Error;
use std::io::{Error, ErrorKind};
use std::collections::HashSet;
use std::fmt::{self, Display};


#[derive(Debug)]
pub enum ErrorStatus {
    ConsumptionFailed(i16)
}
impl Display for ErrorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorStatus::ConsumptionFailed(code) => {
                write!(f, "{}", code)
            }
        }
    }
}

#[derive(Error, Debug)]
pub struct  ForexError {
    wrapped: WebSocketError,
    status: ErrorStatus,
}
impl Display for ForexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ForexError(status: {}, wrapped: {})",
            self.status, self.wrapped
        )
    }
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

pub struct ForexSocket {
    config: Arc<Config>,
    handler: GenericHandler,
    tickers: Vec<String>, 
}

impl ForexSocket {
    async fn new(config: Config) -> Self {
        let handler = GenericHandler::new(config.clone()).await;
        let config = Arc::new(config);
        Self {
            config, 
            handler, 
            tickers: Vec::new(), 
        }
    }
    

    async fn consum_stream(&mut self, auto_shutdown: bool) -> Result<(), ForexError> {
        let config = Arc::clone(&self.config);
        let ws = &config.websocket.stock_ws;
        let fwd_ws = Some(config.websocket.fwd_stock_ws.clone());

        let tickers = self.tickers.clone();

        self.handler.consume(ws, tickers, auto_shutdown, fwd_ws).await
            .map_err(|err| {ForexError {wrapped: err, status: ErrorStatus::ConsumptionFailed(0)}})
    }
}