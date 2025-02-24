use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::num::ParseIntError;

use serde::{de, Deserialize, Serialize};
use thiserror::Error;

use super::{PlayerID, Team};

#[derive(Error, Debug)]
pub enum ItemsError {
    #[error("the container `{0}` has no slot")]
    MissingSlotInContainer(String),
    #[error("failed to parse slot number")]
    ParseSlotError(#[from] ParseIntError),
    #[error("the filed `{0}` is missing in `{1}`")]
    MissingRequiredField(String, ItemContainer),
    #[error("an unknown item container was found: `{0}`")]
    UnknownItemContainer(String),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(from = "String")]
pub enum Rune {
    Arcane,
    Bounty,
    DoubleDamage,
    Empty,
    Haste,
    Illusion,
    Invisibility,
    Regeneration,
    Shield,
    Undefined(String),
}

impl From<String> for Rune {
    fn from(s: String) -> Self {
        return match s.as_str() {
            "arcane" => Rune::Arcane,
            "bounty" => Rune::Bounty,
            "double_damage" => Rune::DoubleDamage,
            "empty" => Rune::Empty,
            "haste" => Rune::Haste,
            "illusion" => Rune::Illusion,
            "invisibility" => Rune::Invisibility,
            "regen" => Rune::Regeneration,
            "shield" => Rune::Shield,
            _ => Rune::Undefined(s),
        };
    }
}

impl fmt::Display for Rune {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rune::Arcane => write!(f, "Arcane"),
            Rune::Bounty => write!(f, "Bounty"),
            Rune::DoubleDamage => write!(f, "Double damage"),
            Rune::Empty => write!(f, "Empty"),
            Rune::Haste => write!(f, "Haste"),
            Rune::Illusion => write!(f, "Illusion"),
            Rune::Invisibility => write!(f, "Invisibility"),
            Rune::Regeneration => write!(f, "Regeneration"),
            Rune::Shield => write!(f, "Shield"),
            Rune::Undefined(s) => write!(f, "Rune {}", s),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(try_from = "String")]
pub enum ItemContainer {
    Inventory(u8),
    Stash(u8),
    Teleport,
    Neutral,
    PreservedNeutral,
}

impl ItemContainer {
    fn index(&self) -> u8 {
        match self {
            ItemContainer::Inventory(n) | ItemContainer::Stash(n) => *n,
            ItemContainer::Teleport | ItemContainer::Neutral | ItemContainer::PreservedNeutral => 0,
        }
    }
}

impl fmt::Display for ItemContainer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ItemContainer::Inventory(n) => write!(f, "Inventory: {}", n),
            ItemContainer::Stash(n) => write!(f, "Stash: {}", n),
            ItemContainer::Teleport => write!(f, "Teleport"),
            ItemContainer::Neutral => write!(f, "Neutral"),
            ItemContainer::PreservedNeutral => write!(f, "Preserved neutral"),
        }
    }
}

impl TryFrom<String> for ItemContainer {
    type Error = ItemsError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let index = match find_first_numeric(&s) {
            Some(i) => i,
            None => {
                return Err(ItemsError::MissingSlotInContainer(s.to_string()));
            }
        };

        let (container, slot) = s.split_at(index);
        let numeric_slot = slot.parse::<u8>()?;

        match container {
            "slot" => Ok(ItemContainer::Inventory(numeric_slot)),
            "stash" => Ok(ItemContainer::Stash(numeric_slot)),
            "teleport" => Ok(ItemContainer::Teleport),
            "neutral" => Ok(ItemContainer::Neutral),
            "preserved_neutral" => Ok(ItemContainer::PreservedNeutral),
            s => Err(ItemsError::UnknownItemContainer(s.to_owned())),
        }
    }
}

