use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] std::io::Error),

	#[error(transparent)]
	ParseInt(#[from] std::num::ParseIntError),

	#[error(transparent)]
	Recv(#[from] tokio::sync::broadcast::error::RecvError),

	#[error("Failed to send command")]
	Send,

	#[error("Channel closed")]
	Closed,

	#[error("Message syntax error")]
	SyntaxError,

	#[error("Unknown message")]
	UnknownMessage,

	#[error("Invalid payload")]
	InvalidPayload(String),

	#[error("Authorization failed")]
	Auth,
}
