use std::io;

use bytes::BytesMut;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task;

pub mod components;

#[derive(Error, Debug)]
pub enum GSIServerError {
    #[error("incomplete headers have been parsed from GSI request")]
    IncompleteHeaders,
    #[error("failed to read (write) from (to) socket listening to GSI")]
    SocketError(#[from] io::Error),
    #[error("failed to complete the assigned GSI task")]
    TaskError(#[from] task::JoinError),
}

const OK: &str = "HTTP/1.1 200 OK\ncontent-type: text/html\n";

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
    pub fn new(uri: &str) -> Self {
        GSIServer {
            uri: uri.to_owned(),
        }
    }

    pub async fn run(self, handler: fn(components::GameState)) -> Result<(), GSIServerError> {
        log::info!("Listening on {}", self.uri);

        let listener = TcpListener::bind(self.uri).await?;

        loop {
            let (mut socket, addr) = listener.accept().await?;
            log::info!("Accepted: {}", addr);

            let _ = tokio::spawn(async move {
                log::debug!("Task spawned");

                if let Err(e) = socket.readable().await {
                    log::error!("socket is not readable");
                    return Err(GSIServerError::from(e));
                };

                let mut buf = BytesMut::with_capacity(122880);

                let n = match socket.read_buf(&mut buf).await {
                    Ok(n) if n == 0 => {
                        log::debug!("Socket closed");
                        return Ok(());
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

                let gs: components::GameState =
                    serde_json::from_slice(&buf).expect("Failed to parse JSON body");

                log::debug!("Parsed: {:?}", gs);
                handler(gs);

                Ok(())
            })
            .await?;
        }
    }
}

pub fn parse_headers(buf: &[u8]) -> Option<usize> {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut r = httparse::Request::new(&mut headers);

    let status = r.parse(buf).expect("Failed to parse HTTP request");

    match status {
        httparse::Status::Complete(amt) => Some(amt),
        httparse::Status::Partial => None,
    }
}
