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

# The Calling Sequence

Tracing a full call is the easiest way to understand how data flows through the
system. The selected call will be the one at the top,

```rust
let msg = cb.route(*uid)?.ping("Game start").await?;
```

The call starts within the service [game\_thread](csr-server/src/service.rs#L193).
This is a separate thread of execution that runs to interact with the client.
`cb` here the [Callback](csr-server/src/service.rs#L32) object which simply
keeps a map of [ServerEventSender](csr-protocol/src/event.rs#L23) for each
connected client, allowing the right sender to be selected. One sender is required
to contact each client. The `route` function simply returns the correct sender
based on the ID of the client that has connected, providing a lookup.

The [ServerEventSender](csr-protocol/src/event.rs#L23) is the provided interface
for communicating with the client. As the name suggests, it sends server
events. The interface it provides is by implementing the trait
[ServerEvent](csr-protocol/src/event.rs#L11). This provides the entire menu of
functions that can be called from server to client. This interface is implemented
twice, once on in the sender, and again by the client. The machinery between
these provides the path that retains the illusion of a single call.

The server uses this sender through its implementation of the
[server\_events](csr-protocol/src/server.rs#L89). This is part of the RPC contract
defined in the [protobuf](csr-protocol/protos/csr.proto#L13). This implementation
creates three channels to pass messages around. This is implemented through
the Clean trait that is generated by Tonic, representing this gRPC service.

## Server channels
The first channel, is between the server and Tonic. This is the standard way
streams are implemented in rust, and is expected for any gRPC function that
returns a stream.

The second channel is what is used to receive messages from the ServerEventSender.
The transmitter half of this channel is passed in to the ServerEventSender,
and this is what channel is first called by the sender when a function is called.
For example, with the [Ping](csr-protocol/src/event.rs#L60) call, the Sender
being called there is this one. The receiver end of this call is a separate
[thread](csr-protocol/src/server.rs#L114) within the server\_events implementation,
that simply receives these messages, converts the internal type into a protobuf
type for sending over the network, then calls the first channel to send it
on across the network.

The third channel is for responses. The receiver side of this channel is also
passed into ServerEventSender. This is read in the [poll](csr-protocol/src/event.rs#L39)
function which every function in the ServerEventSender ServerEvent trait implementation
calls to check results. For Ping, this happens [here](csr-protocol/src/event.rs#L61).

Note here, the response channel being read could have any response. The client
can send any sort of message over the wire, so we have to check if the response
is the one we expect. If it is, we pass this as the return of the interface.
This returns back to the [caller](csr-server/src/server.rs#L193) becoming the
value read into `msg` in the example. If an invalid response is sent the
service needs to handle that error - in this case it terminates the game.

The sender for the third channel is special. It is stored in the
[state](csr-protocol/src/server.rs#L25) of the server. This is needed as it is
the [respond\_to\_server\_event](csr-protocol/src/server.rs#L128) function that
receives the protobuf values over the wire from the client, translates them to
local types and then sends through this third channel to the ServerEventSender.
How this method is invoked will be looked at in the return path.

## Outbound from the server
Recapping from above, the outbound path looks like:
* [Ping](csr-server/src/service.rs#L193) from within the server through the ServerEventSender
* [Transmit](csr-protocol/src/event.rs#L60) over the second channel
* [Receive](csr-protocol/src/server.rs#L114) in the server\_events method
* [Convert](csr-protocol/src/server.rs#L115) to a protobuf type for sending
* [Transmit](csr-protocol/src/server.rs#L116) over the network, through the first channel, out of the ReceiverStream

After this, the message is on its way over the network, with Tonic taking care
of that piece of the networking.

A note here: why are we converting types? Why don't we just use the protobuf types?

## Anti-Corruption Layer
This examples provides an anti-corruption layer, implemented in the
[types](csr-protocol/src/types.rs) module. Rust has a very intuitive type
type conversion facility through the From trait. This trait allows us to convert
the protobuf type to a local type. This is strongly recommended as protobuf types
have some drawbacks:
* Enumerations need to have default values, and have an underlying i32 value
* Field access can be awkward
* Network types may not represent the business domain
* Changing protocol implementation touches a lot more code
* Error checking and validation can be localized

If we wanted to move from protobuf to say, thrift, or even JSON, we can contain
all that code in this anti-corruption layer. Otherwise we would have to change
types throughout the entire program if we change the protocol implementation.
This provides robustness and allows for precise types. The downside is a large
amount of boilerplate conversion code, but it is worth the cost.

## Receiving on the client
The [client](csr-protocol/src/client.rs) wraps up the ClientClient from the
gRPC definition. The client [connects](csr-protocol/src/client.rs#L25) to the
server address, and can send messages on it.

To receive messages from the server, the client has to proactive register with
the server to receive events. This is done through initially calling the
[server\_events](csr-protocol/src/client.rs#L107) method. This returns the
stream object that the server sent. Tonic abstracts this all away, but this is
like receiving the other side of the first channel described above - the channel
that is created to represent a stream.

The client creates two threads to handle communication with the server. The first
thread is spawned that then just [waits](csr-protocol/src/client.rs#L110)
for messages from the server. Whenever a message is received, it puts that
message on a channel to the [second thread](csr-protocol/src/client.rs#L70).

This second thread looks at what type of message is being
[received](csr-protocol/src/client.rs#L127), and delegates this to a listener
that implements [ServerEvent](csr-protocol/src/event.rs#L14).
This is where the illusion of the function call is bound - the server and client
implement this same trait, and here is where the client calls out to this trait
to complete the function call.

The implementer of this trait is in the [client itself](csr-client/src/game.rs#L21).
For ping, the ping function is called, prints out a log and returns a string.

Back in the [protocol](csr-protocol/src/client.rs#L136) this value is received
from calling this trait, encoded back into protobuf and returned.

The client then calls [respond\_to\_server\_request](csr-protocol/src/client.rs#L91)
which Tonic dispatches back to the server.

## Client flow
Recapping from above, the client path looks like:
* [Receive](csr-protocol/src/client.rs#110) a message from the server
* [Pass](csr-protocol/src/client.rs#112) to another thread for responding
* [Receive](csr-protocol/src/client.rs#70) internally
* [Classify](csr-protocol/src/client.rs#L134) the message type
* [Delegate](csr-protocol/src/client.rs#L136) to the ServerEvent trait implementation
* [Process](csr-client/src/game.rs#L28) the message and create a response
* [Return](csr-protocol/src/client.rs#L91) to the server

## Receiving the response on the server
Through Tonic, the server receives this message over the network and Tonic
calls the [respond\_to\_server\_event](csr-protocol/src/server.rs#L128) trait
implementation method on the server. The request is then decoded, and now
our third channel transmitter is invoked. The client has to return to the server
some indicator of its source - here we use the UserID - and this is looked up
in our channel. This is then [transmitted](csr-protocol/src/server.rs#140)
back to the thread where the ServerEventSender is running. This is received
in the [poll](csr-protocol/src/event.rs#L39) method, and this value is now
released. Our ping message is [received](csr-protocol/src/event.rs#L61) just
a line below where it departed originally.

This received message is then returned from the ServerEventSender function, and
this returns back to where we [started](csr-server/src/service.rs#L193) as the
return type of the function we called.

This completes the trip from the server, and just looks like a single method
being called. The return value is what was produced on the client, and flows
simply to the service to be used.

# Adding new server-side methods
When a new method needs to be added to the server to call the client, the following
places need to be edited:
* The [ServerEvent](csr-protocol/src/event.rs#L14) trait to add the definition of the new function
* The [ServerEventSender](csr-protocol/src/event.rs#L52) implementation
* The Client [listener](csr-client/src/game.rs#L21) ServerEvent implementation

Everything else will then accept this, and the method can be implemented on both sides.
If any new types need to be created, this is added to the protobuf and to the
types anti-corruption layer.
