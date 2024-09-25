#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client disconnected")]
    ClientDisconnected,
    #[error("Invalid session type")]
    InvalidSessionType,
    #[error("Invalid coin value")]
    InvalidCoinValue,
    #[error("Invalid server request")]
    InvalidServerRequest,
    #[error("Invalid client response")]
    InvalidClientResponse,
}
