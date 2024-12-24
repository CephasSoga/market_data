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
pub struct  StockError {
    wrapped: WebSocketError,
    status: ErrorStatus,
}
impl Display for StockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StockError(status: {}, wrapped: {})",
            self.status, self.wrapped
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StockUpdate {
    pub s: String,  // Ticker symbol
    pub t: u64,     // Timestamp
    pub r#type: String, // Type
    pub ap: Option<f64>, // Ask price
    pub r#as: Option<u64>, // Ask size
    pub bp: Option<f64>, // Bid price
    pub bs: Option<u64>, // Bid size
    pub lp: Option<f64>, // Last price
    pub ls: Option<u64>, // Last size
}

// ... (Rest of the code for Update, BatchState, GenericHandler, WebSocketHandler, and WebSocketError) ...

pub struct StockSocket {
    config: Arc<Config>,
    pub handler: GenericHandler,
    pub tickers: Vec<String>, 
}

impl StockSocket {
    pub async fn new(config: Config) -> Self {
        let handler = GenericHandler::new(config.clone()).await;
        let config = Arc::new(config);
        Self {
            config, 
            handler, 
            tickers: Vec::new(), 
        }
    }
    /// Adds a ticker to the `tickers` field only if it is not already present.
    fn update_tickers(&mut self, tickers: Vec<&str>) {
        // Check if the ticker is not already in the list
        for ticker in tickers {
            if !self.tickers.contains(&ticker.to_string()) {
                self.tickers.push(ticker.to_string());
            }
        }
    }

    pub async fn consum_stream(&mut self, auto_shutdown: bool) -> Result<(), StockError> {
        println!("Running...");
        let config = Arc::clone(&self.config);
        let ws = &config.websocket.stock_ws;
        let fwd_ws = Some(config.websocket.fwd_stock_ws.clone());

        let tickers = self.tickers.clone();

        self.handler.consume(ws, tickers, auto_shutdown, fwd_ws).await
            .map_err(|err| {StockError {wrapped: err, status: ErrorStatus::ConsumptionFailed(0)}})
    }
}

pub async fn example() {
    let config = Config::new().unwrap();
    let mut stockWs = StockSocket::new(config).await;
    stockWs.update_tickers(vec!["AAPL"]);
    stockWs.consum_stream(false).await.expect("Failed to run.");
}