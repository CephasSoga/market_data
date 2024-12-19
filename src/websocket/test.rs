use crate::websocket::base_batch::{Update, GenericHandler, WebSocketHandler, WebSocketError};
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
    handler: GenericHandler,
    subscribed_tickers: HashSet<String>, 
}

impl StockSocket {
    async fn new(config: Config) -> Self {
        let handler = GenericHandler::new(config.clone()).await;
        Self { 
            handler, 
            subscribed_tickers: HashSet::new(), 
        }
    }

    async fn subscribe_to_stock(&mut self, ticker: &str) -> Result<(), WebSocketError> {
        if !self.subscribed_tickers.contains(ticker) {
            let subscribe_message = json!({
                "event": "subscribe",
                "data": { "ticker": vec![ticker.to_string()] } 
            });
            self.handler.write.as_mut().unwrap().send(Message::Text(subscribe_message.to_string())).await;
            self.subscribed_tickers.insert(ticker.to_string());
        }
        Ok(())
    }

    async fn unsubscribe_from_stock(&mut self, ticker: &str) -> Result<(), WebSocketError> {
        if self.subscribed_tickers.remove(ticker) {
            let unsubscribe_message = json!({
                "event": "unsubscribe",
                "data": { "ticker": vec![ticker.to_string()] } 
            });
            self.handler.write.as_mut().unwrap().send(Message::Text(unsubscribe_message.to_string())).await;
        }
        Ok(())
    }

    async fn consume(&mut self, ) {

    }
}
