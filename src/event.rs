use tokio::sync::broadcast::Receiver;
use crate::as_match;
use crate::lio::Signal;
use crate::Error;
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
	Ein { line: u32, signal: Signal },
}

impl Event {
	pub fn try_from_parts(msg: &[String]) -> Result<Self> {
		let event = match as_match!(msg) {
			["EIN", line, signal] => Self::Ein {
				line: line.parse()?,
				signal: signal.parse()?,
			},
			_ => Err(Error::UnknownMessage)?,
		};

		Ok(event)
	}
}

impl TryFrom<&[String]> for Event {
	type Error = Error;

	fn try_from(value: &[String]) -> Result<Self, Self::Error> {
		Self::try_from_parts(value)
	}
}

pub type EventReceiver = Receiver<Event>;
