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
        info!("Received ping message: {}", ping);
        Ok("pong".to_owned())
    }
    async fn roll_dice(&self, sides: u8, count: u8) -> Result<Vec<u8>> {
        let mut ret = Vec::new();
        for x in 0..count {
            let input = read_input(
                &format!("Guess the value of die {} with {} sides:", x, sides))?;
            let value: u8 = input.parse()?;
            ret.push(value);
        }
        Ok(ret)
    }
    async fn flip_coin(&self, count: u8) -> Result<Vec<Coin>> {
        let mut ret = Vec::new();
        for x in 0..count {
            let input = read_input(
                &format!("Guess coin flip {}, h or t", x))?;
            if input == "h" {
                ret.push(Coin::Heads);
            } else {
                ret.push(Coin::Tails);
            }
        }
        Ok(ret)
    }
    async fn winner(&self, uid: UserID, name: &str) -> Result<()> {
        info!("Winner: [{}] {}", uid.0, name);
        Ok(())
    }
    async fn try_again(&self) -> Result<bool> {
        let again = read_input("Try again? [y/n]")?;
        if again == "y" {
            return Ok(true);
        }
        Ok(false)
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
