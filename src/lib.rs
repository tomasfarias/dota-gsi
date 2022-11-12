//! Game State Integration with Dota 2.
//!
//! Provides a server that listens for JSON events sent by Dota 2. Enabling Game State
//! Integration requires:
//! 1. Creating a `.cfg` [configuration file] in the Dota 2 game configuration directory.
//! 2. Running Dota 2 with the -gamestateintegration [launch option].
//!
//! The configuration file can have any name name, but must be prefixed by `gamestate_integration_`.
//! For example, `gamestate_integration_test.cfg` would be located:
//! * In Linux: `~/.steam/steam/steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration_test.cfg`
//! * In Windows: `D:\Steam\steamapps\common\dota 2 beta\csgo\cfg\gamestate_integration_test.cfg`
//!
//! Here's A sample configuration file:
//!
//! ```
//! "dota2-gsi Configuration"
//!{
//!    "uri"               "http://127.0.0.1:3000/"
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
//!        "token"         "hello1234"
//!    }
//!}
//!```
//!
//! Notice that the URI used in the configuration file must be the same URI used when creating a new [`GSIServer`].
//!
//! [configuration file]: https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration
//! [launch option]: https://help.steampowered.com/en/faqs/view/7d01-d2dd-d75e-2955
use std::future::Future;
use std::io;

use async_trait::async_trait;
use bytes::BytesMut;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;

pub mod components;

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
    ParseError(#[from] serde_json::Error),
}

/// The response expected by every GameState Integration request.
/// Failued to deliver this response would cause the request to be retried infinitely.
const OK: &str = "HTTP/1.1 200 OK\ncontent-type: text/html\n";

/// Trait implemented by handlers of Game State data.
#[async_trait]
pub trait GameStateHandler<D>
where
    D: DeserializeOwned + std::fmt::Debug + Send + 'static,
{
    async fn handle(self, gs: D);
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
        D: DeserializeOwned + std::fmt::Debug + Send + 'static,
        U: Future + Send + Sync + 'static,
        U::Output: Send,
    {
        let listener = TcpListener::bind(self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        loop {
            let (socket, addr) = listener.accept().await?;
            log::info!("Accepted: {}", addr);

            let _ = tokio::spawn(async move {
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
        D: DeserializeOwned + std::fmt::Debug + Send + 'static,
    {
        let listener = TcpListener::bind(self.uri).await?;
        log::info!("Listening on: {:?}", listener.local_addr());

        loop {
            let (socket, addr) = listener.accept().await?;
            log::info!("Accepted: {}", addr);
            // Need to clone as handler will be moved by spawn.
            let this_handler = handler.clone();

            let _ = tokio::spawn(async move {
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

/// Process a TcpStream.
/// Ensures the stream's contents can be parsed and returns an appropiate response to Dota.
pub async fn process(mut socket: TcpStream) -> Result<BytesMut, GSIServerError> {
    if let Err(e) = socket.readable().await {
        log::error!("socket is not readable");
        return Err(GSIServerError::from(e));
    };

    let mut buf = BytesMut::with_capacity(122880);

    let n = match socket.read_buf(&mut buf).await {
        Ok(n) if n == 0 => {
            log::debug!("Socket closed");
            return Err(GSIServerError::SocketClosed);
        }
        Ok(n) => n,
        Err(e) => {
            log::error!("failed to read from socket");
            return Err(GSIServerError::from(e));
        }
    };
    log::debug!("Read: {}", n);

    if let Err(e) = socket.write_all(OK.as_bytes()).await {
        log::error!("failed to write to socket");
        return Err(GSIServerError::from(e));
    };

    log::debug!("Raw request: {:?}", buf);
    let amt = match parse_headers(&buf) {
        Some(amt) => amt,
        None => {
            return Err(GSIServerError::IncompleteHeaders);
        }
    };

    let _ = buf.split_to(amt);
    log::debug!("Raw data: {:?}", buf);

    Ok(buf)
}

/// Parse the HTTP request headers.
/// For the time being, we don't care about the headers themselves.
/// We parse them only to ensure they are valid and to get to the beginning of the body.
pub fn parse_headers(buf: &[u8]) -> Option<usize> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut r = httparse::Request::new(&mut headers);

    let status = r.parse(buf).expect("Failed to parse HTTP request");

    match status {
        httparse::Status::Complete(amt) => Some(amt),
        httparse::Status::Partial => None,
    }
}
