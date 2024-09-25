#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SessionID(pub u64);

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UserID(pub u64);

use crate::error::Error;

// import the protobuf types
use crate::clean;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionType {
    Dice,
    Coin,
}

impl TryFrom<i32> for SessionType {
    type Error = Error;

    fn try_from(proto: i32) -> Result<Self, Self::Error> {
        if proto == clean::SessionType::Dice as i32 {
            return Ok(SessionType::Dice);
        } else if proto == clean::SessionType::Coin as i32 {
            return Ok(SessionType::Coin);
        } else {
            return Err(Error::InvalidSessionType);
        }
    }
}

impl From<SessionType> for clean::SessionType {
    fn from(st: SessionType) -> Self {
        match st {
            SessionType::Dice => clean::SessionType::Dice,
            SessionType::Coin => clean::SessionType::Coin,
        }
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coin {
    Heads,
    Tails,
}

impl TryFrom<i32> for Coin {
    type Error = Error;

    fn try_from(proto: i32) -> Result<Self, Self::Error> {
        if proto == clean::Coin::Heads as i32 {
            return Ok(Coin::Heads);
        } else if proto == clean::Coin::Tails as i32 {
            return Ok(Coin::Tails);
        } else {
            return Err(Error::InvalidCoinValue);
        }
    }
}

impl From<Coin> for clean::Coin {
    fn from(c: Coin) -> Self {
        match c {
            Coin::Heads => clean::Coin::Heads,
            Coin::Tails => clean::Coin::Tails,
        }
    }
}

pub struct HostInfo {
    typ: SessionType,
}

impl HostInfo {
    pub fn new(typ: SessionType) -> Self {
        Self {
            typ: typ
        }
    }

    pub fn session_type(&self) -> SessionType { self.typ }
}

impl TryFrom<clean::HostInfo> for HostInfo {
    type Error = Error;

    fn try_from(proto: clean::HostInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            typ: proto.r#type.try_into()?,
        })
    }
}

impl From<HostInfo> for clean::HostInfo {
    fn from(hi: HostInfo) -> Self {
        let t: clean::SessionType = hi.typ.into();
        Self {
            r#type: t.into(),
        }
    }
}

#[derive(Clone)]
pub struct SessionData {
    sid: SessionID,
    typ: SessionType,
    users: Vec<String>,
}

impl SessionData {
    pub fn new(sid: SessionID, typ: SessionType, users: &[String]) -> Self {
        Self {
            sid: sid,
            typ: typ,
            users: users.to_vec(),
        }
    }

    pub fn session_id(&self) -> SessionID { self.sid }
    pub fn session_type(&self) -> SessionType { self.typ }
    pub fn users<'a>(&'a self) -> &'a [String] { &self.users }
}

impl TryFrom<clean::SessionData> for SessionData {
    type Error = Error;

    fn try_from(proto: clean::SessionData) -> Result<Self, Self::Error> {
        Ok(Self {
            sid: SessionID(proto.session_id),
            typ: proto.r#type.try_into()?,
            users: proto.users,
        })
    }
}

impl From<SessionData> for clean::SessionData {
    fn from(sd: SessionData) -> Self {
        let t: clean::SessionType = sd.typ.into();
        Self {
            session_id: sd.sid.0,
            r#type: t.into(),
            users: sd.users,
        }
    }
}

pub struct Sessions {
    data: Vec<SessionData>,
}

impl Sessions {
    pub fn new(data: &[SessionData]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    pub fn sessions<'a>(&'a self) -> &'a [SessionData] { &self.data }
}

impl TryFrom<clean::Sessions> for Sessions {
    type Error = Error;

    fn try_from(proto: clean::Sessions) -> Result<Self, Self::Error> {
        let mut data = Vec::new();
        for d in proto.data {
            data.push(d.try_into()?);
        }
        Ok(Self {
            data: data,
        })
    }
}

impl From<Sessions> for clean::Sessions {
    fn from(s: Sessions) -> Self {
        Self {
            data: s.data.iter().map(|d| d.clone().into()).collect(),
        }
    }
}

pub struct JoinInfo {
    sid: SessionID,
    uid: UserID,
    user_name: String,
}

impl JoinInfo {
    pub fn new(sid: SessionID, uid: UserID, user_name: &str) -> Self {
        Self {
            sid: sid,
            uid: uid,
            user_name: user_name.to_owned(),
        }
    }

    pub fn session_id(&self) -> SessionID { self.sid }
    pub fn user_id(&self) -> UserID { self.uid }
    pub fn user_name<'a>(&'a self) -> &'a str { &self.user_name }
}

