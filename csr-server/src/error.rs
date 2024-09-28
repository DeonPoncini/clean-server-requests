use csr_protocol::types::{SessionID, UserID};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client unreachable {0:?}")]
    ClientUnreachable(UserID),
    #[error("Session not found {0:?}")]
    SessionNotFound(SessionID),
    #[error("Winner is unknown")]
    UnknownWinner,
    #[error("User {0:?} already in session {0:?}")]
    UserAlreadyInSession(UserID, SessionID),
    #[error("User {0:?} not in session {0:?}")]
    UserNotInSession(UserID, SessionID),
}
