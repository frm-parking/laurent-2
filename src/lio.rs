use crate::Error;
use crate::Event;
use crate::EventReceiver;
use crate::Gateway;
use crate::Result;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
	High,
	Low,
}

impl Signal {
	pub fn from_bool(value: bool) -> Self {
		match value {
			true => Self::High,
			false => Self::Low,
		}
	}

	pub fn parse(value: &str) -> Result<Self> {
		match value {
			"1" => Ok(Self::High),
			"0" => Ok(Self::Low),
			val => Err(Error::InvalidPayload(format!(
				"The signal level can only be represented as `0` or `1`. Received: `{val}`"
			))),
		}
	}
}

impl Display for Signal {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::High => write!(f, "1"),
			Self::Low => write!(f, "0"),
		}
	}
}

impl From<bool> for Signal {
	fn from(value: bool) -> Self {
		Self::from_bool(value)
	}
}

impl FromStr for Signal {
	type Err = Error;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		Self::parse(s)
	}
}

#[derive(Debug)]
pub enum RelayAction {
	On,
	Off,
	Toggle,
}

impl Display for RelayAction {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::On => write!(f, "1"),
			Self::Off => write!(f, "0"),
			Self::Toggle => write!(f, "2"),
		}
	}
}

#[derive(Debug)]
pub enum ClickDelay {
	Millis100(u32),
	Seconds(u32),
}

impl Display for ClickDelay {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Millis100(m100) => write!(f, ".{m100}"),
			Self::Seconds(secs) => write!(f, "{secs}"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Relay {
	line: u32,
	gw: Arc<dyn Gateway + Send + Sync + 'static>,
}

impl Relay {
	pub fn new(gw: Arc<dyn Gateway + Send + Sync + 'static>, line: u32) -> Self {
		Self { gw, line }
	}

	pub async fn status(&self) -> Result<bool> {
		self.gw.relay_status(self.line).await
	}

	pub async fn on(&self) -> Result<()> {
		self.gw.relay(self.line, RelayAction::On, None).await
	}

	pub async fn off(&self) -> Result<()> {
		self.gw.relay(self.line, RelayAction::Off, None).await
	}

	pub async fn toggle(&self) -> Result<()> {
		self.gw.relay(self.line, RelayAction::Toggle, None).await
	}

	pub async fn click(&self, delay: ClickDelay) -> Result<()> {
		self.gw.relay(self.line, RelayAction::On, Some(delay)).await
	}

	pub async fn programmatic_click(&self, duration: Duration) -> Result<()> {
		self.on().await?;
		tokio::time::sleep(duration).await;
		self.off().await
	}
}

#[derive(Debug)]
pub struct InputLine {
	line: u32,
	sub: EventReceiver,
}

impl InputLine {
	pub fn new(gw: Arc<dyn Gateway + Send + 'static>, line: u32) -> Self {
		Self {
			sub: gw.subscibe(),
			line,
		}
	}

	pub async fn wait_signal(&mut self) -> Result<Signal> {
		loop {
			match self.sub.recv().await? {
				Event::Ein { line, signal } if line == self.line => return Ok(signal),
				_ => (),
			}
		}
	}
}
