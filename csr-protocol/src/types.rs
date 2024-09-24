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
