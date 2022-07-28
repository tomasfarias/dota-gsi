use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub mod components;

const OK: &str = "HTTP/1.1 200 OK";

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

    pub async fn run(
        self,
        handler: fn(components::GameState),
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Listening on {}", self.uri);

        let listener = TcpListener::bind(self.uri).await?;

        loop {
            let (mut socket, addr) = listener.accept().await?;
            log::debug!("Accepted: {}", addr);

            tokio::spawn(async move {
                log::debug!("Task spawned");

                if let Err(e) = socket.readable().await {
                    log::error!("Unreadable socket; err = {:?}", e);
                    return;
                };

                let mut buf = BytesMut::with_capacity(122880);

                let n = match socket.read_buf(&mut buf).await {
                    Ok(n) if n == 0 => {
                        log::debug!("Socket closed");
                        return;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };
                log::debug!("Read: {}", n);

                if let Err(e) = socket.write_all(OK.as_bytes()).await {
                    log::error!("failed to write to socket; err = {:?}", e);
                    return;
                };
                log::debug!("Raw request: {:?}", buf);
                let amt = parse_headers(&buf);

                let _ = buf.split_to(amt);
                log::debug!("Raw data: {:?}", buf);

                let gs: components::GameState =
                    serde_json::from_slice(&buf).expect("Failed to parse JSON body");

                log::debug!("Parsed: {:?}", gs);
                handler(gs);
            })
            .await?;
        }
    }
}

pub fn parse_headers(buf: &[u8]) -> usize {
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut r = httparse::Request::new(&mut headers);

    let status = r.parse(buf).expect("Failed to parse HTTP request");

    let amt = match status {
        httparse::Status::Complete(amt) => amt,
        httparse::Status::Partial => panic!("I'll figure it out later"),
    };

    amt
}
