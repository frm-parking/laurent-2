mod codec;
mod err;
mod event;
mod gw;
mod lio;
mod utils;

pub use err::*;
pub use event::*;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
pub use gw::*;
pub use lio::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
	let laurent_stream = TcpStream::connect("94.141.183.146:44105").await?;
	let gw = Arc::new(StreamGateway::connect(laurent_stream));
	gw.cfg_event(EventKind::Time, false).await?;
	gw.ping().await?;
	gw.authorize("Laurent").await?;
	gw.cfg_event(EventKind::Ein, true).await?;

	println!("Module configured");

	let relay = Relay::new(gw.clone(), 1);
	let t1 = tokio::spawn(async move {
		println!("Relay 1");
		relay.programmatic_click(Duration::from_secs(10)).await
	});

	let relay = Relay::new(gw.clone(), 2);
	let t2 = tokio::spawn(async move {
		println!("Relay 2");
		relay.programmatic_click(Duration::from_secs(10)).await
	});

	let mut line = InputLine::new(gw.clone(), 1);
	let t3: JoinHandle<Result<()>> = tokio::spawn(async move {
		println!("Line");
		loop {
			let signal = line.wait_signal().await?;
			println!("{signal:?}");
		}
	});

	let tasks = FuturesUnordered::from_iter([t1, t2, t3]);
	let _: Vec<_> = tasks.collect().await;

	Ok(())
}
