use serde::{de, Deserialize};

/// Deserialize Vec<T> by flattening JSON of teams and players
pub fn deserialize_nested<'de, D, T: Deserialize<'de>>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Helper<T> {
        team2: Team2<T>,
        team3: Team3<T>,
    }

    #[derive(Deserialize)]
    struct Team2<T> {
        player0: T,
        player1: T,
        player2: T,
        player3: T,
        player4: T,
    }

    #[derive(Deserialize)]
    struct Team3<T> {
        player5: T,
        player6: T,
        player7: T,
        player8: T,
        player9: T,
    }

    let helper = Helper::deserialize(deserializer)?;

    // I don't know if there is a better way of doing this.
    let v: Vec<T> = vec![
        helper.team2.player0,
        helper.team2.player1,
        helper.team2.player2,
        helper.team2.player3,
        helper.team2.player4,
        helper.team3.player5,
        helper.team3.player6,
        helper.team3.player7,
        helper.team3.player8,
        helper.team3.player9,
    ];
    Ok(v)
}
