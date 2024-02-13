use crate::as_match;
use crate::codec::Codec;
use crate::codec::JoinParts;
use crate::event::Event;
use crate::utils::is_event;
use crate::ClickDelay;
use crate::Error;
use crate::RelayAction;
use crate::Result;
use futures::SinkExt;
use futures::StreamExt;
use std::borrow::Cow;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver as BroadcastReceiver;
use tokio::sync::broadcast::Sender as Broadcaster;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;

#[derive(Debug)]
pub struct Gateway {
	cmd_tx: Sender<Box<dyn JoinParts + Send + 'static>>,
	cmd_rx: Mutex<Receiver<Result<Vec<String>>>>,
	events: Broadcaster<Event>,
}

impl Gateway {
	pub fn connect<T>(stream: T) -> Self
	where
		T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
	{
		let (cmd_tx, mut cmd_tx_rx) = mpsc::channel(1);
		let (cmd_rx_tx, cmd_rx) = mpsc::channel(1);
		let (events, _) = broadcast::channel(1024);

		let event_tx = events.clone();
		tokio::spawn(async move {
			let mut stream = Framed::new(stream, Codec::new());

			loop {
				tokio::select! {
					Some(msg) = stream.next() => {
						match msg.as_deref() {
							Ok([ty, rest @ ..]) if is_event(ty) => {
								if let Ok(event) = Event::try_from(rest) {
									event_tx.send(event).expect("Failed to send event");
								}
							},
							_ => cmd_rx_tx.send(msg).await.expect("Failed to return command response"),
						}
					},
					cmd = cmd_tx_rx.recv() => {
						if let Some(cmd) = cmd {
							stream.send(cmd).await.expect("Failed to send command");
						}
					}
				}
			}
		});

		Self {
			cmd_rx: Mutex::new(cmd_rx),
			cmd_tx,
			events,
		}
	}

	async fn send<T>(&self, cmd: T) -> Result<()>
	where
		T: JoinParts + Send + 'static,
	{
		self.cmd_tx.send(cmd.boxed()).await.map_err(|_| Error::Send)
	}

	async fn recv(&self) -> Result<Vec<String>> {
		match self.cmd_rx.lock().await.recv().await {
			Some(Ok(msg)) => Ok(msg),
			_ => Err(Error::Closed),
		}
	}

	pub fn subscibe(&self) -> BroadcastReceiver<Event> {
		self.events.subscribe()
	}

	pub async fn ping(&self) -> Result<()> {
		self.send(("$KE",)).await?;

		match as_match!(self.recv().await?) {
			["#OK"] => Ok(()),
			["#ERR"] => Err(Error::SyntaxError),
			_ => Err(Error::UnknownMessage),
		}
	}

	pub async fn authorize<'a>(&self, pwd: impl Into<Cow<'a, str>>) -> Result<()> {
		let pwd = pwd.into().into_owned();
		self.send(("$KE", "PSW", "SET", pwd)).await?;

		match as_match!(self.recv().await?) {
			["#PSW", "SET", "OK"] => Ok(()),
			["$PSW", "SET", "ERR"] => Err(Error::Auth),
			["#ERR"] => Err(Error::SyntaxError),
			_ => Err(Error::UnknownMessage),
		}
	}

	pub async fn relay(
		&self,
		relay: u32,
		action: RelayAction,
		delay: Option<ClickDelay>,
	) -> Result<()> {
		match delay {
			None => self.send(("$KE", "REL", relay, action)).await?,
			Some(delay) => self.send(("$KE", "REL", relay, action, delay)).await?,
		}

		match as_match!(self.recv().await?) {
			["#REL", "OK"] => Ok(()),
			["#ERR"] => Err(Error::SyntaxError),
			_ => Err(Error::UnknownMessage),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Signal;
	use tokio::io::AsyncReadExt;
	use tokio::io::AsyncWriteExt;
	use tokio::net::TcpListener;
	use tokio::net::TcpStream;

	#[tokio::test]
	async fn gateway_ping() -> Result<()> {
		let listener = TcpListener::bind("0.0.0.0:0").await?;
		let addr = listener.local_addr()?;

		let gw = Gateway::connect(TcpStream::connect(addr).await?);

		let (mut stream, _) = listener.accept().await.unwrap();

		tokio::spawn(async move {
			let mut buf = vec![0; 64];
			let _ = stream.read(&mut buf).await.unwrap();
			stream.write_all(b"#OK\r\n").await.unwrap();
		});

		gw.ping().await?;

		Ok(())
	}

	#[tokio::test]
	async fn gateway_events() -> Result<()> {
		let listener = TcpListener::bind("0.0.0.0:0").await?;
		let addr = listener.local_addr()?;

		let gw = Gateway::connect(TcpStream::connect(addr).await?);

		let (mut stream, _) = listener.accept().await.unwrap();

		tokio::spawn(async move {
			stream.write_all(b"#M,EIN,1,1\r\n").await.unwrap();
			stream.write_all(b"#M,EIN,1,0\r\n").await.unwrap();
			stream.write_all(b"#M,EIN,2,1\r\n").await.unwrap();
			stream.write_all(b"#M,EIN,2,0\r\n").await.unwrap();
		});

		let mut sub = gw.subscibe();

		let event = sub.recv().await.unwrap();
		assert_eq!(
			event,
			Event::Ein {
				line: 1,
				signal: Signal::High,
			}
		);
		let event = sub.recv().await.unwrap();
		assert_eq!(
			event,
			Event::Ein {
				line: 1,
				signal: Signal::Low,
			}
		);
		let event = sub.recv().await.unwrap();
		assert_eq!(
			event,
			Event::Ein {
				line: 2,
				signal: Signal::High,
			}
		);
		let event = sub.recv().await.unwrap();
		assert_eq!(
			event,
			Event::Ein {
				line: 2,
				signal: Signal::Low,
			}
		);

		Ok(())
	}

	#[tokio::test]
	async fn gateway_relay() -> Result<()> {
		let listener = TcpListener::bind("0.0.0.0:0").await?;
		let addr = listener.local_addr()?;

		let gw = Gateway::connect(TcpStream::connect(addr).await?);

		let (mut stream, _) = listener.accept().await.unwrap();

		tokio::spawn(async move {
			let mut buf = vec![0; 64];
			let _ = stream.read(&mut buf).await.unwrap();
			stream.write_all(b"#REL,OK\r\n").await.unwrap();
		});

		gw.relay(1, RelayAction::On, None).await?;

		Ok(())
	}
}