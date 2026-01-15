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
//! * In Windows: `D:\Steam\steamapps\common\dota 2 beta\dota\cfg\gamestate_integration_test.cfg`
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
//! Note the URI used in the configuration file must be the same URI used with a [`ServerBuilder`].
//!
//! [^configuration file]: Details on configuration file: https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration
//! [^launch option]: Available launch options: https://help.steampowered.com/en/faqs/view/7d01-d2dd-d75e-2955
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::future::Future;
use std::io;

use async_trait::async_trait;
use futures::{StreamExt, stream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task;

#[cfg(feature = "models")]
pub mod components;
#[cfg(feature = "handlers")]
pub mod handlers;

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

#[derive(thiserror::Error, Debug)]
pub enum GameStateIntegrationError {
    #[error("incomplete headers from game state integration request")]
    IncompleteHeaders,
    #[error("failed to read from socket")]
    SocketRead(#[from] io::Error),
    #[error("no handlers available to process request, is the server shutting down?")]
    NoHandlersAvailable,
    #[error("invalid content length header: {0}")]
    InvalidContentLength(String),
    #[error("missing Content-Length header in request")]
    MissingContentLengthHeader,
    #[error("invalid request received")]
    InvalidRequest(#[from] httparse::Error),
    #[error("server has already shutdown")]
    ServerShutdown,
    #[error("handler failed when handling event")]
    Handler {
        #[source]
        source: anyhow::Error,
    },
    #[error("an error occurred while running the server")]
    Unknown(#[from] task::JoinError),
}

pub type HandlerResult = Result<(), anyhow::Error>;

/// Trait for any async function or struct that can be used to handle game state integration events.
///
/// This trait is automatically implemented for async functions and closures.
#[async_trait]
pub trait Handler: Send + Sync + 'static {
    async fn handle(&self, event: bytes::Bytes) -> HandlerResult;
}

#[async_trait]
impl<F, Fut, E> Handler for F
where
    F: Fn(bytes::Bytes) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), E>> + Send,
    E: Into<anyhow::Error>,
{
    async fn handle(&self, event: bytes::Bytes) -> HandlerResult {
        (self)(event).await.map_err(|e| e.into())?;
        Ok(())
    }
}

/// Manage lifecycle of a handler registered in a server
pub(crate) struct HandlerRegistration {
    inner: Box<dyn Handler>,
    is_shutdown: bool,
    notify: broadcast::Receiver<()>,
    events: broadcast::Receiver<bytes::Bytes>,
}

impl HandlerRegistration {
    pub(crate) fn new<H>(
        handler: H,
        notify: broadcast::Receiver<()>,
        events: broadcast::Receiver<bytes::Bytes>,
    ) -> Self
    where
        H: Handler,
    {
        Self {
            inner: Box::new(handler),
            is_shutdown: false,
            notify,
            events,
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), GameStateIntegrationError> {
        loop {
            tokio::select! {
                received = self.events.recv() => {
                    match received {
                        Ok(event) => {
                            if let Err(e) = self.inner.handle(event).await {
                                return Err(GameStateIntegrationError::Handler{source: e});
                            };
                        },
                        Err(_) => {break;}
                    }
                }
                _ = self.notify.recv() => {
                    break;
                }
            }
        }

        self.is_shutdown = true;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }
}

/// Manage lifecycle of a server's listening task
pub(crate) struct Listener {
    uri: String,
    is_shutdown: bool,
    notify: broadcast::Receiver<()>,
    send_events: broadcast::Sender<bytes::Bytes>,
}

impl Listener {
    pub(crate) fn new(
        uri: &str,
        notify: broadcast::Receiver<()>,
        send_events: broadcast::Sender<bytes::Bytes>,
    ) -> Self {
        Self {
            uri: uri.to_owned(),
            is_shutdown: false,
            notify,
            send_events,
        }
    }

    pub(crate) async fn run(mut self) -> Result<(), GameStateIntegrationError> {
        let listener = TcpListener::bind(&self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let (socket, _) = match accepted {
                        Ok(val) => val,
                        Err(e) => {
                            self.is_shutdown = true;
                            return Err(GameStateIntegrationError::SocketRead(e));
                        }
                    };

                    if self.send_events.receiver_count() == 0 {
                        // terminate if no handlers available
                        return Err(GameStateIntegrationError::NoHandlersAvailable);
                    }

                    let sender = self.send_events.clone();

                    tokio::spawn(async move {
                        match process(socket).await {
                            Err(e) => {
                                log::error!("{}", e);
                                Err(e)
                            }
                            Ok(buf) => match sender.send(buf) {
                                Ok(_) => Ok(()),
                                Err(_) => {
                                    // send can only fail if there are no active receivers
                                    // meaning no where registered or the server is shutting down.
                                    Err(GameStateIntegrationError::NoHandlersAvailable)
                                }
                            },
                        }
                    });

                }
                _ = self.notify.recv() => {
                    break;
                }
            }
        }

        self.is_shutdown = true;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }
}

#[derive(Debug)]
pub struct ServerError {
    listener_error: Option<GameStateIntegrationError>,
    handler_errors: Option<Vec<GameStateIntegrationError>>,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "There were one or more errors while running the server")?;

        if let Some(e) = self.listener_error.as_ref() {
            writeln!(f)?;
            write!(f, "- {:#}", e)?;
        }

        if let Some(errors) = self.handler_errors.as_ref() {
            for e in errors {
                writeln!(f)?;
                write!(f, "- {:#}", e)?;
            }
        }

        Ok(())
    }
}

