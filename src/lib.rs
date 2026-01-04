//! Game State Integration with Dota 2.
//!
//! Provides a [`Server`] that listens for JSON events sent by Dota 2. Enabling game state
//! integration requires:
//! 1. Creating a `.cfg` [configuration file] in the Dota 2 game configuration directory.
//! 2. Running Dota 2 with the -gamestateintegration [launch option].
//!
//! The configuration file can have any name name, but must be prefixed by `gamestate_integration_`.
//! For example, `gamestate_integration_test.cfg` would be located:
//! * In Linux: `~/.steam/steam/steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration_test.cfg`
//! * In Windows: `D:\Steam\steamapps\common\dota 2 beta\csgo\cfg\gamestate_integration_test.cfg`
//!
//! Here's a sample configuration file:
//!
//! "dota2-gsi Configuration"
//!{
//!    "uri"               "http://127.0.0.1:53000/"
//!    "timeout"           "5.0"
//!    "buffer"            "0.1"
//!    "throttle"          "0.1"
//!    "heartbeat"         "30.0"
//!    "data"
//!    {
//!        "buildings"     "1"
//!        "provider"      "1"
//!        "map"           "1"
//!        "player"        "1"
//!        "hero"          "1"
//!        "abilities"     "1"
//!        "items"         "1"
//!        "draft"         "1"
//!        "wearables"     "1"
//!    }
//!    "auth"
//!    {
//!        "token"         "abcdefghijklmopqrstuvxyz123456789"
//!    }
//!}
//!
//! Notice that the URI used in the configuration file must be the same URI used when
//! initializing a [`Server`].
//!
//! [^configuration file]: Details on configuration file: https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration
//! [^launch option]: Available launch options: https://help.steampowered.com/en/faqs/view/7d01-d2dd-d75e-2955
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task;

pub mod components;

/// The payload sent on every game state integration request is usually between 50-60kb.
/// We initialize a buffer to read the request with this initial capacity.
/// The code then looks at the Content-Length header to reserve the required capacity.
const INITIAL_REQUEST_BUFFER_CAPACITY_BYTES: usize = 1024;

/// The POST game state integration request includes this number of headers.
/// We parse them to find the Content-Length.
const EXPECTED_NUMBER_OF_HEADERS: usize = 7;

/// The response expected for every game state integration request.
/// Failure to deliver this response would cause the request to be retried infinitely.
const OK: &str = "HTTP/1.1 200 OK\ncontent-type: text/html\n";

