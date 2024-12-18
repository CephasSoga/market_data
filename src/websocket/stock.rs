use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::error::Error;
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
    config: Config,
}
impl StockSocket {
    async fn handle_update(update: StockUpdate) {
        println!("Update for {}: {:?}", update.s, update);
        // Simulate some processing time (e.g., database insert or analysis)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    async fn consume(&self, tickers: &Vec<String>) -> Result<(), Box<dyn Error>> {
        let url = &self.config.websocket.stock_ws;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Channel for updates
        let (tx, mut rx) = mpsc::channel
            ::<StockUpdate>(self.config.websocket.channel_bound);

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
                    if let Ok(update) = serde_json::from_str::<StockUpdate>(&text) {
                        if tx.send(update).await.is_err() {
                            eprintln!("Receiver dropped. Exiting WebSocket reader.");
                            break;
                        }
                    }
                }
                Message::Binary(bin) => {
                    if let Ok(update) = serde_json::from_slice::<StockUpdate>(&bin) {
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



use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new()?;

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        // Example: Trigger shutdown after <SOME> seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(
            config.websocket.shutdown_delay_ms
        )).await;
        let _ = shutdown_tx.send(true);
    });

    let tickers = vec!["AAPL".to_string(), "GOOGL".to_string(), "MSFT".to_string()];
    let stock_socket = StockSocket { config };

    tokio::select! {
        result = stock_socket.consume(&tickers) => {
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
