use crate::Error;
use itertools::Itertools;
use std::cmp;
use std::convert::identity;
use std::fmt::Display;
use std::io;
use std::str;
use tokio_util::bytes::Buf;
use tokio_util::bytes::BufMut;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub trait Serke {
	fn serke(&self) -> Option<String>;
}

impl<T> Serke for T
where
	T: Display,
{
	fn serke(&self) -> Option<String> {
		Some(self.to_string())
	}
}

pub trait JoinParts {
	fn join_parts(&self) -> String;
	fn boxed(self) -> Box<dyn JoinParts + Send + 'static>
	where
		Self: Sized + Send + 'static,
	{
		Box::new(self)
	}
}

impl JoinParts for String {
	fn join_parts(&self) -> String {
		self.into()
	}
}

impl JoinParts for &str {
	fn join_parts(&self) -> String {
		(*self).into()
	}
}

impl<T> JoinParts for &[T]
where
	T: Serke,
{
	fn join_parts(&self) -> String {
		self.iter().filter_map(Serke::serke).join(",")
	}
}

macro_rules! join_parts_impl {
  ($($ty:ident),*) => {
    impl<$($ty),*> JoinParts for ($($ty),*,) where $($ty: Serke + Send + 'static),* {
	    fn join_parts(&self) -> String {
		    #[allow(non_snake_case)]
		    let ($($ty),*,) = self;
				[$($ty.serke()),*].into_iter().filter_map(identity).join(",")
	    }
    }
  };
}

join_parts_impl!(T0);
join_parts_impl!(T0, T1);
join_parts_impl!(T0, T1, T2);
join_parts_impl!(T0, T1, T2, T3);
join_parts_impl!(T0, T1, T2, T3, T4);
join_parts_impl!(T0, T1, T2, T3, T4, T5);
join_parts_impl!(T0, T1, T2, T3, T4, T5, T6);
join_parts_impl!(T0, T1, T2, T3, T4, T5, T6, T7);
join_parts_impl!(T0, T1, T2, T3, T4, T5, T6, T7, T8);

#[derive(Debug)]
pub struct Codec {
	next_index: usize,
	max_length: usize,
	is_discarding: bool,
}

impl Codec {
	pub fn new() -> Self {
		Self {
			next_index: 0,
			max_length: 1024,
			is_discarding: false,
		}
	}
}

fn utf8(buf: &[u8]) -> Result<&str, io::Error> {
	str::from_utf8(buf)
		.map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unable to decode input as UTF8"))
}

fn without_carriage_return(s: &[u8]) -> &[u8] {
	if let Some(&b'\r') = s.last() {
		&s[..s.len() - 1]
	} else {
		s
	}
}

// Ported from tokio-util's `LineCoded`
// https://github.com/tokio-rs/tokio/blob/master/tokio-util/src/codec/lines_codec.rs
impl Decoder for Codec {
	type Error = Error;
	type Item = Vec<String>;

	fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			let read_to = cmp::min(self.max_length.saturating_add(1), src.len());

			let newline_offset = src[self.next_index..read_to]
				.iter()
				.position(|&b| b == b'\n');

			match (self.is_discarding, newline_offset) {
				(true, Some(offset)) => {
					src.advance(offset + self.next_index + 1);
					self.is_discarding = false;
					self.next_index = 0;
				}
				(true, None) => {
					src.advance(read_to);
					self.next_index = 0;
					if src.is_empty() {
						return Ok(None);
					}
				}
				(false, Some(offset)) => {
					let newline_index = offset + self.next_index;
					self.next_index = 0;
					let line = src.split_to(newline_index + 1);
					let line = &line[..line.len() - 1];
					let line = without_carriage_return(line);
					let line = utf8(line)?;
					return Ok(Some(line.split(',').map(ToOwned::to_owned).collect()));
				}
				(false, None) if src.len() > self.max_length => {
					self.is_discarding = true;
					panic!("Message too long");
				}
				(false, None) => {
					self.next_index = read_to;
					return Ok(None);
				}
			}
		}
	}
}

impl Encoder<Box<dyn JoinParts + Send + 'static>> for Codec {
	type Error = Error;

	fn encode(
		&mut self,
		item: Box<dyn JoinParts + Send + 'static>,
		dst: &mut BytesMut,
	) -> Result<(), Self::Error> {
		let line = item.join_parts();
		dst.reserve(line.len() + 2);
		dst.put(line.as_bytes());
		dst.put_u16(0x0D0A); // 0x0D0A = CR+LF
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tokio_util::bytes::BytesMut;

	#[test]
	fn encode() {
		let mut bytes = BytesMut::new();
		let mut codec = Codec::new();

		codec.encode(("$KE", "INF").boxed(), &mut bytes).unwrap();
		assert_eq!(bytes.as_ref(), b"$KE,INF\r\n");
	}

	#[test]
	fn decode() {
		let mut bytes = BytesMut::from(b"#INF,Laurent-2\r\n".as_slice());
		let mut codec = Codec::new();

		if let Some(parts) = codec.decode(&mut bytes).unwrap() {
			assert_eq!(parts.as_slice(), &["#INF", "Laurent-2"]);
		} else {
			panic!("No parts found");
		}
	}
}
