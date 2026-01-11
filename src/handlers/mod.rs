use serde::de::DeserializeOwned;

/// Handler to echo back game state integration events.
pub async fn echo_handler<T>(event: bytes::Bytes) -> Result<(), serde_json::Error>
where
    T: DeserializeOwned + std::fmt::Display,
{
    let value: T = match serde_json::from_slice(&event) {
        Err(e) => {
            log::error!("Failed to deserialize JSON body: {}", e);
            return Err(e);
        }
        Ok(v) => v,
    };

    println!("{:#}", value);
    Ok(())
}
