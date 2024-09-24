pub mod client;
pub mod event;
pub mod server;
pub mod types;

mod clean {
    tonic::include_proto!("clean");
}
