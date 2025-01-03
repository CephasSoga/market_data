use tokio::net::TcpStream;
use async_tungstenite::tokio::{TokioAdapter, connect_async};
use async_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use async_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use async_tungstenite::stream::Stream;
use tokio_native_tls::TlsStream;
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::watch;

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
}
impl ForexSocket {
    async fn handle_update(update: ForexUpdate) {
        println!("Update for {}: {:?}", update.s, update);
        // Simulate some processing time (e.g., database insert or analysis)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    async fn stream(&self, tickers: &Vec<String>) -> Result<(), Box<dyn Error>> {
        let url = &self.config.websocket.forex_ws; //######
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Channel for updates
        let (tx, mut rx) = mpsc::channel
            ::<ForexUpdate>(self.config.websocket.channel_bound);

        // Spawn a task to handle updates
        tokio::spawn(async move {
            while let Some(update) = rx.recv().await {
                tokio::spawn(Self::handle_update(update));
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
        while let Some(message) = read.next().await {
            match message? {
                Message::Text(text) => {
                    if let Ok(update) = serde_json::from_str::<ForexUpdate>(&text) {
                        if tx.send(update).await.is_err() {
                            eprintln!("Receiver dropped. Exiting WebSocket reader.");
                            break;
                        }
                    }
                }
                Message::Binary(bin) => {
                    if let Ok(update) = serde_json::from_slice::<ForexUpdate>(&bin) {
                        if tx.send(update).await.is_err() {
                            eprintln!("Receiver dropped. Exiting WebSocket reader.");
                            break;
                        }
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    async fn consume(&self, tickers: Vec<String>) -> Result<(), Box<dyn Error>> {

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let config = Arc::clone(&self.config);
    
        tokio::spawn(async move {
            // Example: Trigger shutdown after <SOME> seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(
                config.websocket.shutdown_delay_ms
            )).await;
            let _ = shutdown_tx.send(true);
        });
    
        tokio::select! {
            result = self.stream(&tickers) => {
                if let Err(err) = result {
                    eprintln!("Error in WebSocket consumption: {}", err);
                }
            }
            _ = shutdown_rx.changed() => {
                println!("Shutdown signal received. Exiting.");
            }
        }

        Ok(())
    }
}