fn find_first_numeric(s: &str) -> Option<usize> {
    for (i, c) in s.chars().enumerate() {
        if c.is_numeric() {
            return Some(i);
        }
    }

    None
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    name: String,
    purchaser: i16,
    item_level: Option<u16>,
    contains_rune: Option<Rune>,
    can_cast: Option<bool>,
    cooldown: Option<u16>,
    passive: bool,
    charges: Option<u16>,
    item_charges: Option<u16>,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ItemSlot {
    Empty { index: u8 },
    Full { index: u8, item: Item },
}

impl fmt::Display for ItemSlot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ItemSlot::Full { index, item } => write!(f, "Slot {}: {}", index, item.name),
            ItemSlot::Empty { index } => write!(f, "Slot {}: Empty", index),
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(untagged)]
pub enum GameItems {
    Playing(Items),
    Spectating(HashMap<Team, HashMap<PlayerID, Items>>),
}

#[derive(Serialize, Debug)]
pub struct Items {
    inventory: Vec<ItemSlot>,
    stash: Vec<ItemSlot>,
    teleport: ItemSlot,
    neutrals: Vec<ItemSlot>,
    preserved_neutrals: Vec<ItemSlot>,
}

impl Items {
    pub fn is_inventory_empty(&self) -> bool {
        self.inventory.iter().all(|item| match item {
            ItemSlot::Empty { index: _ } => true,
            ItemSlot::Full { index: _, item: _ } => false,
        })
    }

    pub fn is_stash_empty(&self) -> bool {
        self.stash.iter().all(|item| match item {
            ItemSlot::Empty { index: _ } => true,
            ItemSlot::Full { index: _, item: _ } => false,
        })
    }

    pub fn is_teleport_empty(&self) -> bool {
        match self.teleport {
            ItemSlot::Empty { index: _ } => true,
            ItemSlot::Full { index: _, item: _ } => false,
        }
    }

    pub fn is_neutrals_empty(&self) -> bool {
        self.neutrals.iter().all(|item| match item {
            ItemSlot::Empty { index: _ } => true,
            ItemSlot::Full { index: _, item: _ } => false,
        })
    }

    pub fn is_preserved_neutrals_empty(&self) -> bool {
        self.preserved_neutrals.iter().all(|item| match item {
            ItemSlot::Empty { index: _ } => true,
            ItemSlot::Full { index: _, item: _ } => false,
        })
    }
}

impl fmt::Display for Items {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Inventory: ")?;

        if self.is_inventory_empty() {
            writeln!(f, "Empty")?;
        } else {
            for (index, slot) in self.inventory.iter().enumerate() {
                writeln!(f, "{{ {} }}", slot)?;

                if (index + 1) != self.inventory.len() {
                    write!(f, "{:11}", "")?;
                }
            }
        };

        write!(f, "Stash: ")?;

        if self.is_stash_empty() {
            writeln!(f, "Empty")?;
        } else {
            for (index, slot) in self.stash.iter().enumerate() {
                writeln!(f, "{{ {} }}", slot)?;

                if (index + 1) != self.stash.len() {
                    write!(f, "{:7}", "")?;
                }
            }
        };

        if self.is_teleport_empty() {
            writeln!(f, "Teleport: Empty")?;
        } else {
            writeln!(f, "Teleport: {}", self.teleport)?;
        }

        if self.is_neutrals_empty() {
            writeln!(f, "Neutral: Empty")?;
        } else {
            for (index, slot) in self.neutrals.iter().enumerate() {
                writeln!(f, "{{ {} }}", slot)?;

                if (index + 1) != self.inventory.len() {
                    write!(f, "{:11}", "")?;
                }
            }
        }

