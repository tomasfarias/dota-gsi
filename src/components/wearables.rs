use serde::{de, de::Error, ser, Deserialize, Serialize};
use std::collections::HashMap;

use thiserror;

use super::{PlayerID, Team};

#[derive(thiserror::Error, Debug)]
pub enum WearablesError {
    #[error("failed to parse wearable ID number in `{0}`")]
    ParseIDError(String),
    #[error("attempted to parse an empty wearables slot")]
    EmptyWearablesSlot,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct WearableID(u8);

impl<'de> Deserialize<'de> for WearableID {
    fn deserialize<D>(deserializer: D) -> Result<WearableID, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut slot_split = s.split("wearable");

        if let (_, Some(index)) = (slot_split.next(), slot_split.next()) {
            let id = index.parse::<u8>().map_err(D::Error::custom)?;
            let wearable = WearableID(id);
            return Ok(wearable);
        }

        Err(WearablesError::ParseIDError(s)).map_err(D::Error::custom)
    }
}

impl Serialize for WearableID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&format!("wearable{}", self.0))
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GameWearables {
    Spectating(HashMap<Team, HashMap<PlayerID, u32>>),
    Playing(HashMap<WearableID, u32>),
}