impl From<clean::JoinInfo> for JoinInfo {
    fn from(proto: clean::JoinInfo) -> Self {
        Self {
            sid: SessionID(proto.session_id),
            uid: UserID(proto.user_id),
            user_name: proto.user_name,
        }
    }
}

impl From<JoinInfo> for clean::JoinInfo {
    fn from(ji: JoinInfo) -> Self {
        Self {
            session_id: ji.sid.0,
            user_id: ji.uid.0,
            user_name: ji.user_name,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EventRegister {
    sid: SessionID,
    uid: UserID,
}

impl EventRegister {
    pub fn new(sid: SessionID, uid: UserID) -> Self {
        Self {
            sid: sid,
            uid: uid,
        }
    }

    pub fn session_id(&self) -> SessionID { self.sid }
    pub fn user_id(&self) -> UserID { self.uid }
}

impl From<clean::EventRegister> for EventRegister {
    fn from(proto: clean::EventRegister) -> Self {
        Self {
            sid: SessionID(proto.session_id),
            uid: UserID(proto.user_id),
        }
    }
}

impl From<EventRegister> for clean::EventRegister {
    fn from(er: EventRegister) -> Self {
        Self {
            session_id: er.sid.0,
            user_id: er.uid.0,
        }
    }
}

pub struct Ping {
    text: String,
}

impl Ping {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
        }
    }

    pub fn text<'a>(&'a self) -> &'a str { &self.text }
}

impl From<clean::Ping> for Ping {
    fn from(proto: clean::Ping) -> Self {
        Self {
            text: proto.text,
        }
    }
}

impl From<Ping> for clean::Ping {
    fn from(p: Ping) -> Self {
        Self {
            text: p.text,
        }
    }
}

pub struct Pong {
    text: String,
}

impl Pong {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
        }
    }

    pub fn text<'a>(&'a self) -> &'a str { &self.text }
}

impl From<clean::Pong> for Pong {
    fn from(proto: clean::Pong) -> Self {
        Self {
            text: proto.text,
        }
    }
}

impl From<Pong> for clean::Pong {
    fn from(p: Pong) -> Self {
        Self {
            text: p.text,
        }
    }
}

pub struct RollDice {
    sides: u8,
    count: u8,
}

impl RollDice {
    pub fn new(sides: u8, count: u8) -> Self {
        Self {
            sides: sides,
            count: count,
        }
    }

    pub fn sides(&self) -> u8 { self.sides }
    pub fn count(&self) -> u8 { self.count }
}

impl From<clean::RollDice> for RollDice {
    fn from(proto: clean::RollDice) -> Self {
        Self {
            sides: proto.sides as u8,
            count: proto.count as u8,
        }
    }
}

impl From<RollDice> for clean::RollDice {
    fn from(rd: RollDice) -> Self {
        Self {
            sides: rd.sides as u32,
            count: rd.count as u32,
        }
    }
}

pub struct FlipCoin {
    count: u8,
}

impl FlipCoin {
    pub fn new(count: u8) -> Self {
        Self {
            count: count,
        }
    }

    pub fn count(&self) -> u8 { self.count }
}

impl From<clean::FlipCoin> for FlipCoin {
    fn from(proto: clean::FlipCoin) -> Self {
        Self {
            count: proto.count as u8,
        }
    }
}

impl From<FlipCoin> for clean::FlipCoin {
    fn from(fc: FlipCoin) -> Self {
        Self {
            count: fc.count as u32,
        }
    }
}

pub struct DiceGuess {
    number: Vec<u8>,
}

impl DiceGuess {
    pub fn new(number: &[u8]) -> Self {
        Self {
            number: number.to_vec(),
        }
    }

    pub fn number<'a>(&'a self) -> &'a [u8] { &self.number }
}

impl From<clean::DiceGuess> for DiceGuess {
    fn from(proto: clean::DiceGuess) -> Self {
        Self {
            number: proto.number.iter().map(|n| *n as u8).collect(),
        }
    }
}

impl From<DiceGuess> for clean::DiceGuess {
    fn from(dg: DiceGuess) -> Self {
        Self {
            number: dg.number.iter().map(|n| *n as u32).collect(),
        }
    }
}

pub struct CoinGuess {
    coins: Vec<Coin>,
}

impl CoinGuess {
    pub fn new(coins: &[Coin]) -> Self {
        Self {
            coins: coins.to_vec(),
        }
    }

    pub fn coins<'a>(&'a self) -> &'a [Coin] { &self.coins }
}

impl TryFrom<clean::CoinGuess> for CoinGuess {
    type Error = Error;

    fn try_from(proto: clean::CoinGuess) -> Result<Self, Self::Error> {
        let mut coins = Vec::new();
        for coin in proto.coins {
            coins.push(coin.try_into()?);
        }
        Ok(Self {
            coins: coins,
        })
    }
}

