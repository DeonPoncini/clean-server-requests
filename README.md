# Clean Server Requests

RPC frameworks make it simple to call the server from the client, but what
about from the server to the client? This set of projects provides a mechanism
to make a clean call from the client to the server, and in reverse.

This project shows an idiomatic way to have clean server calls that execute
like regular functions. The server to client calls look like this:

```rust
let msg = cb.route(*uid)?.ping("Game start").await?;
```

Where the `uid` is an ID to select which client to send a message to, and then
a regular function call, that calls that server, and encodes the response back
to the client.

## Scenario
To provide an example of how this works, there is a simple game provided. A user
can host a game, choosing between a coin game or a dice game. The host who
creates a game selects how many players to join, and the type of game to play.
It continues for as many rounds as long as all players vote to continue playing.
For simplicity as this is just a framing device, if two players tie the winner
is arbitrarily chosen from set of winners.

### Coin Game
Flip between 1 and 6 coins. The players have to guess in order whether the
coins were heads or tails. The winner gets the most heads and tails in sequence
correct. For example, if the game flipped H, T, H, with two users, and one
guessed H, T, T and the other guessed T, T, T, the first player would win having
two in sequence correct.

### Dice Game
Roll between 1 and 6 dice, with a set number of faces picked from the standard
polygonal dice shapes. The players guess which numbers were rolled, and are
awarded points based on how many are correct, in any order, and including
duplicates. For example, if the game selected a d8 to roll, and rolled
3, 7, 1, 4, and player 1 guessed, 3, 3, 7, 2, and player 2 guessed 7, 2, 1, 3,
player 1 would get four points and player 2 would get 3.

These games are neither fun, interesting or fair, but they provide a good
motivation for a simple API and only require a few lines of code to express,
so suffice for a motivating example. The idea is to focus on how the interface
is constructed, not on the particulars of the use case.

## Crate Layout
There are three crates that make up the project example:
* csr-client: this is a simple command line application that implements the
client side of the game. One of these is run per player.
* csr-server: this implements the server side of the game logic and hosts the
server that clients connect to
* csr-protocol: This is the library that both client and server depend on. It
provides all the logic that converts server calls into messages to the client
and vice versa.

The protocol is where all the action (and magic!) happens.

# The Protocol
This library is using [gRPC](https://grpc.io/) for client-server communication.
This pattern is usable with any sort of server client communication, as long
as it provides some mechanism for server side communication. For example, web
sockets and JSON is possible, although the code will look different inside, the
same interfaces should be able to be provided.

## The protobuf definition
The protobuf is defined [here](csr-protocol/protos/csr.proto). This is where
our adventure starts. The service definition looks like this:

```protobuf
service Clean {
    // client initiated API
    rpc HostSession(HostInfo) returns (SessionData);
    rpc ListSessions(Empty) returns (Sessions);
    rpc JoinSession(JoinInfo) returns (Empty);
    rpc StartSession(StartInfo) returns (Empty);

    // server initiated API
    rpc ServerEvents(EventRegister) returns (stream ServerRequest);
    rpc RespondToServerEvent(ClientEventResponse) returns (Empty);
}
```
It is separated into two parts here. The first set of messages represent the
client to server API. This is a set of functions that are regular client to server
calls, and aren't the focus of this example. This is standard gRPC functionality,
and exist just to allow the server to be setup.

The two functions at the bottom are a pair, that will be the mechanism by
which we support the server side API. The server side API is a set of
functions that the server will call against the client.

This server side API is found in the [event](csr-protocol/src/event.rs) module.
This interface looks like this:

```rust
pub trait ServerEvent: Send + Sync + 'static {
    async fn join_info(&self, sid: SessionID, uid: UserID, user_name: &str)
        -> Result<()>;
    async fn ping(&self, ping: &str) -> Result<String>;
    async fn roll_dice(&self, sides: u8, count: u8) -> Result<Vec<u8>>;
    async fn flip_coin(&self, count: u8) -> Result<Vec<Coin>>;
    async fn winner(&self, uid: UserID, name: &str) -> Result<()>;
    async fn try_again(&self) -> Result<bool>;
    async fn error(&self, err: &str) -> Result<()>;
}
```

As you can see, this API isn't directly expressed in the protobuf file, as it
is encoded instead in the ServerRequest message type. This looks like this
in the protobuf file:

```protobuf
message ServerRequest {
    oneof msg {
        JoinInfo user_joined = 1;
        Ping ping = 2;
        RollDice dice = 3;
        FlipCoin coin = 4;
        Winner winner = 5;
        bool try_again = 6;
        string error = 7;
    }
}
```

As this uses the `oneof` structure, it allows sending a selection of disjoint
message types, that correspond with responses.

The responses are passed through the `RespondToServerEvent` function, which takes
a `ClientResponse`. (The `ClientEventResponse` just wraps up an ID so messages
can be tracked between request and response).

```protobuf
message ClientResponse {
    oneof msg {
        Pong pong = 1;
        DiceGuess dice_guess = 2;
        CoinGuess coin_guess = 3;
        bool again = 4;
        string error = 5;
    }
}
```

Each of these responses maps to a server request, according to the API presented
from the rust code. For example, `Ping` maps to `Pong`, and are represented
by the `ping` server side function (the types are destructured so only the
contents are passed for ease of use - both Ping and Pong simply wrap a single
string.) Some functions return nothing from the client (such as `Winner`),
so have no corresponding return message (in protobuf this requires us to return
something, so we return an empty message, called `Empty`).

| Server Request | Client Response | Rust function |
|----------------|-----------------|---------------|
| JoinInfo       | Empty           | join\_info    |
| Ping           | Pong            | ping          |
| RollDice       | DiceGuess       | roll\_dice    |
| FlipCoin       | CoinGuess       | flip\_coin    |
| Winner         | Empty           | winner        |
| try\_again     | again           | try\_again    |
| error          | Empty           | error         |

The client `error` is a special case, that encodes the client throwing an
`Err` type on a response, and is handled differently as it could be a response
to any message - this is encoded by all rust methods in the server interface
returning `Result` types.

At this point, the problem becomes clear to solve. Create a wrapper that looks
like the `ServerEvent` trait, that is implemented by both the server and the
client.

On the server side, the implementation of these functions serializes the request
through responding to `ServerEvents` in our RPC, listens for a response from
`RespondToServerEvents`, deserializes this back to a local type and responds.
Then the server just needs to call one of these functions and waits for a
response.

On the client side, we also implement the same trait `ServerEvent`, but instead
we take it and listen for the methods to be called by a thread that is reading
from the network - it reads the data in, deserializes it, calls the function
looks at the result from the client and sends it to the server by calling
`RespondToServerEvents`.
