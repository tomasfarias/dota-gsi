use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use serde::{de, de::Error, de::IntoDeserializer, ser, Deserialize, Serialize};
use serde_json::{map, Value};
use thiserror;

use super::{PlayerID, Team};

#[derive(thiserror::Error, Debug)]
pub enum WearablesError {
    #[error("failed to parse wearable slot number in `{0}`")]
    ParseSlotError(String),
    #[error("failed to parse wearable from value `{0}`")]
    ParseWearableError(Value),
    #[error("attempted to parse an empty wearables slot")]
    EmptyWearablesSlot,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct WearableSlot(u8);

impl fmt::Display for WearableSlot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WearableSlot({})", self.0)
    }
}

impl From<u8> for WearableSlot {
    fn from(n: u8) -> WearableSlot {
        WearableSlot(n)
    }
}

impl FromStr for WearableSlot {
    type Err = WearablesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = s
            .parse::<u8>()
            .map_err(|_| WearablesError::ParseSlotError(s.to_owned()))?;
        Ok(WearableSlot::from(id))
    }
}

impl<'de> Deserialize<'de> for WearableSlot {
    fn deserialize<D>(deserializer: D) -> Result<WearableSlot, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserialize_slot_by_string_split::<D, WearableSlot>(deserializer, vec!["wearable", "style"])
    }
}

impl Serialize for WearableSlot {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&format!("wearable{}", self.0))
    }
}

#[derive(Debug, Serialize)]
pub struct Wearable {
    id: Option<u32>,
    style: Option<u32>,
}

impl Wearable {
    pub fn new(id: Option<u32>, style: Option<u32>) -> Wearable {
        Wearable { id, style }
    }
}

/// Wrapper for Wearable items.
#[derive(Debug, Serialize)]
pub struct Wearables {
    inner: HashMap<WearableSlot, Wearable>,
}

impl Wearables {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get(&self, slot: &WearableSlot) -> Option<&Wearable> {
        self.inner.get(slot)
    }
}

impl<'de> Deserialize<'de> for Wearables {
    fn deserialize<D>(deserializer: D) -> Result<Wearables, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let m = map::Map::<String, Value>::deserialize(deserializer)?;
        let mut hm: HashMap<WearableSlot, Wearable> = HashMap::new();

        for (key, value) in m {
            let slot: WearableSlot = WearableSlot::deserialize(key.clone().into_deserializer())?;
            let id: u32 = match value.as_u64() {
                Some(n) => n as u32,
                None => return Err(D::Error::custom(WearablesError::ParseWearableError(value))),
            };

            match hm.get_mut(&slot) {
                Some(entry) => {
                    if key.starts_with("style") {
                        entry.style = Some(id);
                    } else {
                        entry.id = Some(id);
                    }
                }
                None => {
                    if key.starts_with("style") {
                        hm.insert(slot, Wearable::new(None, Some(id)));
                    } else {
                        hm.insert(slot, Wearable::new(Some(id), None));
                    }
                }
            }
        }

        Ok(Wearables { inner: hm })
    }
}

fn deserialize_slot_by_string_split<'de, D, T>(
    deserializer: D,
    split_on: Vec<&str>,
) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    for split_on_str in split_on.iter() {
        let mut slot_split = s.split(split_on_str);

        if let (_, Some(index)) = (slot_split.next(), slot_split.next()) {
            return T::from_str(index).map_err(D::Error::custom);
        }
    }

    Err(D::Error::custom(WearablesError::ParseSlotError(s)))
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GameWearables {
    Spectating(HashMap<Team, HashMap<PlayerID, Wearables>>),
    Playing(Wearables),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wearables_deserialize() {
        let json_str = r#"{
  "wearable0": 8863,
  "wearable1": 8865,
  "wearable2": 18582,
  "wearable3": 8869,
  "wearable4": 8871,
  "style4": 2,
  "wearable5": 4764,
  "wearable6": 5105,
  "wearable7": 8632,
  "wearable8": 790,
  "style8": 1,
  "wearable9": 791,
  "wearable10": 14912,
  "style10": 3
}"#;

        let wearables: Wearables =
            serde_json::from_str(json_str).expect("Failed to deserialize Wearables");

        let wearable_4 = wearables.get(&WearableSlot::from(4)).unwrap();
        let wearable_10 = wearables.get(&WearableSlot::from(10)).unwrap();
        let wearable_8 = wearables.get(&WearableSlot::from(8)).unwrap();
        let wearable_1 = wearables.get(&WearableSlot::from(1)).unwrap();

        assert_eq!(wearables.len(), 11);

        assert!(wearable_4.id.is_some());
        assert!(wearable_4.style.is_some());
        assert_eq!(wearable_4.style.unwrap(), 2);
        assert_eq!(wearable_4.id.unwrap(), 8871);

        assert!(wearable_10.id.is_some());
        assert!(wearable_10.style.is_some());
        assert_eq!(wearable_10.style.unwrap(), 3);
        assert_eq!(wearable_10.id.unwrap(), 14912);

        assert!(wearable_8.id.is_some());
        assert!(wearable_8.style.is_some());
        assert_eq!(wearable_8.style.unwrap(), 1);
        assert_eq!(wearable_8.id.unwrap(), 790);

        assert!(wearable_1.id.is_some());
        assert!(wearable_1.style.is_none());
        assert_eq!(wearable_1.id.unwrap(), 8865);
    }

    #[test]
    fn test_wearables_deserialize_no_styles() {
        let json_str = r#"{
  "wearable0": 8863,
  "wearable1": 8865,
  "wearable2": 18582,
  "wearable3": 8869,
  "wearable4": 8871,
  "wearable5": 4764,
  "wearable6": 5105,
  "wearable7": 8632,
  "wearable8": 790,
  "wearable9": 791,
  "wearable10": 14912
}"#;

        let wearables: Wearables =
            serde_json::from_str(json_str).expect("Failed to deserialize Wearables");

        let wearable_4 = wearables.get(&WearableSlot::from(4)).unwrap();
        let wearable_10 = wearables.get(&WearableSlot::from(10)).unwrap();
        let wearable_8 = wearables.get(&WearableSlot::from(8)).unwrap();
        let wearable_1 = wearables.get(&WearableSlot::from(1)).unwrap();

        assert_eq!(wearables.len(), 11);

        assert!(wearable_4.id.is_some());
        assert!(wearable_4.style.is_none());
        assert_eq!(wearable_4.id.unwrap(), 8871);

        assert!(wearable_10.id.is_some());
        assert!(wearable_10.style.is_none());
        assert_eq!(wearable_10.id.unwrap(), 14912);

        assert!(wearable_8.id.is_some());
        assert!(wearable_8.style.is_none());
        assert_eq!(wearable_8.id.unwrap(), 790);

        assert!(wearable_1.id.is_some());
        assert!(wearable_1.style.is_none());
        assert_eq!(wearable_1.id.unwrap(), 8865);
    }
}
