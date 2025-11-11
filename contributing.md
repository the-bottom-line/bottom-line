# Contributing

This document outlines the architecture of the project and what one should consider before adding or removing anything.

## Rules

* Use a feature branch
* Make sure tests pass before merging into main (`cargo test`)
* Run `cargo fmt` before committing

## Architecture

The project contains three main workspaces, `game`, `server` and `responses`. In short, `game` is responsible for handling all game logic, `server` handles the websocket-based backend and `responses` contains the requests and the responses the frontend and backend use to communicate with each other.

### The Game Workspace

The game workspace contains all game logic and is in that sense self-contained. The way the game is designed is that its state is inaccessable and can only be changed by api functions that take a mutable reference in order to change its internal state. There are a few smaller files which contain the following content:

* `cards.rs`: Contains all code that is responsible for parsing `boardgame.json`, which can be found in `/assets/cards`. `boardgame.json` has information to initialize all assets, liabilities and events.
* `errors.rs`: Contains all errors used in the game workspace. Has suberrors for a few different actions, with `GameError` being the main error propagated by the exposed API. It uses the [thiserror crate](https://crates.io/crates/thiserror) crate for this purpose.
* `utility.rs`: Contains a wrapper for `Either<Asset, Liability>` to serialize that nicely.

#### game.rs

This is where the core gamestate is kept. The main struct is `GameState`:

```rs
#[derive(Debug, Clone)]
pub enum GameState {
    Lobby(Lobby),
    SelectingCharacters(SelectingCharacters),
    Round(Round),
    Results(Results),
}
```

This is an enum value of underlying states `Lobby`, `SelectingCharacters`, `Round` and `Results`. `GameState` implements `<state>()` and `<state>_mut()` to get conditional access to the underlying state object if it is in that state. Besides that, it only contains its transition functions:

* `start_game()`: can change state from `Lobby` to `SelectingCharacters` if between 4 and 7 players are currently in the lobby.
* `select_character()`: can change state from `SelectingCharacters` to `Round` if the last person picked their character.
* `end_turn()`: can change state from `Round` back to `SelectingCharacters` if the last player ends their turn.

Each round struct implements its own relevant functions. `SelectingCharacters` implements functions like `select_character()`, `Round` implements things like `draw_card()` and `play_card()` and so on.

#### player.rs

This is where the code for the player state is kept. There is a player struct that corresponds to each game state:

* `LobbyPlayer`: does not need to perform any actions, so doesn't implement anything. Contains player name and player id.
* `SelectingCharactersPlayer`: implements `select_character()`. Now also has `character` as `Option<Character>`, `cash`, `hand`, and bought assets and liabilities.
* `RoundPlayer`: implements things like `draw_card()`, `put_back_card()` and so on on the player level. For the roundplayer, the `character` is not optional as everyone has selected a character when the round starts. Also contains a lot of fields that keep track of whether or not player can perform certain actions in each turn.
* `ResultsPlayer`: implements a lot of score calculation functions.

### The Responses Workspace

* `Connect`: an enum with a single value `Connect` which the frontend sends to connect to a room.
* `FrontendRequest`: an enum with all possible valid requests the frontend can send.
* `DirectResponse`: an enum with all possible responses the server can send directly back to the player when they make a request. If they draw a card, the response they get is of type `DirectResponse`.
* `UniqueResponse`: an enum with all possible responses everybody gets back personally. If a player that's not you successfully draws a card, everyone will get a response of this type with a card type, which is anonymous compared to a whole asset or liability.

### The Server Workspace

This is where the code for the websocket server layer is kept. The server uses axum websockets to do its bidding. Globally it contains a collection of `RoomState`s, that are accessed by channel string:

```rs
pub struct RoomState {
    pub tx: broadcast::Sender<UniqueResponse>,
    pub player_tx: [broadcast::Sender<UniqueResponse>; 7],
    pub game: Mutex<GameState>,
}
```

Each roomstate has a global broadcast `tx` to which everybody listens in `send_task` inside `server.rs`. Each player listens to a specific `player_tx` based on their player id in `send_player_task` and everybody receives things from their specific player connection in `recv_task`.

`RoomState` has a `handle_request` function which takes a `FrontendRequest` and handles it depending on what it is. It uses functions from `request_handler.rs`, which can handle each individual request. Each function is called similar to the following:

```rs
// matching on `FrontendRequest`:
FrontendRequest::DrawCard { card_type } => {
    let player_id = state.round()?.player_by_name(player_name)?.id;
    draw_card(state, card_type, player_id)
}
```

An example is `draw_card()`:

```rs
pub fn draw_card(
    state: &mut GameState,
    card_type: CardType,
    player_id: PlayerId,
) -> Result<Response, GameError> {
    let round = state.round_mut()?;
    let card = round.player_draw_card(player_id, card_type)?.cloned();
    let player = round.player(player_id)?;

    let internal = round
        .players()
        .iter()
        .filter(|p| p.id != player_id)
        .map(|p| {
            (
                p.id,
                vec![UniqueResponse::DrewCard {
                    player_id,
                    card_type,
                }],
            )
        })
        .collect();

    Ok(Response(
        InternalResponse(internal),
        DirectResponse::YouDrewCard {
            card,
            can_draw_cards: player.can_draw_cards(),
            can_give_back_cards: player.should_give_back_cards(),
        },
    ))
}
```

These always return `Result<Response, GameError>`. As arguments they take a `GameState` plus whatever the corresponding `FrontendRequest` provided (in this case `card_type`) and the `PlayerId` of the player whose client it is. The function is always executed for this player, so they get the `DirectResponse` back. Inside the function, the required actual state is obtained (in this case `state.round_mut()?`) on which the actual intended action can be executed (`round.player_draw_card()`). When this is done a `HashMap<PlayerId, UniqueResponse>` is collected from an iterator of `(PlayerId, UniqueResponse)` tuples (check the return type of the `.map()`). In this case the player who executed the request is skipped, as indicated by the `.filter()` line. This is everything needed to then build the internal and external responses.

## Contributing

In this section I'll outline some common ways one might contribute code. I'll cover what to think about and where you should first look when changing something.

### Changing logic that runs before a turn

If it's purely something that runs for the start of the player whose turn it is, look at `RoundPlayer::turn_starts()`. If it's something that should run for every player, check both `SelectingCharacters::select_character()` when nobody can select characters anymore and `Round::end_turn()` if it's not the last round.

### Adding Errors

When your feature requires extra errors, you might either want to create a new error type or add errors to `GameError`. You should add a new error type if you're working on a specific feature of the game (say picking characters logic or drawing a card) that is not exposed directly outward on API found on `Round`, `SelectingCharacters` and so on. If you do, add lines like this

```rs
#[error(transparent)]
LobbyError(#[from] LobbyError),
```

to `GameError` to make them convertable into one another, either with `.into()` or the `?` operator.

### Adding a gameplay feature in the gamestate

When adding a feature to the gamestate, remember that unless you're adding an entirely new gamestate you should never add any new API functions to `GameState`. Besides that, remember the way each API function works is that you pass `&mut self` and change the state internally. Never return any information that isn't otherwise kept track of in some way internally.

1. When should this information be accessed? Is it relevant just during a round, or also in the results stage for example?
2. At what level (game state or player state) should the new information required for the feature be implemented? Sometimes it's obvious whether you should add something to either state, but think about the implications of each. Player state is preferred when it's something that can be unique for each player.
3. What error/success states do I have within each action? Should I add new errors? For more on this, check [Adding Errors](#adding-errors) for more info.

### Adding requests

If you want to add a request, do so in the `responses` workspace. There are three main components to it:

* `FrontendRequest`: this is what the backend receives from the frontend when a player performs an action.
* `DirectResponse`: this is something that's sent back to the player who sent the request. Current naming convention is that it follows a structure of `YouDitThing`.
* `UniqueResponse`: this is something that's usually sent to everyone but the player who performed the action. Looking at the `draw_card()` function above, logic is needed to receive a `FrontendRequest` which should be a function in `request_handler.rs`. This function should be able to perform an action on the specific game state (`SelectingCharacters`, `Round` and so on) and that should return whatever the frontend roughly wants to see + any errors encountered. That's the path a request should generally walk.
