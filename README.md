# dota-gsi

[![crates.io](https://img.shields.io/crates/v/dota-gsi.svg)](https://crates.io/crates/dota-gsi)
[![CI/CD](https://github.com/tomasfarias/dota-gsi/actions/workflows/cd.yaml/badge.svg)](https://github.com/tomasfarias/dota-gsi/actions)

Game State Integration with Dota 2 in Rust. Provides a server that listens for requests sent by Dota 2, processes them to extract their JSON payloads, and broadcasts the payloads to any user-configured handlers.

# Requirements

Integration requires:
1. Creating a `.cfg` [configuration file](https://developer.valvesoftware.com/wiki/Counter-Strike:_Global_Offensive_Game_State_Integration) in the Dota 2 game configuration directory.
2. Running Dota 2 with the -gamestateintegration [launch option](https://help.steampowered.com/en/faqs/view/7d01-d2dd-d75e-2955).

The configuration file can have any name name, but must be prefixed by `gamestate_integration_`.
For example, `gamestate_integration_test.cfg` would be located:
* In Linux: `~/.steam/steam/steamapps/common/dota 2 beta/game/dota/cfg/gamestate_integration_test.cfg`
* In Windows: `D:\Steam\steamapps\common\dota 2 beta\csgo\cfg\gamestate_integration_test.cfg`

Here's a sample configuration file:

```cfg
"dota2-gsi Configuration"
{
   "uri"               "http://127.0.0.1:53000/"
   "timeout"           "5.0"
   "buffer"            "0.1"
   "throttle"          "0.1"
   "heartbeat"         "30.0"
   "data"
   {
       "buildings"     "1"
       "provider"      "1"
       "map"           "1"
       "player"        "1"
       "hero"          "1"
       "abilities"     "1"
       "items"         "1"
       "draft"         "1"
       "wearables"     "1"
   }
   "auth"
   {
       "token"         "abcdefghijklmopqrstuvxyz123456789"
   }
}
```

Note the URI used in the configuration file must be the same URI used when initializing a `Server`.

# Examples

## Echoslam: echo game state integration

This program echoes game state integration events either in raw JSON or after deserialization into components provided by this library. The full program is available at [`src/bin/echoslam.rs`](./src/bin/echoslam.rs)

The program defines a handler to handle game state integration events by deserializing them to `T` (which is later defined to be `serde_json::Value` or `components::GameState`:

```rust
use dota::{Server, components::GameState};

/// Echo back game state integration events.
async fn echo_handler<T>(bytes: bytes::Bytes)
where
    T: DeserializeOwned + std::fmt::Display,
{
    let value: T = match serde_json::from_slice(&bytes) {
        Err(e) => {
            log::error!("Failed to deserialize JSON body: {}", e);
            panic!("deserialize error");
        }
        Ok(v) => v,
    };

    println!("{:#}", value);
}
```

A handler must implement the `Handler` trait, which is automatically implemented for async functions like this one, so it can be directly used in the next step.

In the `main` function, we run the `Server`. This includes first configuring the URI the `Server` will be listening on, and passing the handler function with `T` depending on inputs:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let mut server = Server::new(&args.uri);

    if args.raw {
        server = server.register(echo_handler::<serde_json::Value>);
    } else {
        server = server.register(echo_handler::<GameState>);
    }

    server.run().await?;
    Ok(())
}
```

Finally, the server runs forever.

This program is provided with `dota-gsi` and can be compiled with:

```sh
cargo build --release --bin echoslam
```