#[derive(Error, Debug)]
pub enum GSIServerError {
    #[error("incomplete headers have been parsed from GSI request")]
    IncompleteHeaders,
    #[error("failed to read (write) from (to) socket listening to GSI")]
    SocketError(#[from] io::Error),
    #[error("socket was closed")]
    SocketClosed,
    #[error("failed to complete the assigned GSI task")]
    TaskError(#[from] task::JoinError),
    #[error("failed to parse game state integration from JSON")]
    ParseJSONError(#[from] serde_json::Error),
    #[error("failed to parse Content-Length Header sent by Dota: {0}")]
    ParseContentLengthError(String),
    #[error("failed to parse Request sent by Dota")]
    ParseRequestError(#[from] httparse::Error),
}

/// Trait implemented by handlers of Game State data.
#[async_trait]
pub trait GameStateHandler<D>
where
    D: DeserializeOwned + std::fmt::Debug + Send,
{
    async fn handle(self, gs: D);
}

#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn handle(&self, event: bytes::Bytes);
}

#[async_trait]
impl<F, Fut> Handler for F
where
    F: Fn(bytes::Bytes) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send,
{
    async fn handle(&self, event: bytes::Bytes) {
        (self)(event).await;
    }
}

/// A server that handles game state integration requests.
pub struct Server {
    uri: String,
    handlers: Vec<Arc<dyn Handler>>,
}

impl Server {
    /// Create a new Server with given URI.
    ///
    /// The provided URI must match the one used when configuring the game state integration.
    pub fn new(uri: &str) -> Self {
        Server {
            uri: uri.to_owned(),
            handlers: Vec::new(),
        }
    }

    /// Register a new handler on this Server.
    ///
    /// The Server will handle all incoming data from game state integration using all registered handlers.
    pub fn register<H>(mut self, handler: H) -> Self
    where
        H: Handler,
    {
        self.handlers.push(Arc::new(handler));
        self
    }

    /// Run the game state integration server.
    pub async fn run(self) -> Result<(), GSIServerError> {
        let listener = TcpListener::bind(&self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        let (sender, _) = broadcast::channel(16);
        self.handle_broadcast(&sender);

        loop {
            let (socket, addr) = listener.accept().await?;

            let sender = sender.clone();

            tokio::spawn(async move {
                match process(socket).await {
                    Err(e) => {
                        log::error!("{}", e);
                        return Err(e);
                    }
                    Ok(buf) => {
                        sender.send(buf).unwrap();
                    }
                };

                Ok(())
            });
        }
    }

    fn handle_broadcast(&self, broadcast: &broadcast::Sender<bytes::Bytes>) {
        for handler in &self.handlers {
            let handler = Arc::clone(handler);
            let mut receiver = broadcast.subscribe();

            tokio::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(data) => {
                            handler.handle(data).await;
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            });
        }
    }
}

/// A server that handles GameState Integration requests from Dota.
/// The URI used in the configuration file must be the same URI used when creating a new [`GSIServer`].
pub struct GSIServer {
    uri: String,
}

impl Default for GSIServer {
    fn default() -> Self {
        GSIServer {
            uri: "127.0.0.1:3000".to_owned(),
        }
    }
}

impl GSIServer {
    /// Create a new GSIServer with given URI.
    pub fn new(uri: &str) -> Self {
        GSIServer {
            uri: uri.to_owned(),
        }
    }

    /// Run the Game State Integration server.
    /// A handler function is taken to process the data sent by Dota 2.
    pub async fn run<D, U>(
        self,
        handler: impl Fn(D) -> U + Sync + Send + Copy + 'static,
    ) -> Result<(), GSIServerError>
    where
        D: DeserializeOwned + std::fmt::Debug + Send,
        U: Future + Send + Sync,
        U::Output: Send,
    {
        let listener = TcpListener::bind(self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        loop {
            let (socket, addr) = listener.accept().await?;
            log::info!("Accepted: {}", addr);

            tokio::spawn(async move {
                log::debug!("Task spawned");

                match process(socket).await {
                    Err(e) => {
                        log::error!("{}", e);
                        return Err(e);
                    }
                    Ok(buf) => match serde_json::from_slice(&buf) {
                        Err(e) => {
                            log::debug!("{:?}", buf);
                            log::error!("Failed to parse JSON body: {}", e);
                            return Err(GSIServerError::from(e));
                        }
                        Ok(parsed) => {
                            handler(parsed).await;
                        }
                    },
                };

                Ok(())
            });
        }
    }

    /// Run the Game State Integration server.
    /// A handler function is taken to process the data sent by Dota 2.
    pub async fn run_with_handler<D>(
        self,
        handler: impl GameStateHandler<D> + Send + Sync + Clone + 'static,
    ) -> Result<(), GSIServerError>
    where
        D: DeserializeOwned + std::fmt::Debug + Send,
    {
        let listener = TcpListener::bind(self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        loop {
            let (socket, addr) = listener.accept().await?;
            log::info!("Accepted: {}", addr);
            // Need to clone as handler will be moved by spawn.
            let this_handler = handler.clone();

            tokio::spawn(async move {
                log::debug!("Task spawned");

                match process(socket).await {
                    Err(e) => {
                        log::error!("{}", e);
                        return Err(e);
                    }
                    Ok(buf) => match serde_json::from_slice(&buf) {
                        Err(e) => {
                            log::error!("Failed to parse JSON body: {}", e);
                            return Err(GSIServerError::from(e));
                        }
                        Ok(parsed) => {
                            this_handler.handle(parsed).await;
                        }
                    },
                };

                Ok(())
            });
        }
    }
}

/// Process a game state integration request.
///
/// This function parses the request and reads the entire payload, returning it as a
/// [`bytes::Bytes`].
pub async fn process(mut socket: TcpStream) -> Result<bytes::Bytes, GSIServerError> {
    if let Err(e) = socket.readable().await {
        log::error!("socket is not readable");
        return Err(GSIServerError::from(e));
    };

    let mut buf = bytes::BytesMut::with_capacity(INITIAL_REQUEST_BUFFER_CAPACITY_BYTES);
    let request_length: usize;
    let content_length: usize;

    loop {
        match socket.read_buf(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("failed to read request from socket: {}", e);
                return Err(GSIServerError::from(e));
            }
        };

        let mut headers = [httparse::EMPTY_HEADER; EXPECTED_NUMBER_OF_HEADERS];
        let mut r = httparse::Request::new(&mut headers);

        request_length = match r.parse(&buf) {
            Ok(httparse::Status::Complete(size)) => size,
            Ok(httparse::Status::Partial) => {
                log::debug!("partial request parsed, need to read more");
                continue;
            }
            Err(e) => {
                log::error!("failed to parse request: {}", e);
                return Err(GSIServerError::from(e));
            }
        };
        log::debug!("headers: {:?}", headers);
        content_length = get_content_length_from_headers(&headers)?;
        break;
    }

    if buf.len() <= request_length + content_length {
        buf.reserve(request_length + content_length);
        match socket.read_buf(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("failed to read body from socket: {}", e);
                return Err(GSIServerError::from(e));
            }
        };
    }

    if let Err(e) = socket.write_all(OK.as_bytes()).await {
        log::error!("failed to write to socket: {}", e);
        return Err(GSIServerError::from(e));
    };

    Ok(buf.split_off(request_length).freeze())
}

/// Extract Content-Length value from a list of HTTP headers.
pub fn get_content_length_from_headers(
    headers: &[httparse::Header],
) -> Result<usize, GSIServerError> {
    match headers
        .iter()
        .filter(|h| h.name == "Content-Length")
        .map(|h| h.value)
        .next()
    {
        Some(value) => {
            let str_length = match std::str::from_utf8(value) {
                Ok(s) => s,
                Err(e) => {
                    return Err(GSIServerError::ParseContentLengthError(format!(
                        "failed to parse bytes as str: {}",
                        e
                    )));
                }
            };
            match str_length.parse::<usize>() {
                Ok(n) => Ok(n),
                Err(e) => Err(GSIServerError::ParseContentLengthError(format!(
                    "failed to parse str into usize: {}",
                    e
                ))),
            }
        }
        None => Err(GSIServerError::ParseContentLengthError(
            "Content-Length header not found".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net;
    use std::time;
    use tokio::sync::mpsc;
    use tokio::time::{sleep, timeout};

    const TEST_URI: &'static str = "127.0.0.1:10080";

    #[test]
    fn test_get_content_length_from_headers() {
        let mut headers = [httparse::EMPTY_HEADER; EXPECTED_NUMBER_OF_HEADERS];
        let mut r = httparse::Request::new(&mut headers);
        let request_bytes = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:3000\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\nContent-Length: 54943\r\n\r\n";
        r.parse(request_bytes)
            .expect("parsing the request should never fail");

        let expected = 54943 as usize;
        let content_length =
            get_content_length_from_headers(&r.headers).expect("failed to get Content-Length");

        assert_eq!(content_length, expected);
    }

    #[test]
    fn test_get_content_length_from_headers_not_found() {
        let mut headers = [httparse::EMPTY_HEADER; EXPECTED_NUMBER_OF_HEADERS];
        let mut r = httparse::Request::new(&mut headers);
        let request_bytes = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:3000\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\n\r\n";
        r.parse(request_bytes)
            .expect("parsing the request should never fail");

        let content_length = get_content_length_from_headers(&r.headers);

        assert!(matches!(
            content_length,
            Err(GSIServerError::ParseContentLengthError(_))
        ));
    }

    #[test]
    fn test_get_content_length_from_headers_not_a_number() {
        let mut headers = [httparse::EMPTY_HEADER; EXPECTED_NUMBER_OF_HEADERS];
        let mut r = httparse::Request::new(&mut headers);
        let request_bytes = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:3000\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\nContent-Length: asdasd\r\n\r\n";
        r.parse(request_bytes)
            .expect("parsing the request should never fail");

        let content_length = get_content_length_from_headers(&r.headers);

        assert!(matches!(
            content_length,
            Err(GSIServerError::ParseContentLengthError(_))
        ));
    }

    #[tokio::test]
    async fn test_process() {
        let listener = TcpListener::bind(TEST_URI)
            .await
            .expect("failed to bind to address");
        let local_addr = listener.local_addr().unwrap();
        let sample_request = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:3000\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\nContent-Length: 173\r\n\r\n{\n\t\"provider\": {\n\t\t\"name\": \"Dota 2\",\n\t\t\"appid\": 570,\n\t\t\"version\": 47,\n\t\t\"timestamp\": 1688514013\n\t},\n\t\"player\": {\n\n\t},\n\t\"draft\": {\n\n\t},\n\t\"auth\": {\n\t\t\"token\": \"hello1234\"\n\t}\n}";
        let expected = b"{\n\t\"provider\": {\n\t\t\"name\": \"Dota 2\",\n\t\t\"appid\": 570,\n\t\t\"version\": 47,\n\t\t\"timestamp\": 1688514013\n\t},\n\t\"player\": {\n\n\t},\n\t\"draft\": {\n\n\t},\n\t\"auth\": {\n\t\t\"token\": \"hello1234\"\n\t}\n}";

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = stream.write_all(sample_request).await;
                let _ = stream.shutdown().await;
            }
        });

        let stream = TcpStream::connect(local_addr)
            .await
            .expect("failed to connect to address");

        let result = process(stream).await.expect("processing failed");
        assert_eq!(result.len(), expected.len());
        assert_eq!(result.as_ref(), expected);
    }

    #[tokio::test]
    async fn test_server_handles_events() {
        let sample_request = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:20080\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\nContent-Length: 173\r\n\r\n{\n\t\"provider\": {\n\t\t\"name\": \"Dota 2\",\n\t\t\"appid\": 570,\n\t\t\"version\": 47,\n\t\t\"timestamp\": 1688514013\n\t},\n\t\"player\": {\n\n\t},\n\t\"draft\": {\n\n\t},\n\t\"auth\": {\n\t\t\"token\": \"hello1234\"\n\t}\n}";
        let expected = bytes::Bytes::from_static(b"{\n\t\"provider\": {\n\t\t\"name\": \"Dota 2\",\n\t\t\"appid\": 570,\n\t\t\"version\": 47,\n\t\t\"timestamp\": 1688514013\n\t},\n\t\"player\": {\n\n\t},\n\t\"draft\": {\n\n\t},\n\t\"auth\": {\n\t\t\"token\": \"hello1234\"\n\t}\n}");

        let (tx1, mut rx1) = mpsc::channel(1);
        let (tx2, mut rx2) = mpsc::channel(1);

        tokio::spawn(async move {
            Server::new("127.0.0.1:20080")
                .register(move |event| {
                    let tx1 = tx1.clone();
                    println!("sending on 1");
                    async move {
                        let _ = &tx1.send(event).await.unwrap();
                        println!("sent on 1");
                    }
                })
                .register(move |event| {
                    let tx2 = tx2.clone();
                    println!("sending on 2");
                    async move {
                        let _ = &tx2.send(event).await.unwrap();
                        println!("sent on 2");
                    }
                })
                .run()
                .await
                .unwrap();
        });

        tokio::spawn(async move {
            for _ in 0..2 {
                let mut stream = TcpStream::connect("127.0.0.1:20080").await.unwrap();
                let _ = stream.write_all(sample_request).await;
                let _ = stream.shutdown().await;
            }
        });

        let mut v1 = Vec::new();
        let mut v2 = Vec::new();

        async fn capture(rx: &mut mpsc::Receiver<bytes::Bytes>, v: &mut Vec<bytes::Bytes>) {
            v.push(rx.recv().await.unwrap());
        }

        if let Err(_) = timeout(time::Duration::from_secs(5), async {
            tokio::join!(capture(&mut rx1, &mut v1), capture(&mut rx2, &mut v2));
            tokio::join!(capture(&mut rx1, &mut v1), capture(&mut rx2, &mut v2));
        })
        .await
        {
            println!("did not receive values within 5 seconds");
        }

        assert_eq!(v1.len(), 2);
        assert_eq!(v2.len(), 2);
        assert_eq!(v1[0], &expected);
        assert_eq!(v1[1], &expected);
        assert_eq!(v2[0], &expected);
        assert_eq!(v2[1], &expected);
    }
}
