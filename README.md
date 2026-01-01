# dota-gsi

[![crates.io](https://img.shields.io/crates/v/dota-gsi.svg)]
[![CI/CD](https://github.com/tomasfarias/dota-gsi/actions/workflows/cd.yaml/badge.svg)](https://github.com/tomasfarias/dota-gsi/actions)

Game State Integration with Dota 2 in rust. Provides a server that listens for JSON events sent by Dota 2.

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

Take note of the URI used in the configuration file as it must be the same URI used when creating a new `GSIServer`.

# Examples

Examples showcase how to implement handlers that parse the game state data and do whatever we want with it.

## Echoslam: echo back data received by the server

This program uses the provided component models to attempt to parse the JSON received by the server. See the full program at [`src/bin/echoslam.rs`](./src/bin/echoslam.rs)

We simply define two echo handlers as:

```rust
use dota::{components::GameState, GSIServer};

/// Echo back Dota GameState integration state.
async fn echo_gamestate_handler(gs: GameState) {
    println!("{}", gs);
}

/// Echo back raw JSON events.
async fn echo_json_handler(value: serde_json::Value) {
    println!("{}", value);
}
```

Initialize the `GSIServer` using command line arguments with:

```rust
let server = GSIServer::new(&args.uri);
```

And we pass the handlers to the server when running:

```rust
if args.raw {
    server.run(echo_json_handler).await?;
} else {
    server.run(echo_gamestate_handler).await?;
}
```

We have defined a command line flag that determines whether we are attempting to parse the JSON data before echoing it or passing the raw JSON.

This program is provided with `dota-gsi` and can be compiled with:

```sh
cargo build --release --bin echoslam
```
