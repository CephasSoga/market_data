

use std::sync::Arc;
use std::ops::Deref;

use rustls::internal::msgs::message;
use tokio::net::{TcpStream, TcpListener};
use async_tungstenite::tokio::{TokioAdapter, connect_async, accept_async};
use async_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use async_tungstenite::tungstenite::protocol::{CloseFrame, frame::coding::CloseCode};
use async_tungstenite::stream::Stream;
use tokio::{sync::mpsc, sync::watch, sync::Mutex, time};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitStream, SplitSink};
use serde::{Serialize, Deserialize};
use serde_json::{to_string, to_value};
use tracing::{info, error};
use thiserror::Error;
use std::io::{Error, ErrorKind};
use tokio_native_tls::TlsStream;

use crate::config::Config;
use crate::logging::setup_logger;
use crate::options::FetchType;
use crate::request_parser::parser::CallParser;
use crate::request_parser::params::*;


#[derive(Error, Debug)]
pub enum WebSocketError {
    #[error("Failed to connect to WebSocket: {0}")]
    ConnectError(#[source] async_tungstenite::tungstenite::Error),

    #[error("Failed to send WebSocket message: {0}")]
    SendError(#[source] tokio::io::Error),

    #[error("Failed to receive WebSocket message: {0}")]
    ReceiveError(#[source] tokio::io::Error),

    #[error("Failed to serialize JSON message: {0}")]
    SerializationError(#[source] serde_json::Error),

    #[error("Failed to deserialize JSON message: {0}")]
    DeserializationError(#[source] serde_json::Error),


    #[error("Channel send error: {0}")]
    ChannelSendError(async_tungstenite::tungstenite::Error),

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
}

// reader and writer types
type WebSocketStreamType = WebSocketStream<Stream<TokioAdapter<TcpStream>, TokioAdapter<TlsStream<TcpStream>>>>; 
type WebSocketWriter = SplitSink<WebSocketStreamType, Message>; 
type WebSocketReader = SplitStream<WebSocketStreamType>;
type WebSocketTask = tokio::task::JoinHandle<Result<(), WebSocketError>>;
type ClientSender = SplitSink<WebSocketStream<TokioAdapter<TcpStream>>, Message>;
type ClientReceiver = SplitStream<WebSocketStream<TokioAdapter<TcpStream>>>;
type Handles = (HandlerArgs, ClientSender, ClientReceiver);

pub trait WsHandler {
    async fn login(config: Arc<Config>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn subscribe(tickers: &Vec<String>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn setup_stream_req(tickers: &Vec<String>, config: Arc<Config>, sender: &mut WebSocketWriter) -> Result<(), WebSocketError>;
    async fn make_handler(stream: TcpStream, config: Arc<Config>) -> Result<Handles, WebSocketError>;
    async fn handle_connection(stream: TcpStream, config: Arc<Config>) -> Result<(), WebSocketError>;
    async fn run(&mut self) -> Result<(), WebSocketError>;
}

pub struct WebSocketProxyServer {
    config: Arc<Config>,
    uri: String,
}
impl WebSocketProxyServer{
    pub fn new(config: Arc<Config>, uri: &str)-> Self {
        Self {config, uri: uri.to_string()}
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

}
impl WsHandler for WebSocketProxyServer {
    async fn login(config: Arc<Config>, mut sender: &mut WebSocketWriter) -> Result<(), WebSocketError> {
        let login_message = serde_json::json!({"event": "login","data": { "apiKey": config.api.token }})
            .to_string();
        sender.send(Message::Text(login_message)).await
            .map_err(|err| {
                error!("Login failed.");
                WebSocketError::ConnectError(err)
            })
    }

    async fn subscribe(tickers: &Vec<String>, mut sender: &mut WebSocketWriter) -> Result<(), WebSocketError> {
        let subscribe_message = serde_json::json!({
            "event": "subscribe",
            "data": { "ticker": tickers }
        });
        sender.send(Message::Text(subscribe_message.to_string())).await
            .map_err(|err| {
                error!("Subscription failed.");
                WebSocketError::ConnectError(err)
            })
    }

    async fn setup_stream_req(tickers: &Vec<String>, config: Arc<Config>, mut sender: &mut WebSocketWriter) -> Result<(), WebSocketError> {
        info!("Proxy Ws: Loging in FMP websocket...");
        let _ = Self::login(config, sender).await?;
        
        info!("Proxy Ws: Subscribing to tickers streams. | Tickers: {:?}.", tickers);
        let _ = Self::subscribe(&tickers, sender).await?;
        Ok(())
    }

    async fn make_handler(stream: TcpStream, config: Arc<Config>) -> Result<Handles, WebSocketError> {
        // Accept the client WebSocket connection.
        let ws_stream = accept_async(stream).await
            .map_err(|err| {
                error!("Unable to accept stream. | Error: {}.", err);
                WebSocketError::ConnectError(err)
            })?;
        //  Split the ws_stream
        let (mut sender, mut receiver) = ws_stream.split();
        // Create a ref. to the sender to use accross spawn tasks.
        let sender_ref = &mut sender;
        // Initialize a HandlerArgs instnace for req redirection
        let mut handler = HandlerArgs {fetch_type: None, target_uri:  None, tickers: None};
        // Parse the client's Json
        if let Some(message) = receiver.next().await {
            info!("Client message received. | Processing...");
            match message {
                Ok(inner) => {
                    match inner {
                        Message::Text(text) => {
                            handler = Self::parse_req_string(&text, config)?;
                        },
                        _  => {
                            error!("Client request should be a JSON payload. | Message: {:?}", &inner);
                            let err = WebSocketError::RequestError(format!("Failed to parse JSON message. 
                                | Message: {:?}", &inner));
                            sender_ref.send(Message::Text(ErrorLog::from(&err).to_string()));
                            return Err(err);
                        }
                        
                    }
                },
                Err(err) => {
                    error!("Failed to receive client's message. | Error: {}", err);
                    return Err(WebSocketError::ConnectError(err))
                }
            }
        };
        Ok((handler, sender, receiver))
    }

    async fn handle_connection(stream: TcpStream, config: Arc<Config>) -> Result<(), WebSocketError> {
        // Clone config & Build handler
        let cfg = config.clone();
        let (handler, mut client_sender, mut client_receiver) =  Self::make_handler(stream, cfg).await?;
        // Etract target uri from handler
        let target_uri = handler.clone().target_uri.ok_or_else({
            error!("Target uri is missimg. | Handler: {:?}", &handler.clone());
            || WebSocketError::RequestError("Target uri is missimg".to_string())
        })?;
        //Connect to target ws server
        let (stream, _) = connect_async(&target_uri).await
            .map_err(|err| {
                error!("Unable to connect to target server: {}. | Error: {}", &target_uri, &err);
                WebSocketError::ConnectError(err)
            })?;
        // Split the stream
        let (mut sender, mut receiver) = stream.split();
        // Create a mutable ref. to sender
        let sender_ref = &mut sender;
        //Now login in and subscribe
        let _setup = Self::setup_stream_req(
            &handler.clone().tickers.unwrap_or(vec![]),
            config.clone(), 
            sender_ref).await?;
        // Accept mssages from client & transfer to target [Client -> Target]
        let accept: WebSocketTask = tokio::spawn(async move {
            while let Some(message) =  client_receiver.next().await {
                match message {
                    Ok(inner) => {
                        match inner {
                            Message::Text(text) => {
                                info!("Client message is  <Utf8bytes> type. | Parsing...");
                                let update = serde_json::from_str::<Update>(&text)
                                    .map_err(|err| WebSocketError::DeserializationError(err))?;
                                let send_msg = Message::Text(
                                    to_string(&update).map_err(|err| WebSocketError::SerializationError(err))?
                                );
                                sender.send(send_msg).await.map_err(|err| WebSocketError::ChannelSendError(err))?
                            },
                            Message::Binary(bin) => {
                                info!("Client message is  <Byteschunk> type. | Parsing...");
                                let update = serde_json::from_slice::<Update>(&bin)
                                    .map_err(|err| WebSocketError::DeserializationError(err))?;
                                let send_msg = Message::Text(
                                    to_string(&update).map_err(|err| WebSocketError::SerializationError(err))?
                                );
                                sender.send(send_msg).await.map_err(|err| WebSocketError::ChannelSendError(err))?
                            },
                            _  => {
                                info!("Message is one of [<Close_>, <Frame_>, <Ping_>, <Pong_>]. Message will not be processed further.")
                            }
                            
                        }
                    },
                    Err(err) => {
                        error!("Failed to receive client's message. | Error: {}", err);
                        return Err(WebSocketError::ConnectError(err));
                    }
                }
            }
            Ok(())
        });
        // Forward messages from the target WebSocket server to the client [Target -> Client]
        let foward = tokio::spawn(async move{
            while let Some(message) = receiver.next().await {
                match message {
                    Ok(msg) => {
                        info!("Fowarding message to client...");
                        if let Err(e) = client_sender.send(msg).await {
                            error!("Failed to forward message to client. | Error: {}.", e);
                            break;
                        }
                    },
                    Err(err) => {
                        error!("Invalid message. Failed to forward message to client. | Error: {}", &err);
                        break;
                    }
                }
            }
        });
        // Wait for both directions to complete
        let _tasks = tokio::join!(accept, foward);
        Ok(())
    }

    async fn run(&mut self) -> Result<(), WebSocketError> {
        let _logger = setup_logger("info");
        let listener = TcpListener::bind(&self.uri)
            .await
            .expect("Failed to bind to address");

        info!("WebSocket proxy server running on ws://{}", &self.uri);

        let _op = while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(Self::handle_connection(stream, self.config.clone()));
        };

        Ok(())
    }
}
