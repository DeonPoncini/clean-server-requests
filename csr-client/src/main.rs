use std::path::Path;
use std::ffi::OsStr;
use std::io::{Read, Write};

use clap::Parser;

use csr_protocol::client::CleanClient;
use csr_protocol::types::Result;

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

    // connect to the server
    let client = CleanClient::new(&cli.address).await?;

    println!("Connected to server at {}", cli.address);
    println!("Type ? for help");
    // main execution loop
    loop {
        print!("> ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        // trim white space
        let input = input.trim();

        // check values
        if input == "h" {
        } else if input == "l" {
        } else if input == "j" {
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
