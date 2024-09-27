#[macro_use] extern crate log;

use std::path::Path;
use std::ffi::OsStr;
use std::io::Write;

use tonic::transport::Server;
use tonic_web::GrpcWebLayer;

use csr_protocol::server::make_server;
use csr_protocol::types::Result;

mod error;
mod service;

use service::CleanService;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:5555".parse().expect("IP address malformed");

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

    let s = CleanService::new();

    trace!("Clean service listening on {}", addr);

    Server::builder()
        .accept_http1(true)
        .layer(GrpcWebLayer::new())
        .add_service(make_server(s))
        .serve(addr)
        .await?;

    Ok(())
}