        if self.is_preserved_neutrals_empty() {
            writeln!(f, "Preserved neutral: Empty")?;
        } else {
            for (index, slot) in self.preserved_neutrals.iter().enumerate() {
                writeln!(f, "{{ {} }}", slot)?;

                if (index + 1) != self.preserved_neutrals.len() {
                    write!(f, "{:11}", "")?;
                }
            }
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for Items {
    /// Deserialize Items by flattening JSON of ItemContainers.
    /// Items can be contained in Inventory, Stash, Teleport slot, or Neutral slot.
    fn deserialize<D>(deserializer: D) -> Result<Items, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            #[serde(flatten)]
            items: HashMap<String, NestedItem>,
        }

        #[derive(Deserialize)]
        struct NestedItem {
            name: String,
            purchaser: Option<i16>,
            item_level: Option<u16>,
            contains_rune: Option<Rune>,
            can_cast: Option<bool>,
            cooldown: Option<u16>,
            passive: Option<bool>,
            item_charges: Option<u16>,
            charges: Option<u16>,
        }

        let helper = Helper::deserialize(deserializer)?;
        let mut inventory: Vec<ItemSlot> = Vec::new();
        let mut stash: Vec<ItemSlot> = Vec::new();
        let mut teleport: ItemSlot = ItemSlot::Empty { index: 0 };
        let mut neutrals: Vec<ItemSlot> = Vec::new();
        let mut preserved_neutrals: Vec<ItemSlot> = Vec::new();

        for (k, v) in helper.items.into_iter() {
            let container = ItemContainer::try_from(k).map_err(de::Error::custom)?;

            let item = if v.name == "empty" {
                ItemSlot::Empty {
                    index: container.index(),
                }
            } else {
                ItemSlot::Full {
                    index: container.index(),
                    item: Item {
                        name: v.name,
                        purchaser: v
                            .purchaser
                            .ok_or_else(|| {
                                ItemsError::MissingRequiredField("purchaser".to_owned(), container)
                            })
                            .map_err(de::Error::custom)?,
                        item_level: v.item_level,
                        contains_rune: v.contains_rune,
                        can_cast: v.can_cast,
                        cooldown: v.cooldown,
                        passive: v
                            .passive
                            .ok_or_else(|| {
                                ItemsError::MissingRequiredField("passive".to_owned(), container)
                            })
                            .map_err(de::Error::custom)?,
                        item_charges: v.item_charges,
                        charges: v.charges,
                    },
                }
            };

            match container {
                ItemContainer::Inventory(_) => inventory.push(item),
                ItemContainer::Stash(_) => stash.push(item),
                ItemContainer::Teleport => {
                    teleport = item;
                }
                ItemContainer::Neutral => neutrals.push(item),
                ItemContainer::PreservedNeutral => preserved_neutrals.push(item),
            }
        }

        Ok(Items {
            inventory,
            stash,
            teleport,
            neutrals,
            preserved_neutrals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_items_deserialize() {
        let json_str = r#"{
          "slot0": {
              "name": "empty"
          },
          "slot1": {
              "name": "empty"
          },
          "slot2": {
              "name": "empty"
          },
          "slot3": {
              "name": "empty"
          },
          "slot4": {
              "name": "empty"
          },
          "slot5": {
              "name": "empty"
          },
          "slot6": {
              "name": "empty"
          },
          "slot7": {
              "name": "empty"
          },
          "slot8": {
              "name": "empty"
          },
          "stash0": {
              "name": "empty"
          },
          "stash1": {
              "name": "empty"
          },
          "stash2": {
              "name": "empty"
          },
          "stash3": {
              "name": "empty"
          },
          "stash4": {
              "name": "empty"
          },
          "stash5": {
              "name": "empty"
          },
          "teleport0": {
              "name": "item_tpscroll",
              "purchaser": 0,
              "item_level": 1,
              "can_cast": false,
              "cooldown": 96,
              "passive": false,
              "item_charges": 1,
              "charges": 1
          },
          "neutral0": {
            "name": "empty"
          },
          "neutral1": {
            "name": "empty"
          },
          "preserved_neutral6": {
            "name": "empty"
          },
          "preserved_neutral7": {
            "name": "empty"
          },
          "preserved_neutral8": {
            "name": "empty"
          },
          "preserved_neutral9": {
            "name": "empty"
          },
          "preserved_neutral10": {
            "name": "empty"
          }
        }"#;

        let items: Items = serde_json::from_str(json_str).expect("Failed to deserialize items");

        assert!(matches!(
            items.teleport,
            ItemSlot::Full { index: 0, item: _ }
        ));

        assert!(items.is_inventory_empty());
        assert!(items.is_stash_empty());
        assert!(items.is_neutrals_empty());
        assert!(items.is_preserved_neutrals_empty());
    }
}
