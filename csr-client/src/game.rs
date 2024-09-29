use std::io::Write;

use async_trait::async_trait;

use csr_protocol::event::ServerEvent;
use csr_protocol::types::Result;
use csr_protocol::types::{
    Coin, SessionID, UserID,
};

pub struct Game;

impl Game {
    pub fn new() -> Self {
        Self {
        }
    }
}

#[async_trait]
impl ServerEvent for Game {
    async fn join_info(&self, sid: SessionID, uid: UserID, user_name: &str)
            -> Result<()> {
        println!("Session [{}]: User [{}]{} has joined this session",
                 sid.0, uid.0, user_name);
        Ok(())
    }
    async fn ping(&self, ping: &str) -> Result<String> {
        println!("Received ping message: {}", ping);
        let reply = read_input("Reply to ping:")?;
        Ok(reply)
    }
    async fn roll_dice(&self, sides: u8, count: u8) -> Result<Vec<u8>> {
        todo!()
    }
    async fn flip_coin(&self, count: u8) -> Result<Vec<Coin>> {
        todo!()
    }
    async fn winner(&self, uid: UserID, name: &str) -> Result<()> {
        todo!()
    }
    async fn try_again(&self) -> Result<bool> {
        todo!()
    }
    async fn error(&self, err: &str) -> Result<()> {
        error!("Server error found: {}", err);
        Ok(())
    }
}

pub fn read_input(prefix: &str) -> Result<String> {
    print!("{} ", prefix);
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // trim white space
    let input = input.trim();
    Ok(input.to_owned())
}
