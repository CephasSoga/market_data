use tokio::net::TcpStream;
use async_tungstenite::tokio::{TokioAdapter, connect_async};
use async_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use async_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use async_tungstenite::stream::Stream;
use tokio_native_tls::TlsStream;
use tokio::{sync::mpsc, sync::watch, time};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::error::Error;
use tracing::{info, error};
use futures::stream::{SplitSink, SplitStream};
use crate::config::Config;


#[derive(Serialize, Deserialize, Debug)]
struct StockUpdate {
    s: String,              // Ticker symbol
    t: u64,                 // Timestamp
    r#type: String,         // Type
    ap: Option<f64>,        // Ask price
    r#as: Option<u64>,      // Ask size
    bp: Option<f64>,        // Bid price
    bs: Option<u64>,        // Bid size
    lp: Option<f64>,        // Last price
    ls: Option<u64>,        // Last size
}

// reader and writer types
type WebSocketStreamType = WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>; 
type WebSocketWriter = SplitSink<WebSocketStreamType, Message>; 
type WebSocketReader = SplitStream<WebSocketStreamType>;


struct StockSocket {
    config: Arc<Config>,
    write: Option<WebSocketWriter>,
    read: Option<WebSocketReader>,
}
impl StockSocket {

    pub async fn new(config: Config) -> Self {
        StockSocket {
                config: Arc::new(config),
                write: None,
                read: None,
        }
    }

    async fn handle_update(update: StockUpdate) -> Result<(), Box<dyn Error>> {
        info!("Update for {}: {:?}", update.s, update);
        // Simulate some processing time (e.g., database insert or analysis)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    async fn process_batch(buffer: &mut Vec<StockUpdate>) {
        let batch = buffer.drain(..).collect::<Vec<_>>();
        for update in batch {
            if let Err(err) = StockSocket::handle_update(update).await {
                error!("Failed to handle update: {:?}", err);
            }
        }
    }

    /// Close the WebSocket connection
    pub async fn close(&mut self, reason: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let reason = reason.unwrap_or_else(|| "No specific reason provided".to_string());

        let close_frame = CloseFrame {
            code: CloseCode::Normal, // Specify the close code
            reason: reason.clone().into(), // Optional reason
        };

        // Close the write side of the WebSocket
        if let Some(mut write) = self.write.take() {
            write.close().await?;
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


    async fn stream(&mut self, tickers: &Vec<String>, shutdown_rx: watch::Receiver<bool>) -> Result<(), Box<dyn Error>> {
        let url = &self.config.websocket.stock_ws;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
    
        self.write = Some(write);
    
        let (tx, mut rx) = mpsc::channel::<StockUpdate>(self.config.websocket.channel_bound);
        let interval_ms = self.config.websocket.batch_interval_ms;
    
        // Clone the `Arc` for use in the spawned task
        let config = Arc::clone(&self.config);

        // Spawn task for batch processing
        let mut shutdown_rx_clone = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut interval = time::interval(time::Duration::from_millis(interval_ms));
    
            loop {
                tokio::select! {
                    Some(update) = rx.recv() => {
                        buffer.push(update);
                        if buffer.len() >= config.websocket.buffer_size {
                            Self::process_batch(&mut buffer).await;
                        }
                    },
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            Self::process_batch(&mut buffer).await;
                        }
                    },
                    _ = shutdown_rx_clone.changed() => {
                        info!("Shutdown signal received in batch processor.");
                        break;
                    }
                }
            }
        });
    
        // Send login and subscription messages
        let login_message = serde_json::json!({
            "event": "login",
            "data": { "apiKey": self.config.api.token }
        });
        self.write.as_mut().unwrap().send(Message::Text(login_message.to_string())).await?;
    
        let subscribe_message = serde_json::json!({
            "event": "subscribe",
            "data": { "ticker": tickers }
        });
        self.write.as_mut().unwrap().send(Message::Text(subscribe_message.to_string())).await?;
    
        // Read WebSocket messages
        while !*shutdown_rx.borrow() {
            if let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(update) = serde_json::from_str::<StockUpdate>(&text) {
                            if tx.send(update).await.is_err() {
                                error!("Receiver dropped. Exiting WebSocket reader.");
                                break;
                            }
                        }
                    }
                    Ok(Message::Binary(bin)) => {
                        if let Ok(update) = serde_json::from_slice::<StockUpdate>(&bin) {
                            if tx.send(update).await.is_err() {
                                error!("Receiver dropped. Exiting WebSocket reader.");
                                break;
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    
        // Clean up
        self.close(Some("Stream ending.".to_string())).await?;
        Ok(())
    }
    

    async fn consume(&mut self, tickers: Vec<String>, auto_shutdown: bool) -> Result<(), Box<dyn Error>> {
    
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
            result = self.stream(&tickers, shutdown_rx.clone()) => {
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



#[tokio::main]
/// Tickers: ["AAPL", "GOOGL", "MSFT"]
async fn example(tickers: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = Config::new()?;
    
    let mut websocket = StockSocket::new(config).await;
    
    websocket.consume(tickers, true).await?;

    Ok(())
}