impl StdError for ServerError {}

/// A [`Server`] that handles game state integration requests.
///
/// The [`Server`] spawns a task per registered handler to handle events incoming from the game state integration.
/// On server shutdown, any pending tasks are canceled. A separate listener task is spawned to actually listen
/// for game state integration requests on the configured URI, process them to extract the payload, and broadcast
/// each event to all registered handlers.
pub struct Server {
    listener: Option<task::JoinHandle<Result<(), GameStateIntegrationError>>>,
    handlers: Vec<task::JoinHandle<Result<(), GameStateIntegrationError>>>,
    notify_shutdown: broadcast::Sender<()>,
    is_shutdown: bool,
}

impl Server {
    pub fn new(
        listener: task::JoinHandle<Result<(), GameStateIntegrationError>>,
        handlers: impl IntoIterator<Item = task::JoinHandle<Result<(), GameStateIntegrationError>>>,
        notify_shutdown: broadcast::Sender<()>,
    ) -> Self {
        Self {
            listener: Some(listener),
            handlers: handlers.into_iter().collect(),
            notify_shutdown,
            is_shutdown: false,
        }
    }

    pub async fn run_forever(&self) {
        let _ = self.notify_shutdown.subscribe().recv().await;
    }

    /// Shutdown the server.
    pub async fn shutdown(&mut self) -> Result<(), ServerError> {
        let _ = self.notify_shutdown.send(());

        let listener_result = if let Some(listener) = self.listener.take() {
            match listener.await {
                Ok(r) => r,
                Err(e) => Err(GameStateIntegrationError::Unknown(e)),
            }
        } else {
            Ok(())
        };

        let mut handler_errors: Vec<GameStateIntegrationError> = Vec::new();
        let mut futures: stream::FuturesUnordered<_> = self.handlers.drain(..).collect();
        while let Some(result) = futures.next().await {
            match result {
                Ok(Err(e)) => handler_errors.push(e),
                Err(e) => handler_errors.push(GameStateIntegrationError::from(e)),
                Ok(Ok(())) => {}
            }
        }

        self.is_shutdown = true;

        match (listener_result, handler_errors.len()) {
            (Ok(()), 0) => Ok(()),
            (Err(e), 0) => Err(ServerError {
                listener_error: Some(e),
                handler_errors: None,
            }),
            (Ok(()), _) => Err(ServerError {
                listener_error: None,
                handler_errors: Some(handler_errors),
            }),
            (Err(e), _) => Err(ServerError {
                listener_error: Some(e),
                handler_errors: Some(handler_errors),
            }),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }
}

pub struct ServerBuilder {
    uri: String,
    handlers: Vec<HandlerRegistration>,
    notify_shutdown: broadcast::Sender<()>,
    send_events: broadcast::Sender<bytes::Bytes>,
    is_shutdown: bool,
}

impl ServerBuilder {
    /// Create a new Server with given URI.
    ///
    /// The provided URI must match the one used when configuring the game state integration.
    pub fn new(uri: &str) -> Self {
        let (notify_shutdown, _) = broadcast::channel(1);
        let (send_events, _) = broadcast::channel(16);

        Self {
            uri: uri.to_owned(),
            notify_shutdown,
            send_events,
            is_shutdown: false,
            handlers: Vec::new(),
        }
    }

