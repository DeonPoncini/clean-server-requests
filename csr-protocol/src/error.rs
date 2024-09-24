#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid session type")]
    InvalidSessionType,
    #[error("Invalid coin value")]
    InvalidCoinValue,
}
