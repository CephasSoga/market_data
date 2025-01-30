use async_tungstenite::tungstenite::buffer;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{StreamExt, SinkExt};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::{accept_async, connect_async};
use async_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::protocol::Message as ByteMessage;
use tokio_tungstenite::tungstenite::Message as SendMessage;
use tungstenite::Message as Message_;
use tungstenite::Error as Error_;
use async_tungstenite::tokio::{TokioAdapter}; //, connect_async};
//use async_tungstenite::WebSocketStream;
use tokio_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use async_tungstenite::stream::Stream;
use tokio::{sync::mpsc, sync::watch, sync::Mutex, time};
use futures_util::stream::{SplitStream, SplitSink};
use tokio_native_tls::TlsConnector;
use native_tls::TlsConnector as NativeTlsConnector;
use serde::{Serialize, Deserialize};
use tungstenite::Utf8Bytes;
use std::fmt::format;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{info, error, warn, debug};
use std::io::{Error, ErrorKind};
use tokio_native_tls::TlsStream;
use serde_json::{to_string, to_value};

use crate::config::Config;
use crate::logging::setup_logger;
use crate::options::FetchType;
use crate::request_parser::parser::CallParser;
use crate::request_parser::params::*;

#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("Failed to connect to WebSocket: {0}")]
    ConnectError(#[source] tungstenite::Error),

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
    RequestError(String),

    #[error("Other error: {0}")]
    LoginError(String),

    #[error("Other error: {0}")]
    SubscriptionError(String),
    

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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ErrorLog {
    error: String,
}
impl From<&WebSocketError> for ErrorLog {
    fn from(err: &WebSocketError) -> ErrorLog {
        ErrorLog {
            error: err.to_string()
        }
    }
}
impl ErrorLog {
    fn to_string(&self)-> String {
        to_string(&self).unwrap_or(r#"{error: null}"#.to_string())
    }

    fn to_utf8_bytes(&self) -> Utf8Bytes {
        Utf8Bytes::from(self.to_string())
    }
}


// SharedState for Batch Processing
type SharedState = Arc<Mutex<BatchState>>;

// reader and writer types
type WebSocketStreamType = WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>; 
type WebSocketWriter = SplitSink<WebSocketStream<TcpStream>, Message_>; 
type WebSocketReader = SplitStream<WebSocketStream<TcpStream>>;

pub trait WsHandler {
    async fn login(config: Arc<Config>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn subscribe(tickers: &Vec<String>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn connect_secure_websocket(uri: &str);
    async fn setup_stream_req(tickers: &Vec<String>, config: Arc<Config>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn handle_connection(stream: tokio::net::TcpStream, config: Arc<Config>) -> Result<(), WebSocketError>;
    fn parse_req_string(s: &str, config: Arc<Config>) -> Result<HandlerArgs, WebSocketError>;
    fn extract_relevant_args(task_args: TaskArgs) -> Result<(FetchType, Vec<String>), WebSocketError>;
    fn to_ut8bytes(update: Update) -> Result<SendMessage, WebSocketError>;
    fn update(&mut self, writer: WebSocketWriter);
    async fn run(&mut self);
}


#[derive(Debug, Clone)]
struct HandlerArgs {
    fetch_type: Option<FetchType>,
    target_uri: Option<String>,
    tickers: Option<Vec<String>>
}

enum  TargetUri {
    CryptoUri(String),
    ForexUri(String),
    StockUri(String),
}
impl TargetUri {
    fn from_fetch_type(fetch_type: FetchType, config: Arc<Config>) -> Option<String> {
        match  fetch_type {
            FetchType::Crypto => Some(config.websocket.crypto_ws.clone()),
            FetchType::Forex => Some(config.websocket.forex_ws.clone()),
            FetchType::Stock => Some(config.websocket.stock_ws.clone()),
            _ => None,
        }
    }
}

pub struct WebSocketProxyServer {
    config: Arc<Config>,
    uri: String,
    client_write: Option<WebSocketWriter>,
    client_read: Option<WebSocketReader>,
}
impl WebSocketProxyServer{
    pub fn new(
        config: Arc<Config>,
        uri: &str,
    )-> Self {
        Self {
            config,
            uri: uri.to_string(), 
            client_write: None,
            client_read: None
        }
    }
}
impl WsHandler for WebSocketProxyServer {
    async fn login(config: Arc<Config>, mut sender: &mut WebSocketWriter) -> Result<(), WebSocketError> {
        // Send login and subscription messages
        let login_message = serde_json::json!({
            "event": "login",
            "data": { "apiKey": config.api.token }
        });

        sender.send(Message_::Text(
                login_message.to_string().into()
            )).await
            .map_err(|err| {WebSocketError::ConnectError(err)})
    }

    async fn subscribe(tickers: &Vec<String>, mut sender: &mut WebSocketWriter) -> Result<(), WebSocketError>  {
        let subscribe_message = serde_json::json!({
            "event": "subscribe",
            "data": { "ticker": tickers }
        });

        sender.send(Message_::Text(
                subscribe_message.to_string().into()
            )).await
            .map_err(|err| {WebSocketError::ConnectError(err)})
    }

    fn parse_req_string(s: &str, config: Arc<Config>) -> Result<HandlerArgs, WebSocketError> {
        info!("Received client request. | Parsing...");
        let call_request = match CallParser::key_lookup_parse_json(s) {
            Ok(req) => req,
            Err(err) => {
                error!("Failed to parse client request. | JSON is invalid.");
                return Err(WebSocketError::RequestError(err))
            }
        };
    
        if call_request.target.to_str() == "task" {
            if let Some(task_args) = call_request.args.for_task {
                if let TaskFunction::RealTimeMarketData = task_args.function {
                    let (fetch_type_,  tickers) = Self::extract_relevant_args(task_args)?;
                    info!("Successfully parsed Client request.");
                    return  Ok(HandlerArgs {
                        fetch_type: Some(fetch_type_.clone()),
                        target_uri: TargetUri::from_fetch_type(fetch_type_, config),
                        tickers: Some(tickers), 
                    });

                }
            }
        }
        error!("Failed to parse client request. | Either `Target` or `Args` is invalid.");
        return Err(WebSocketError::RequestError(format!("The Json object request object could not be parsed. 
            | Lookup: {:?}", call_request.target)));
        
    }

    fn extract_relevant_args(task_args: TaskArgs) -> Result<(FetchType, Vec<String>), WebSocketError> {
        info!("Extracting Relevant args...");
        if let Some(args) = task_args.params {
            let args = to_value(args).unwrap();
            let fetch_type = args.get("fetch_type").and_then(|v| v.as_str())
                .map(|s| FetchType::from_str(s));

            let tickers: Option<Vec<String>> = args.get("tickers")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });

            if let (Some(f), Some(t)) = (fetch_type.clone(), tickers.clone()) {
                Ok((f, t))
            } else {
                Err(WebSocketError::RequestError(format!("Invalid fetch type or tickers. 
                | Fetch type: {:?}. | Tickers {:?}", &fetch_type, &tickers)))
            }
            
        } else {
            return Err(WebSocketError::RequestError("params can not be `null` for this request.".to_string()));
        }
    }

    fn to_ut8bytes(update: Update) -> Result<SendMessage, WebSocketError> {
        let string = to_string(&update)
            .map_err(|err| WebSocketError::DeserializationError(err))?;

        let bytes = Utf8Bytes::from(string);
        Ok(SendMessage::Text(bytes)) 
    }
    fn update(&mut self, writer: WebSocketWriter) {
        self.client_write = Some(writer)
    }

    async fn connect_secure_websocket(uri: &str) {
        let connector = TlsConnector::from(NativeTlsConnector::new().unwrap());
        let tcp_stream = TcpStream::connect(uri).await.unwrap();

        let tls_stream = connector.connect(uri, tcp_stream).await.unwrap();
        //let stream = connect_async(tls_stream.take(limit)).await.unwrap();
        println!("Connected to WebSocket!");
    }


    async fn handle_connection(stream: tokio::net::TcpStream, config: Arc<Config>) -> Result<(), WebSocketError> {
        // Accept the client WebSocket connection
        let ws_stream = accept_async(stream).await.expect("Error during the WebSocket handshake");
        let (mut client_sender, mut client_receiver) = ws_stream.split();

        let client_sender_ref = &mut client_sender;

        let mut handler = HandlerArgs {
            fetch_type: None,
            target_uri: None,
            tickers: None
        };
        
        // Parse the client's Json request
        if let Some(message) = client_receiver.next().await {
            match message {
                Ok(ByteMessage::Text(text)) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Ok(s) = to_string(&json) {
                            handler = Self::parse_req_string(&s, config.clone())?;
                        } else {
                            error!("Failed to convert to Json req to string. 
                                | JSON: {}", &json);
                            let err = WebSocketError::RequestError(format!("Failed to convert to Json req to string. 
                            | JSON: {:?}", &json));
                            client_sender_ref.send(
                                    Message_::Text(ErrorLog::from(&err).to_utf8_bytes())
                            );
                            return Err(err);
                        }
                        
                    } else {
                        error!("Failed to parse JSON message. | Message: {:?}", &text);
                        let err = WebSocketError::RequestError(format!("Failed to parse JSON message. 
                            | Message: {:?}", &text));
                        client_sender_ref.send(
                            Message_::Text(ErrorLog::from(&err).to_utf8_bytes())
                        );
                        return Err(err);
                    }
                }
                _ => {
                    error!("First message must be a JSON text payload");
                    let err = WebSocketError::RequestError("Invalid message type".to_string());
                    client_sender_ref.send(
                        Message_::Text(ErrorLog::from(&err).to_utf8_bytes())
                    );
                    return Err(err);
                }
            }
        }

        // Now, login and subscribe
        let  _ = Self::setup_stream_req(
            &handler.clone().tickers.unwrap_or(vec![]), 
            config.clone(), 
            client_sender_ref
        ).await?;
        
        // Connect to the target WebSocket server
        let target_uri = handler.clone().target_uri.ok_or_else({
            error!("Target uri is missimg. | Handler: {:?}", &handler.clone());
            || WebSocketError::RequestError("Target uri is missimg".to_string())
        })?;
        let (target_ws_stream, _) = connect_async(&target_uri)
            .await
            .expect("Failed to connect to target WebSocket server");
        let (mut target_sender, mut target_receiver) = target_ws_stream.split();

        // Forward messages from the client to the target WebSocket server
        let client_to_target: tokio::task::JoinHandle<Result<(), WebSocketError>> = tokio::spawn(async move {
            while let Some(message) = client_receiver.next().await {
                match message {
                    Ok(ByteMessage::Text(text)) => {
                        debug!("Text message received: {}", text);

                        let update = serde_json::from_str::<Update>(&text.to_string())
                            .map_err(|err| WebSocketError::DeserializationError(err))?;
                        if let Err(e) = target_sender.send(Self::to_ut8bytes(update)?).await {
                            error!("Failed to forward message to target: {}", e);
                            break;
                        }
                    },
                    Ok(ByteMessage::Binary(bin)) => {
                        debug!("Binary message received: {:?}", &bin);
                        let update = serde_json::from_slice::<Update>(&bin.to_vec())
                            .map_err(|err| WebSocketError::DeserializationError(err))?;
                        if let Err(e) = target_sender.send(Self::to_ut8bytes(update)?).await {
                            error!("Failed to forward message to target: {}", e);
                            break;
                        }
                    },
                    Ok(_) => {
                        debug!("No message is received.");
                    }, 
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                    }
                }
            }
            Ok(())
        });

        // Forward messages from the target WebSocket server to the client
        let target_to_client = tokio::spawn(async move {
            while let Some(message) = target_receiver.next().await {
                if let Ok(msg) = message {
                    if let Err(e) = client_sender.send(msg).await {
                        error!("Failed to forward message to client: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for both directions to complete
        let _ = tokio::join!(client_to_target, target_to_client);
        Ok(())
    }

    async fn setup_stream_req(tickers: &Vec<String>, config: Arc<Config>, mut writer: &mut WebSocketWriter) -> Result<(), WebSocketError> {
        info!("Proxy Ws: Loging in FMP websocket...");
        let _ = Self::login(config, writer).await?;
        
        info!("Proxy Ws: Subscribing to tickers streams. | Tickers: {:?}.", tickers);
        let _ = Self::subscribe(&tickers, writer).await?;
        Ok(())
    }

    async fn run(&mut self) {
        let _ = setup_logger("info");
        let listener = TcpListener::bind(&self.uri)
            .await
            .expect("Failed to bind to address");

        info!("WebSocket proxy server running on ws://{}", &self.uri);

        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(Self::handle_connection(stream, self.config.clone()));
        }
    }
}
