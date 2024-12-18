use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::error::Error;

use crate::config::Config;

#[derive(Serialize, Deserialize, Debug)]
struct CryptoUpdate {
    s: String,          // Symbol
    t: u64,             // Timestamp
    e: String,          // Exchange
    r#type: String,     // Type 
    bs: f64,            // Bid price
    bp: f64,            // Bid size
    r#as: f64,          // Ask size
    ap: f64             // Ask price

}

struct CryptoSocket {
    config: Config,
}
impl CryptoSocket {
    async fn handle_update(update: CryptoUpdate) {
        println!("Update for {}: {:?}", update.s, update);
        // Simulate some processing time (e.g., database insert or analysis)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    async fn consume(&self, tickers: &Vec<String>) -> Result<(), Box<dyn Error>> {
        let url = &self.config.websocket.crypto_ws; //######
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Channel for updates
        let (tx, mut rx) = mpsc::channel
            ::<CryptoUpdate>(self.config.websocket.channel_bound);

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
                    if let Ok(update) = serde_json::from_str::<CryptoUpdate>(&text) {
                        if tx.send(update).await.is_err() {
                            eprintln!("Receiver dropped. Exiting WebSocket reader.");
                            break;
                        }
                    }
                }
                Message::Binary(bin) => {
                    if let Ok(update) = serde_json::from_slice::<CryptoUpdate>(&bin) {
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
}