    /// Register a new handler on this Server.
    ///
    /// Incoming events from game state integration will be broadcast to all registered handlers.
    pub fn register<H>(mut self, handler: H) -> Self
    where
        H: Handler,
    {
        let registration = HandlerRegistration::new(
            handler,
            self.notify_shutdown.subscribe(),
            self.send_events.subscribe(),
        );
        self.handlers.push(registration);

        self
    }

    /// Start listening to requests and return a handle to the associated [`Listener`] task.
    pub fn start(self) -> Result<Server, GameStateIntegrationError> {
        if self.is_shutdown {
            return Err(GameStateIntegrationError::ServerShutdown);
        }

        let listener = Listener::new(
            &self.uri,
            self.notify_shutdown.subscribe(),
            self.send_events,
        );

        Ok(Server::new(
            tokio::spawn(async move { listener.run().await }),
            self.handlers
                .into_iter()
                .map(|h| tokio::spawn(async move { h.run().await })),
            self.notify_shutdown,
        ))
    }
}

/// Process a game state integration request.
///
/// This function parses the request and reads the entire payload, returning it as a
/// [`bytes::Bytes`].
pub async fn process(mut socket: TcpStream) -> Result<bytes::Bytes, GameStateIntegrationError> {
    if let Err(e) = socket.readable().await {
        log::error!("socket is not readable");
        return Err(GameStateIntegrationError::from(e));
    };

    let mut buf = bytes::BytesMut::with_capacity(INITIAL_REQUEST_BUFFER_CAPACITY_BYTES);
    let request_length: usize;
    let content_length: usize;

    loop {
        match socket.read_buf(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("failed to read request from socket: {}", e);
                return Err(GameStateIntegrationError::from(e));
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
                return Err(GameStateIntegrationError::from(e));
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
                return Err(GameStateIntegrationError::from(e));
            }
        };
    }

    if let Err(e) = socket.write_all(OK.as_bytes()).await {
        log::error!("failed to write to socket: {}", e);
        return Err(GameStateIntegrationError::from(e));
    };

    Ok(buf.split_off(request_length).freeze())
}

