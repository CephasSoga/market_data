use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::{sync::mpsc, sync::watch, time};
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::error::Error;
use tracing::{info, error};
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

struct StockSocket {
    config: Arc<Config>,
}

impl StockSocket {
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

    async fn stream(&self, tickers: &Vec<String>, shutdown_rx: watch::Receiver<bool>) -> Result<(), Box<dyn Error>> {
        let url = &self.config.websocket.stock_ws;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Channel for updates
        let (tx, mut rx) = mpsc::channel::<StockUpdate>(self.config.websocket.channel_bound);

        // Read interval setting
        let interval_ms = self.config.websocket.batch_interval_ms;

        // Spawn a task to process updates
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            let mut interval = time::interval(time::Duration::from_millis(interval_ms)); // E.g: Flush every 500ms

            loop {
                tokio::select! {
                    Some(update) = rx.recv() => {
                        buffer.push(update);
                        if buffer.len() >= 10 {
                            Self::process_batch(&mut buffer).await;
                        }
                    },
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            Self::process_batch(&mut buffer).await;
                        }
                    }
                }
            }
        });

        // Send login message
        let login_message = serde_json::json!({
            "event": "login",
            "data": {
                "apiKey": self.config.api.token,
            }
        });
        write.send(Message::Text(login_message.to_string())).await?;

        // Subscribe to tickers
        let subscribe_message = serde_json::json!({
            "event": "subscribe",
            "data": {
                "ticker": tickers,
            }
        });
        write.send(Message::Text(subscribe_message.to_string())).await?;

        // Read WebSocket messages and send to channel
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

        // Close WebSocket connection
        write.close().await?;
        Ok(())
    }

    async fn consume(&self, tickers: Vec<String>) -> Result<(), Box<dyn Error>> {
        // Initialize logging
        tracing_subscriber::fmt::init();
    
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        // Clone the `Arc` for use in the spawned task
        let config = Arc::clone(&self.config);
    
        // Spawn a task to trigger shutdown after a delay
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(config.websocket.shutdown_delay_ms)).await;
            let _ = shutdown_tx.send(true);
        });

    
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
/// Tickers: ["AAPL", "GOOGL", "MSFT"];
async fn example(tickers: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = Arc::new(Config::new()?);
    
    let stock_websocket = StockSocket {config};

    stock_websocket.consume(tickers).await?;

    Ok(())
}
