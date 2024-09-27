use csr_protocol::types::{SessionID, UserID};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Session not found {0:?}")]
    SessionNotFound(SessionID),
    #[error("User {0:?} already in session {0:?}")]
    UserAlreadyInSession(UserID, SessionID),
    #[error("User {0:?} not in session {0:?}")]
    UserNotInSession(UserID, SessionID),
}
