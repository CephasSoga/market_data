use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use tokio::{sync::mpsc, sync::watch, time};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::error::Error;
use tracing::{info, error};
use futures::stream::{SplitSink, SplitStream};
use crate::config::Config;


#[derive(Serialize, Deserialize, Debug)]
struct ForexUpdate {
    s: String,      // Symbol
    t: u64,         // Timestamp
    r#type: String, // Type
    ap: f64,        // Ask price
    r#as: u64,      // Ask size
    bp: f64,        // Bid price
    bs: u64         // Bid size
}


struct ForexSocket {
    config: Arc<Config>,
    write: Option<SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>,
    read: Option<SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}
impl ForexSocket {

    pub async fn new(config: Config) -> Self {
        ForexSocket {
                config: Arc::new(config),
                write: None,
                read: None,
        }
    }

    async fn handle_update(update: ForexUpdate) -> Result<(), Box<dyn Error>> {
        info!("Update for {}: {:?}", update.s, update);
        // Simulate some processing time (e.g., database insert or analysis)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    async fn process_batch(buffer: &mut Vec<ForexUpdate>) {
        let batch = buffer.drain(..).collect::<Vec<_>>();
        for update in batch {
            if let Err(err) = ForexSocket ::handle_update(update).await {
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
        let url = &self.config.websocket.forex_ws;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
    
        self.write = Some(write);
    
        let (tx, mut rx) = mpsc::channel::<ForexUpdate>(self.config.websocket.channel_bound);
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
                        if let Ok(update) = serde_json::from_str::<ForexUpdate>(&text) {
                            if tx.send(update).await.is_err() {
                                error!("Receiver dropped. Exiting WebSocket reader.");
                                break;
                            }
                        }
                    }
                    Ok(Message::Binary(bin)) => {
                        if let Ok(update) = serde_json::from_slice::<ForexUpdate>(&bin) {
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
/// Tickers: ["ETHUSD", "BTCUSD", "DOGUSD"];
async fn example(tickers: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = Config::new()?;
    
    let mut websocket = ForexSocket::new(config).await;
    
    websocket.consume(tickers, true).await?;

    Ok(())
}
