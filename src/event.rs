use crate::as_match;
use crate::lio::Signal;
use crate::Error;
use crate::Result;
use std::fmt::Display;
use std::fmt::Formatter;
use tokio::sync::broadcast::Receiver;

#[derive(Debug)]
pub enum EventKind {
	Ein,
	Time,
	Rele,
	In,
	Out,
	Advc,
	Pwm,
	Wt1,
}

impl Display for EventKind {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Ein => write!(f, "EIN"),
			Self::Time => write!(f, "TIME"),
			Self::Rele => write!(f, "RELE"),
			Self::In => write!(f, "IN"),
			Self::Out => write!(f, "OUT"),
			Self::Advc => write!(f, "ADVC"),
			Self::Pwm => write!(f, "PWM"),
			Self::Wt1 => write!(f, "1WT"),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
	Ein { line: u32, signal: Signal },
	Time(u32),
}

impl Event {
	pub fn try_from_parts(msg: &[String]) -> Result<Self> {
		let event = match as_match!(msg) {
			["EIN", line, signal] => Self::Ein {
				line: line.parse()?,
				signal: signal.parse()?,
			},
			["TIME", time] => Self::Time(time.parse()?),
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
