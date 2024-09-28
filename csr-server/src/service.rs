use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::Rng;
use tokio::sync::RwLock;

use csr_protocol::event::{ServerEvent, ServerEventSender};
use csr_protocol::server::Clean;
use csr_protocol::types::Result;
use csr_protocol::types::{
    Coin, SessionData, SessionID, SessionType, UserID,
};

use crate::error::Error;

#[derive(Clone)]
pub struct UserData {
    pub name: String,
}

pub struct SessionState {
    pub player_count: u8,
    pub users: HashMap<UserID, UserData>,
    pub session_type: SessionType,

    pub server_event_senders: HashMap<UserID, ServerEventSender>,
}

pub type Session = Arc<RwLock<SessionState>>;

pub struct Callback {
    senders: HashMap<UserID, ServerEventSender>,
}

impl Callback {
    pub fn new() -> Self {
        Self {
            senders: HashMap::new(),
        }
    }

    pub fn attach(&mut self, uid: UserID, s: ServerEventSender) {
        self.senders.insert(uid, s);
    }

    pub fn route(&self, uid: UserID) -> Result<&ServerEventSender> {
        Ok(self.senders.get(&uid).ok_or_else(|| Box::new(Error::ClientUnreachable(uid)))?)
    }
}

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
    async fn host_session(&self, typ: SessionType, player_count: u8)
            -> Result<SessionData> {
        let sid = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let session_id = SessionID(sid);

        // create state for a session
        let session = Arc::new(RwLock::new(SessionState {
            player_count: player_count,
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
            name: user_name.to_owned(),
        };
        s.write().await.users.insert(uid, ud);

        // check if we have enough players to start the game
        if s.read().await.users.len() as u8 == s.read().await.player_count {
            info!("Game is starting for session {:?}", sid);
            let session = self.get_session(sid).await?;
            game_setup(session).await;
        }

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

async fn game_setup(session: Session) {
    match game_setup_impl(session.clone()).await {
        Ok(_) => { info!("Game complete"); }
        Err(e) => {
            error!("Unable to start game {:?}", e);
            report_error(session, e).await;
        }
    }
}

async fn game_setup_impl(session: Session) -> Result<()> {
    // read the values out of the session
    let users = session.read().await.users.clone();
    let session_type = session.read().await.session_type;
    // load up the senders
    let mut cb = Callback::new();
    for (uid, _) in &users {
        if let Some(ses) = session.write().await.server_event_senders.remove(uid) {
            cb.attach(*uid, ses);
        }
    }

    // run the game
    let handle = tokio::spawn(async move {
        match game_thread(users, session_type, cb).await {
            Ok(r) => Ok(r),
            Err(e) => Err(e),
        }
    });
    match handle.await {
        Ok(_) => { info!("Game complete"); }
        Err(e) => { report_error(session, e).await; }
    }
    Ok(())
}

async fn game_thread(users: HashMap<UserID, UserData>,
                     session_type: SessionType, cb: Callback) -> Result<()> {
    loop {
        // ping the players and get their response
        for (uid,_) in &users {
            let msg = cb.route(*uid)?.ping("Game start").await?;
            info!("Received ping response: {} from {:?}", msg, uid);
        }

        // depending on the session type, take different actions
        let winner = match session_type {
            SessionType::Dice => dice_game(&users, &cb).await?,
            SessionType::Coin => coin_game(&users, &cb).await?,
        };

        // get the name of the winner
        let username;
        if let Some(ud) = users.get(&winner) {
            username = ud.name.clone();
        } else {
            return Err(Box::new(Error::UnknownWinner));
        }

        // let everyone know who the winner is
        for (uid,_) in &users {
            cb.route(*uid)?.winner(winner, &username).await?;
        }

        // ask if people want to play again, only continue if everyone
        // votes yes
        let mut play_again = true;
        for (uid, _) in &users {
            play_again = play_again & cb.route(*uid)?.try_again().await?;
        }
        if !play_again {
            break;
        }
    }

    Ok(())
}

async fn dice_game(users: &HashMap<UserID, UserData>,
                   cb: &Callback) -> Result<UserID> {
    // pick how many sides the dice have, out of 4, 6, 8, 12, and 20
    let sides_array = vec![4, 6, 8, 12, 20];
    let sides_index = rand::thread_rng().gen_range(0..sides_array.len());
    let sides = sides_array[sides_index];
    // pick a random number of dice to roll, between 1 and 6
    let count = rand::thread_rng().gen_range(1..=6);
    let mut results = Vec::new();
    for _ in 0..count {
        let roll = rand::thread_rng().gen_range(1..=sides);
        results.push(roll);
    }
    // ask each user for their rolls
    let mut winner = None;
    let mut winner_score = 0;
    for (uid, _) in users {
        let guess = cb.route(*uid)?.roll_dice(sides, count).await?;
        let mut score = 0;
        for g in guess {
            if results.contains(&g) {
                score = score + 1;
            }
        }
        if score >= winner_score {
            winner_score = score;
            winner = Some(*uid);
        }
    }
    match winner {
        Some(w) => { return Ok(w); }
        None => { return Err(Box::new(Error::UnknownWinner)); }
    }
}

async fn coin_game(users: &HashMap<UserID, UserData>,
                   cb: &Callback) -> Result<UserID> {
    // pick how many coins to flip between 1 and 6
    let count = rand::thread_rng().gen_range(1..=6);
    // flip coins
    let mut results = Vec::new();
    for _ in 0..count {
        let flip = rand::thread_rng().gen_range(0..=1);
        if flip == 0 {
            results.push(Coin::Heads);
        } else {
            results.push(Coin::Tails);
        }
    }
    let mut winner = None;
    let mut winner_score = 0;
    for (uid, _) in users {
        let result = cb.route(*uid)?.flip_coin(count).await?;
        let mut score = 0;
        for x in 0..result.len() {
            if x > results.len() { break; }
            if results[x] == result[x] {
                score = score + 1;
            }
        }
        if score >= winner_score {
            winner_score = score;
            winner = Some(*uid);
        }
    }
    match winner {
        Some(w) => { return Ok(w); }
        None => { return Err(Box::new(Error::UnknownWinner)); }
    }
}

async fn report_error(session: Session, ew: impl std::fmt::Display) {
    let users = session.read().await.users.clone();
    for (uid, _) in users {
        match session.read().await.server_event_senders.get(&uid) {
            Some(ses) => {
                match ses.error(&format!("{}", ew)).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to send error to user {:?}: [{}], due to {:?}",
                               uid, ew, e);
                    }
                }
            }
            None => {
                error!("Failed to send error to user {:?}: {}, no sender",
                       uid, ew);
            }
        }
    }
}