impl From<CoinGuess> for clean::CoinGuess {
    fn from(cg: CoinGuess) -> Self {
        Self {
            coins: cg.coins.iter()
                .map(|c| c.clone().into())
                .map(|c: clean::Coin| c.into())
                .collect(),
        }
    }
}

pub struct Winner {
    uid: UserID,
    name: String,
}

impl Winner {
    pub fn new(uid: UserID, name: &str) -> Self {
        Self {
            uid: uid,
            name: name.to_owned(),
        }
    }

    pub fn user_id(&self) -> UserID { self.uid }
    pub fn user_name<'a>(&'a self) -> &'a str { &self.name }
}

impl From<clean::Winner> for Winner {
    fn from(proto: clean::Winner) -> Self {
        Self {
            uid: UserID(proto.user_id),
            name: proto.user_name,
        }
    }
}

impl From<Winner> for clean::Winner {
    fn from(w: Winner) -> Self {
        Self {
            user_id: w.uid.0,
            user_name: w.name,
        }
    }
}

pub enum ServerRequest {
    JoinInfo(JoinInfo),
    Ping(Ping),
    RollDice(RollDice),
    FlipCoin(FlipCoin),
    Winner(Winner),
    TryAgain(bool),
}

impl TryFrom<clean::ServerRequest> for ServerRequest {
    type Error = Error;

    fn try_from(proto: clean::ServerRequest) -> Result<Self, Self::Error> {
        let msg = proto.msg.ok_or_else(|| Error::InvalidServerRequest)?;
        match msg {
            clean::server_request::Msg::UserJoined(ji) =>
                return Ok(ServerRequest::JoinInfo(ji.into())),
            clean::server_request::Msg::Ping(p) =>
                return Ok(ServerRequest::Ping(p.into())),
            clean::server_request::Msg::Dice(rd) =>
                return Ok(ServerRequest::RollDice(rd.into())),
            clean::server_request::Msg::Coin(fc) =>
                return Ok(ServerRequest::FlipCoin(fc.into())),
            clean::server_request::Msg::Winner(w) =>
                return Ok(ServerRequest::Winner(w.into())),
            clean::server_request::Msg::TryAgain(t) =>
                return Ok(ServerRequest::TryAgain(t)),
        }
    }
}

impl From<ServerRequest> for clean::ServerRequest {
    fn from(sr: ServerRequest)  -> Self {
        let msg = match sr {
            ServerRequest::JoinInfo(ji) =>
                clean::server_request::Msg::UserJoined(ji.into()),
            ServerRequest::Ping(p) =>
                clean::server_request::Msg::Ping(p.into()),
            ServerRequest::RollDice(rd) =>
                clean::server_request::Msg::Dice(rd.into()),
            ServerRequest::FlipCoin(fc) =>
                clean::server_request::Msg::Coin(fc.into()),
            ServerRequest::Winner(w) =>
                clean::server_request::Msg::Winner(w.into()),
            ServerRequest::TryAgain(t) =>
                clean::server_request::Msg::TryAgain(t.into()),
        };
        Self {
            msg: Some(msg),
        }
    }
}

pub enum ClientResponse {
    Pong(Pong),
    DiceGuess(DiceGuess),
    CoinGuess(CoinGuess),
    Again(bool),
}

impl TryFrom<clean::ClientResponse> for ClientResponse {
    type Error = Error;

    fn try_from(proto: clean::ClientResponse) -> Result<Self, Self::Error> {
        let msg = proto.msg.ok_or_else(|| Error::InvalidClientResponse)?;
        match msg {
            clean::client_response::Msg::Pong(p) =>
                return Ok(ClientResponse::Pong(p.into())),
            clean::client_response::Msg::DiceGuess(dg) =>
                return Ok(ClientResponse::DiceGuess(dg.into())),
            clean::client_response::Msg::CoinGuess(cg) =>
                return Ok(ClientResponse::CoinGuess(cg.try_into()?)),
            clean::client_response::Msg::Again(a) =>
                return Ok(ClientResponse::Again(a)),
        }
    }
}

impl From<ClientResponse> for clean::ClientResponse {
    fn from(cr: ClientResponse) -> Self {
        let msg = match cr {
            ClientResponse::Pong(p) =>
                clean::client_response::Msg::Pong(p.into()),
            ClientResponse::DiceGuess(dg) =>
                clean::client_response::Msg::DiceGuess(dg.into()),
            ClientResponse::CoinGuess(cg) =>
                clean::client_response::Msg::CoinGuess(cg.into()),
            ClientResponse::Again(a) =>
                clean::client_response::Msg::Again(a),
        };
        Self {
            msg: Some(msg),
        }
    }
}
