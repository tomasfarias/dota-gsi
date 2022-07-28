use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
#[serde(from = "String")]
pub enum Team {
    Radiant,
    Dire,
    None,
    Undefined(String),
}

impl From<String> for Team {
    fn from(s: String) -> Self {
        return match s.as_str() {
            "radiant" | "team2" => Team::Radiant,
            "dire" | "team3" => Team::Dire,
            "none" => Team::None,
            _ => Team::Undefined(s),
        };
    }
}

impl fmt::Display for Team {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Team::Radiant => write!(f, "Radiant"),
            Team::Dire => write!(f, "Dire"),
            Team::None => write!(f, "None"),
            Team::Undefined(s) => write!(f, "Undefined: {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_from_str() {
        assert!(matches!(Team::from("radiant".to_string()), Team::Radiant));
        assert!(matches!(Team::from("dire".to_string()), Team::Dire));
    }
}
