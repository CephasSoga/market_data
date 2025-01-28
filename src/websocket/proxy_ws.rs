use tokio::net::{TcpListener, TcpStream};
use futures_util::{StreamExt, SinkExt};
use thiserror::Error;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::protocol::Message};

pub struct WebSocketProxyServer;

impl WebSocketProxyServer {
    async fn handle_connection(stream: tokio::net::TcpStream) {
        // Accept the client WebSocket connection
        let ws_stream = accept_async(stream).await.expect("Error during the WebSocket handshake");
        let (mut client_sender, mut client_receiver) = ws_stream.split();

        // Connect to the target WebSocket server
        let target_uri = "ws://example.com"; // Replace with the target WebSocket server
        let (target_ws_stream, _) = connect_async(target_uri)
            .await
            .expect("Failed to connect to target WebSocket server");
        let (mut target_sender, mut target_receiver) = target_ws_stream.split();

        // Forward messages from the client to the target WebSocket server
        let client_to_target = tokio::spawn(async move {
            while let Some(message) = client_receiver.next().await {
                if let Ok(msg) = message {
                    if let Err(e) = target_sender.send(msg).await {
                        eprintln!("Failed to forward message to target: {}", e);
                        break;
                    }
                }
            }
        });

        // Forward messages from the target WebSocket server to the client
        let target_to_client = tokio::spawn(async move {
            while let Some(message) = target_receiver.next().await {
                if let Ok(msg) = message {
                    if let Err(e) = client_sender.send(msg).await {
                        eprintln!("Failed to forward message to client: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for both directions to complete
        let _ = tokio::join!(client_to_target, target_to_client);
    }

    pub async fn run() {
        let listener = TcpListener::bind("127.0.0.1:8765")
            .await
            .expect("Failed to bind to address");

        println!("WebSocket proxy server running on ws://127.0.0.1:8765");

        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(Self::handle_connection(stream));
        }
    }
}

#[tokio::main]
async fn main() {
    WebSocketProxyServer::run().await;
}
