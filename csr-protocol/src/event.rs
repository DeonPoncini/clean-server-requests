use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use crate::error::Error;
use crate::types::Result;
use crate::types::{
    ClientResponse, Coin, FlipCoin, JoinInfo, Ping, RollDice, ServerRequest,
    SessionID, UserID, Winner,
};

#[tonic::async_trait]
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

pub struct ServerEventSender {
    tx: Sender<ServerRequest>,
    rx: Arc<Mutex<Receiver<ClientResponse>>>,
}

impl ServerEventSender {
    pub fn new(tx: Sender<ServerRequest>, rx: Receiver<ClientResponse>) -> Self {
        Self {
            tx: tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    // wait for client messages
    async fn poll(&self) -> Result<ClientResponse> {
        let r = self.rx.lock().await.recv().await.ok_or_else(
            || Error::ClientDisconnected)?;
        match r {
            ClientResponse::ClientError(e) => {
                return Err(Box::new(Error::ClientError(e)));
            }
            _ => Ok(r),
        }
    }
}

#[tonic::async_trait]
impl ServerEvent for ServerEventSender {
    async fn join_info(&self, sid: SessionID, uid: UserID, user_name: &str)
            -> Result<()> {
        let ji = JoinInfo::new(sid, uid, user_name);
        Ok(self.tx.send(ServerRequest::JoinInfo(ji)).await?)
    }
    async fn ping(&self, ping: &str) -> Result<String> {
        let p = Ping::new(ping);
        self.tx.send(ServerRequest::Ping(p)).await?;
        if let ClientResponse::Pong(p) = self.poll().await? {
            return Ok(p.text().to_owned());
        } else {
            return Err(Error::InvalidClientResponse)?;
        }
    }
    async fn roll_dice(&self, sides: u8, count: u8) -> Result<Vec<u8>> {
        let r = RollDice::new(sides, count);
        self.tx.send(ServerRequest::RollDice(r)).await?;
        if let ClientResponse::DiceGuess(d) = self.poll().await? {
            return Ok(d.number().to_vec());
        } else {
            return Err(Error::InvalidClientResponse)?;
        }
    }
    async fn flip_coin(&self, count: u8) -> Result<Vec<Coin>> {
        let f = FlipCoin::new(count);
        self.tx.send(ServerRequest::FlipCoin(f)).await?;
        if let ClientResponse::CoinGuess(c) = self.poll().await? {
            return Ok(c.coins().to_vec());
        } else {
            return Err(Error::InvalidClientResponse)?;
        }
    }
    async fn winner(&self, uid: UserID, name: &str) -> Result<()> {
        let w = Winner::new(uid, name);
        Ok(self.tx.send(ServerRequest::Winner(w)).await?)
    }
    async fn try_again(&self) -> Result<bool> {
        self.tx.send(ServerRequest::TryAgain(true)).await?;
        if let ClientResponse::Again(a) = self.poll().await? {
            return Ok(a);
        } else {
            return Err(Error::InvalidClientResponse)?;
        }
    }
    async fn error(&self, err: &str) -> Result<()> {
        Ok(self.tx.send(ServerRequest::ServerError(err.to_owned())).await?)
    }
}
