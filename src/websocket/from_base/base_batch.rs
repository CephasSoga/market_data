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

#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("Failed to connect to WebSocket: {0}")]
    ConnectError(#[source] tokio_tungstenite::tungstenite::Error),

    #[error("Failed to send WebSocket message: {0}")]
    SendError(#[source] tokio::io::Error),

    #[error("Failed to receive WebSocket message: {0}")]
    ReceiveError(#[source] tokio::io::Error),

    #[error("Failed to serialize JSON message: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Failed to deserialize JSON message: {0}")]
    DeserializationError(#[source] serde_json::Error),

    #[error("Batch processing error: {0}")]
    BatchProcessingError(String),

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

// SharedState for Batch Processing
type SharedState = Arc<Mutex<BatchState>>;

struct BatchState {
    pub buffer: Vec<Update>,
    pub forward_write: Option<Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    pub buffer_size: usize,
}

impl BatchState {
    async fn handle_update(&mut self, update: Update) -> Result<(), WebSocketError> {
        if let Some(ref forward_write) = self.forward_write {
            let message = serde_json::to_string(&update)
                .map_err(|err| WebSocketError::SerializationError(err))?; 
            if let mut write_lock = forward_write.lock().await {
                write_lock.send(Message::Text(message))
                    .await
                    .map_err(|err| WebSocketError::SendError(tokio::io::Error::new(ErrorKind::ConnectionRefused, err.to_string())))?;
            }
        }
        Ok(())
    }

    async fn process(&mut self) -> Result<(), WebSocketError> {
        let batch = self.buffer.drain(..).collect::<Vec<_>>();
    
        for update in batch {
            self.handle_update(update).await;
        }
        Ok(())
    }

    fn log_for_update(&self, msg: &str, update: Update) {
        match update {
            Update::Crypto(crypto_update) => info!("{}: {:?}", msg, crypto_update.s),
            Update::Forex(forex_update) => info!("{}: {:?}", msg, forex_update.s),
            Update::Stock(stock_update) => info!("{}: {:?}", msg, stock_update.s),
        }
    }
}


pub trait WebSocketHandler {
    async fn new(config: Config) -> Self;
    async fn connect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError>;
    async fn reconnect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError>;
    async fn close(&mut self, reason: Option<String>) -> Result<(), WebSocketError>;
    async fn stream(&mut self, ws: &String, tickers: &Vec<String>, shutdown_rx: watch::Receiver<bool>) -> Result<(), WebSocketError>;
    async fn login(&mut self) -> Result<(), WebSocketError>;
    async fn subscribe(&mut self, tickers: &Vec<String>) -> Result<(),WebSocketError>;
    async fn consume(&mut self, ws: &String, tickers: Vec<String>, auto_shutdown: bool, forward_url: Option<String>) -> Result<(), WebSocketError>;
}


pub struct GenericHandler {
    pub config: Arc<Config>,
    pub write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    pub read: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    pub forward_write: Option<Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>

}
impl WebSocketHandler for GenericHandler {
    async fn new(config: Config) -> Self {
        Self {
                config: Arc::new(config),
                write: None,
                read: None,
                forward_write: None,
        }
    }

    async fn connect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError> {
        let (ws_stream, _) = connect_async(url).await
            .map_err(|err| WebSocketError::ConnectError(err))?;

        let (write, _) = ws_stream.split();
        self.forward_write = Some(Arc::new(Mutex::new(write)));
        info!("Connected to forward WebSocket at {}", url);
        Ok(())
    }

    async fn reconnect_forward_ws(&mut self, url: &str) -> Result<(), WebSocketError> {
        loop {
            match self.connect_forward_ws(url).await {
                Ok(_) => break,
                Err(err) => {
                    error!("Failed to connect to forward WebSocket: {}. Retrying...", err);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
        Ok(())
    }

    /// Close the WebSocket connection
    async fn close(&mut self, reason: Option<String>) -> Result<(), WebSocketError> {
        let reason = reason.unwrap_or_else(|| "No specific reason provided".to_string());

        let close_frame = CloseFrame {
            code: CloseCode::Normal, // Specify the close code
            reason: reason.clone().into(), // Optional reason
        };

        // Close the write side of the WebSocket
        if let Some(mut write) = self.write.take() {
            write.close().await
                .map_err(|err| {WebSocketError::ConnectError(err)});
            info!("WebSocket connection closed. Reason: {}", reason);
        } else {
            info!("WebSocket write connection was already closed.");
        }

        // Drop the read half (optional cleanup)
        if self.read.take().is_some() {
            info!("WebSocket read connection dropped.");
        }

        Ok(())
    }

    async fn login(&mut self) -> Result<(), WebSocketError> {
        // Send login and subscription messages
        let login_message = serde_json::json!({
            "event": "login",
            "data": { "apiKey": self.config.api.token }
        });
        self.write.as_mut().unwrap().send(Message::Text(login_message.to_string())).await
            .map_err(|err| {WebSocketError::ConnectError(err)})
    }

    async fn subscribe(&mut self, tickers: &Vec<String>) -> Result<(), WebSocketError>  {
        let subscribe_message = serde_json::json!({
            "event": "subscribe",
            "data": { "ticker": tickers }
        });
        self.write.as_mut().unwrap().send(Message::Text(subscribe_message.to_string())).await
            .map_err(|err| {WebSocketError::ConnectError(err)})
    
    }

    async fn stream(
        &mut self,
        ws: &String,
        tickers: &Vec<String>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<(), WebSocketError> {
        let (ws_stream, _) = connect_async(ws).await
            .map_err(|err| WebSocketError::ConnectError(err))?;
        let (write, mut read) = ws_stream.split();
    
        self.write = Some(write);
    
        let shared_state = Arc::new(Mutex::new(BatchState {
            buffer: Vec::new(),
            forward_write: self.forward_write.clone(),
            buffer_size: self.config.websocket.buffer_size,
        }));
    
        let (tx, mut rx) = mpsc::channel::<Update>(self.config.websocket.channel_bound);
        let interval_ms = self.config.websocket.batch_interval_ms;
    
        let shared_state_clone = shared_state.clone();
        let mut shutdown_rx_clone = shutdown_rx.clone();
    
        // Spawn batch processing task
        tokio::spawn(async move {
            let mut interval = time::interval(time::Duration::from_millis(interval_ms));
            loop {
                tokio::select! {
                    Some(update) = rx.recv() => {
                        let mut state = shared_state_clone.lock().await;
                        state.buffer.push(update);
                        if state.buffer.len() >= state.buffer_size {
                            state.process().await;
                        }
                    },
                    _ = interval.tick() => {
                        let mut state = shared_state_clone.lock().await;
                        if !state.buffer.is_empty() {
                            state.process().await;
                        }
                    },
                    _ = shutdown_rx_clone.changed() => {
                        info!("Shutdown signal received in batch processor.");
                        break;
                    }
                }
            }
        });
    
        //Login end subscribe to tickers
        self.login().await;
        self.subscribe(tickers).await;
        
        // Read WebSocket messages
        while !*shutdown_rx.borrow() {
            if let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        serde_json::from_str::<Update>(&text)
                            .map_err(|err| WebSocketError::DeserializationError(err))?;
                    }
                    Ok(Message::Binary(bin)) => {
                        serde_json::from_slice::<Update>(&bin)
                            .map_err(|err| WebSocketError::DeserializationError(err))?;
                    }
                    _ => (),
                }
            }
        }
    
        // Clean up
        self.close(Some("Stream ending.".to_string())).await?;
        Ok(())
    }
    

    async fn consume(&mut self, ws: &String, tickers: Vec<String>, auto_shutdown: bool, forward_url: Option<String>) -> Result<(), WebSocketError> {

        if let Some(url) = forward_url {
            self.connect_forward_ws(&url).await?;
        }
    
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        // Clone the `Arc` for use in the spawned task
        let config = Arc::clone(&self.config);
    
        if auto_shutdown {
            // Spawn a task to trigger shutdown after configured delay
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(config.websocket.shutdown_delay_ms)).await;
                let _ = shutdown_tx.send(true);
            });
        }
    
        tokio::select! {
            result = self.stream(ws, &tickers, shutdown_rx.clone()) => {
                if let Err(err) = result {
                    error!("Error in WebSocket consumption: {}", err);
                }
            }
            _ = shutdown_rx.changed() => {
                info!("Shutdown signal received. Exiting.");
            }
        }
    
        Ok(())
    }
}