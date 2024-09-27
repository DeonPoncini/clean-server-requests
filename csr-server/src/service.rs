use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use csr_protocol::event::ServerEventSender;
use csr_protocol::server::Clean;
use csr_protocol::types::Result;
use csr_protocol::types::{
    SessionData, SessionID, SessionType, UserID,
};

//use crate::session::Sessions;
use crate::error::Error;

#[derive(Clone)]
pub struct UserData {
    pub uid: UserID,
    pub name: String,
}

pub struct SessionState {
    pub sid: SessionID,
    pub users: HashMap<UserID, UserData>,
    pub session_type: SessionType,

    pub server_event_senders: HashMap<UserID, ServerEventSender>,
}

pub type Session = Arc<RwLock<SessionState>>;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub struct CleanService {
    sessions: Arc<RwLock<HashMap<SessionID, Session>>>,
}

impl CleanService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_session(&self, sid: SessionID) -> Result<Session> {
        match self.sessions.read().await.get(&sid) {
            Some(s) => { return Ok(s.clone()); }
            None => { return Err(Box::new(Error::SessionNotFound(sid))); }
        }
    }

    async fn get_session_for_user(&self, sid: SessionID, uid: UserID)
            -> Result<Session> {
        let guard = self.sessions.read().await;
        let s = guard.get(&sid).ok_or_else(|| Error::SessionNotFound(sid))?;
        if s.read().await.users.contains_key(&uid) {
            return Ok(s.clone());
        }
        Err(Box::new(Error::UserNotInSession(uid, sid)))
    }
}

#[tonic::async_trait]
impl Clean for CleanService {
    // client initiated API
    async fn host_session(&self, typ: SessionType) -> Result<SessionData> {
        let sid = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let session_id = SessionID(sid);

        // create state for a session
        let session = Arc::new(RwLock::new(SessionState {
            sid: session_id,
            users: HashMap::new(),
            session_type: typ,
            server_event_senders: HashMap::new(),
        }));

        // store the session
        self.sessions.write().await.insert(session_id, session);

        // return the session info
        Ok(SessionData::new(session_id, typ, &[]))
    }
    async fn list_sessions(&self) -> Result<Vec<SessionData>> {
        let mut ret = Vec::new();
        for (sid, session) in self.sessions.read().await.iter() {
            let s = session.read().await;
            let users: Vec<_> = s.users.iter().map(|(_, ud)| {
                                          ud.name.clone()
                                      }).collect();
            ret.push(SessionData::new(*sid, s.session_type, &users));
        }
        Ok(ret)
    }
    async fn join_session(&self, sid: SessionID, uid: UserID, user_name: &str)
        -> Result<()> {
        let s = self.get_session(sid).await?;

        if s.read().await.users.contains_key(&uid) {
            return Err(Box::new(Error::UserAlreadyInSession(uid, sid)));
        }

        // insert the user in the session
        let ud = UserData {
            uid: uid,
            name: user_name.to_owned(),
        };
        s.write().await.users.insert(uid, ud);

        Ok(())
    }
    // server callbacks
    async fn register_server_event_sender(&self, sid: SessionID, uid: UserID,
                          s: ServerEventSender) -> Result<()> {
        let z = self.get_session_for_user(sid, uid).await?;
        z.write().await.server_event_senders.insert(uid, s);
        Ok(())
    }
}
