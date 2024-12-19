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

pub struct StockSocket {
    config: Arc<Config>,
    handler: GenericHandler,
    tickers: Vec<String>, 
}

impl StockSocket {
    async fn new(config: Config) -> Self {
        let handler = GenericHandler::new(config.clone()).await;
        let config = Arc::new(config);
        Self {
            config, 
            handler, 
            tickers: Vec::new(), 
        }
    }
    async fn change_tickers(&mut self, new_tickers: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        self.tickers = new_tickers;
        Ok(())
    }

    async fn consum_stream(&mut self, auto_shutdown: bool) -> Result<(), WebSocketError> {
        let config = Arc::clone(&self.config);
        let ws = &config.websocket.stock_ws;
        let fwd_ws = Some(config.websocket.fwd_stock_ws.clone());

        self.consume(ws, self.tickers.clone(), auto_shutdown, fwd_ws).await
    }
}

impl WebSocketHandler for StockSocket {
    async fn new(config: Config) -> Self {
        Self::new(config).await
    }

    async fn close(&mut self, reason: Option<String>) -> Result<(), WebSocketError> {
        self.handler.close(reason).await
    }

    async fn login(&mut self) -> Result<(), WebSocketError> {
        todo!()
    }

    async fn subscribe(&mut self, tickers: &Vec<String>) -> Result<(),WebSocketError> {
        todo!()
    }

    async fn connect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError> {
        self.handler.connect_forward_ws(url).await
    }

    async fn reconnect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError> {
        self.handler.reconnect_forward_ws(url).await
    }

    async fn stream(&mut self, ws: &String, tickers: &Vec<String>, shutdown_rx: watch::Receiver<bool>) -> Result<(), WebSocketError> {
        self.handler.stream(ws, tickers, shutdown_rx).await
    }

    async fn consume(&mut self, ws: &String, tickers: Vec<String>, auto_shutdown: bool, forward_url: Option<String>) -> Result<(), WebSocketError> {
        self.handler.consume(ws, tickers, auto_shutdown, forward_url).await
    }
}