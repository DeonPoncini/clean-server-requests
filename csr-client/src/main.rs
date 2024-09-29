#[macro_use] extern crate log;

use std::path::Path;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::sync::Arc;

use clap::Parser;

use csr_protocol::client::CleanClient;
use csr_protocol::types::Result;
use csr_protocol::types::{
    SessionID, SessionType, UserID,
};

mod game;

use game::{Game, read_input};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    address: String,
    #[arg(short, long)]
    uid: u64,
    #[arg(short, long)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // initialize logger
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level());
            let filename = Path::new(record.file().unwrap_or("unknown"))
                .file_name().unwrap_or(OsStr::new("unknown")).to_str()
                .unwrap_or("unknown");
            writeln!(
                buf,
                "{level_style}{} [{}]:{}\t{}{level_style:#}",
                record.level(),
                filename,
                record.line().unwrap_or(0),
                record.args())
        })
        .init();

    let cli = Cli::parse();
    let uid = UserID(cli.uid);
    let username = cli.name.clone();

    // connect to the server
    let mut client = CleanClient::new(&cli.address).await?;

    println!("Connected to server at {}", cli.address);
    println!("Type ? for help");
    // main execution loop
    loop {
        let input = read_input(">")?;

        if input == "h" {
            // host a new session
            let st = read_input("Session type [c or d]:")?;
            let pc = read_input("Player count [1-255]:")?;
            let session_type;
            if st == "c" {
                session_type = SessionType::Coin;
            } else if st == "d" {
                session_type = SessionType::Dice;
            } else {
                println!("Invalid session type {}", st);
                println!("Enter c for Coin game");
                println!("or d for Dice game");
                continue;
            }
            let player_count: u8;
            if let Ok(pcu8) = pc.parse() {
                if pcu8 > 0 {
                    player_count = pcu8;
                } else {
                    println!("Player count cannot be zero");
                    continue;
                }
            } else {
                println!("Invalid player count {}", pc);
                println!("Valid values are between 1 and 255");
                continue;
            }
            let sd = client.host_session(session_type, player_count).await?;
            println!("Hosting session: {}", sd.session_id().0);
            println!("Use j command to join this session");
        } else if input == "l" {
            let sessions = client.list_sessions().await?;
            for sd in sessions {
                println!("---");
                println!("Session {} Type {:?}", sd.session_id().0, sd.session_type());
                println!("User count: {}", sd.users().len());
                for u in sd.users() {
                    print!("{},", u);
                }
                if sd.users().len() > 0 {
                    println!("");
                }
            }
        } else if input == "j" {
            let si = read_input("Session ID:")?;
            let sid: u64;
            if let Ok(siu64) = si.parse() {
                sid = siu64;
            } else {
                println!("Invalid session ID: {}, please enter a valid u64", si);
                continue;
            }
            let session_id = SessionID(sid);
            // start listening to the server events
            let listener = Arc::new(Game::new());
            let handle = client.server_events_listen(session_id, uid, listener).await?;
            // join the session
            client.join_session(session_id, uid, &username).await?;

            // wait for the game to end
            if let (Err(e),) = futures::try_join!(handle)? {
                error!("Game exited with error {:?}", e);
            }
            println!("Game over");
        } else if input == "q" {
            break;
        } else if input == "?" {
            print_help();
        } else {
            println!("Unknown input: {}", input);
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("h\thost a session");
    println!("l\tlist sessions");
    println!("j\tjoin a session");
    println!("q\tquit");
    println!("?\tprint this menu");
}