/// Extract Content-Length value from a list of HTTP headers.
pub fn get_content_length_from_headers(
    headers: &[httparse::Header],
) -> Result<usize, GameStateIntegrationError> {
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
                    return Err(GameStateIntegrationError::InvalidContentLength(format!(
                        "failed to parse bytes as str: {}",
                        e
                    )));
                }
            };
            match str_length.parse::<usize>() {
                Ok(n) => Ok(n),
                Err(e) => Err(GameStateIntegrationError::InvalidContentLength(format!(
                    "failed to parse str into usize: {}",
                    e
                ))),
            }
        }
        None => Err(GameStateIntegrationError::MissingContentLengthHeader),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            Err(GameStateIntegrationError::MissingContentLengthHeader)
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
            Err(GameStateIntegrationError::InvalidContentLength(_))
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

        let (tx1, mut rx1) = mpsc::channel(2);
        let (tx2, mut rx2) = mpsc::channel(2);

        let mut server = ServerBuilder::new("127.0.0.1:30080")
            .register(move |event| {
                let tx1 = tx1.clone();
                async move {
                    let _ = &tx1.send(event).await?;
                    Ok::<(), mpsc::error::SendError<bytes::Bytes>>(())
                }
            })
            .register(move |event| {
                let tx2 = tx2.clone();
                async move {
                    let _ = &tx2.send(event).await?;
                    Ok::<(), mpsc::error::SendError<bytes::Bytes>>(())
                }
            })
            .start()
            .unwrap();

        // Advance the event loop for listener to start
        sleep(time::Duration::from_millis(10)).await;

        tokio::spawn(async move {
            for _ in 0..2 {
                let mut stream = TcpStream::connect("127.0.0.1:30080").await.unwrap();
                let _ = stream.write_all(sample_request).await;
                let _ = stream.shutdown().await;
            }
        });

        // Advance the event loop for events to be processed
        sleep(time::Duration::from_millis(10)).await;

        if let Err(_) = timeout(time::Duration::from_secs(5), server.shutdown()).await {
            panic!("did not shut down in 5 seconds");
        }

        let mut v1 = Vec::new();
        let mut v2 = Vec::new();

        async fn capture(rx: &mut mpsc::Receiver<bytes::Bytes>, v: &mut Vec<bytes::Bytes>) {
            let val = rx.recv().await;
            v.push(val.unwrap());
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
        assert!(server.is_shutdown());
    }

    #[tokio::test]
    async fn test_listener_shutsdown_when_all_handlers_fail() {
        let sample_request = b"POST / HTTP/1.1\r\nuser-agent: Valve/Steam HTTP Client 1.0 (570)\r\nContent-Type: application/json\r\nHost: 127.0.0.1:20080\r\nAccept: text/html,*/*;q=0.9\r\naccept-encoding: gzip,identity,*;q=0\r\naccept-charset: ISO-8859-1,utf-8,*;q=0.7\r\nContent-Length: 173\r\n\r\n{\n\t\"provider\": {\n\t\t\"name\": \"Dota 2\",\n\t\t\"appid\": 570,\n\t\t\"version\": 47,\n\t\t\"timestamp\": 1688514013\n\t},\n\t\"player\": {\n\n\t},\n\t\"draft\": {\n\n\t},\n\t\"auth\": {\n\t\t\"token\": \"hello1234\"\n\t}\n}";

        let mut server = ServerBuilder::new("127.0.0.1:40080")
            .register(move |_| async move { Err(anyhow::anyhow!("an error")) })
            .register(move |_| async move { Err(anyhow::anyhow!("another error")) })
            .start()
            .unwrap();

        // Advance the event loop for listener to start
        sleep(time::Duration::from_millis(10)).await;

        tokio::spawn(async move {
            for _ in 0..2 {
                let mut stream = TcpStream::connect("127.0.0.1:40080").await.unwrap();
                let _ = stream.write_all(sample_request).await;
                let _ = stream.shutdown().await;
            }
        });

        // Process events, shut down handlers
        sleep(time::Duration::from_millis(10)).await;

        // One more event triggers listener shutdown
        tokio::spawn(async move {
            let mut stream = TcpStream::connect("127.0.0.1:40080").await.unwrap();
            let _ = stream.write_all(sample_request).await;
            let _ = stream.shutdown().await;
        });

        // Listener shuts down
        sleep(time::Duration::from_millis(10)).await;

        let _expected_handler_errors: Vec<GameStateIntegrationError> = vec![
            GameStateIntegrationError::Handler {
                source: anyhow::anyhow!("an error"),
            },
            GameStateIntegrationError::Handler {
                source: anyhow::anyhow!("another error"),
            },
        ];
        match timeout(time::Duration::from_secs(5), server.shutdown()).await {
            Err(_) => {
                panic!("did not finish in 5 seconds");
            }
            Ok(result) => {
                assert!(matches!(
                    result,
                    Err(ServerError {
                        listener_error: Some(GameStateIntegrationError::NoHandlersAvailable),
                        handler_errors: Some(_expected_handler_errors)
                    })
                ));
            }
        }
    }
}
