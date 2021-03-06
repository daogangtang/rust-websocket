//! Structs for dealing with WebSocket clients
#![unstable]

use dataframe::{DataFrameSender, DataFrameReceiver, DataFrameConverter};
use dataframe::WebSocketDataFrame;
use message::WebSocketMessaging;
use common::WebSocketResult;
use std::path::BytesContainer;
use std::sync::{Arc, Mutex};

pub use self::incoming::{IncomingDataFrames, IncomingMessages};
pub use self::fragment::{TextFragmentSender, BinaryFragmentSender};

use dataframe::{WebSocketSender, WebSocketReceiver, WebSocketConverter};
use common::{WebSocketStream, Local, Remote};
use WebSocketMessage;

pub mod incoming;
pub mod fragment;

/// The most common local WebSocketClient type, provided for convenience.
pub type WebSocketLocalClient = WebSocketClient<WebSocketSender<WebSocketStream, Local>, WebSocketReceiver<WebSocketStream, Local>, WebSocketConverter<WebSocketMessage>, WebSocketStream, WebSocketStream, WebSocketMessage>;
/// The most common remote WebSocketClient type, provided for convenience.
pub type WebSocketRemoteClient = WebSocketClient<WebSocketSender<WebSocketStream, Remote>, WebSocketReceiver<WebSocketStream, Remote>, WebSocketConverter<WebSocketMessage>, WebSocketStream, WebSocketStream, WebSocketMessage>;

/// Represents a WebSocketClient which connects to a WebSocketServer. See the main library documentation for how to obtain a ```WebSocketClient```.
pub struct WebSocketClient<S: DataFrameSender<W>, R: DataFrameReceiver<E>, C: DataFrameConverter<M>, E: Reader + Send, W: Writer + Send, M: WebSocketMessaging> {
	sender: Arc<Mutex<S>>,
	receiver: Arc<Mutex<(R, C)>>,
}

unsafe impl<S: DataFrameSender<W>, R: DataFrameReceiver<E>, C: DataFrameConverter<M>, E: Reader + Send, W: Writer + Send, M: WebSocketMessaging> Send for WebSocketClient<S, R, C, E, W, M> {}

impl<S: DataFrameSender<W>, R: DataFrameReceiver<E>, C: DataFrameConverter<M>, E: Reader + Send, W: Writer + Send, M: WebSocketMessaging> WebSocketClient<S, R, C, E, W, M> {
	/// Create a WebSocketClient from the specified DataFrameSender and DataFrameReceiver.
	/// Not required for normal usage (used internally by ```WebSocketResponse```).
	pub fn new(sender: S, receiver: R, converter: C) -> WebSocketClient<S, R, C, E, W, M> {
		WebSocketClient {
			sender: Arc::new(Mutex::new(sender)),
			receiver: Arc::new(Mutex::new((receiver, converter))),
		}
	}
	
	/// Sends a WebSocketDataFrame. Blocks the task until the message has been sent.
	#[stable]
	pub fn send_dataframe(&mut self, dataframe: &WebSocketDataFrame) -> WebSocketResult<()> {
		let mut sender = self.sender.lock();
		sender.send_dataframe(dataframe)
	}
	
	/// Receives a single WebSocketDataFrame - may corrupt messages received from recv_message(),
	/// so do not use both at the same time (ie. either use only recv_dataframe() or use only recv_message())
	#[stable]
	pub fn recv_dataframe(&mut self) -> WebSocketResult<WebSocketDataFrame> {
		let mut receiver = self.receiver.lock();
		receiver.0.recv_dataframe()
	}

	/// Gets an iterator over incoming data frames
	#[stable]
	pub fn incoming_dataframes(&mut self) -> IncomingDataFrames<S, R, C, E, W, M> {
		IncomingDataFrames::new(self)
	}
	
	/// Gets an iterator over incoming messages.
	/// 
	/// The iterator always returns Some(), and each iteration will block until a message is received.
	/// 
	/// ```no_run
	///# extern crate url;
	///# extern crate websocket;
	///# fn main() {
	///# use websocket::{WebSocketRequest, WebSocketMessage};
	///# use url::Url;
	///# let url = Url::parse("ws://127.0.0.1:1234").unwrap();
	///# let request = WebSocketRequest::connect(url).unwrap();
	///# let response = request.send().unwrap();
	///# let mut client = response.begin();
	///for message in client.incoming_messages() {
	///    match message.unwrap() {
	///        WebSocketMessage::Text(text) => { println!("Text: {}", text); },
	///        WebSocketMessage::Binary(data) => { println!("Binary data received"); },
	///        _ => { }
	///    }
	///}
	///# }
	/// ```
	pub fn incoming_messages(&mut self) -> IncomingMessages<S, R, C, E, W, M> {
		IncomingMessages::new(self)
	}
	
	/// Sends a fragmented text message by returning a TextFragmentSender
	///
	/// ```no_run
	///# extern crate url;
	///# extern crate websocket;
	///# fn main() {
	///# use websocket::WebSocketRequest;
	///# use url::Url;
	///# let url = Url::parse("ws://127.0.0.1:1234").unwrap();
	///# let request = WebSocketRequest::connect(url).unwrap();
	///# let response = request.send().unwrap();
	///# let mut client = response.begin();
	///let mut fragment_sender = client.frag_send_text("This").unwrap();
	///let _ = fragment_sender.send("is ");
	///let _ = fragment_sender.send("a ");
	///let _ = fragment_sender.send("fragmented ");
	///let _ = fragment_sender.finish("message.");
	///# }
	/// ```
	pub fn frag_send_text<'a, T: ToString>(&'a mut self, text: T) -> WebSocketResult<TextFragmentSender<'a, S, W>> {
		TextFragmentSender::new(self.sender.lock(), text)
	}
	
	/// Sends a fragmented binary message by returning a BinaryFragmentSender
	///
	/// ```no_run
	///# extern crate url;
	///# extern crate websocket;
	///# fn main() {
	///# use websocket::WebSocketRequest;
	///# use url::Url;
	///# let url = Url::parse("ws://127.0.0.1:1234").unwrap();
	///# let request = WebSocketRequest::connect(url).unwrap();
	///# let response = request.send().unwrap();
	///# let mut client = response.begin();
	///let mut fragment_sender = client.frag_send_bytes("ascii_bytes").unwrap();
	///let _ = fragment_sender.send([100u8, ..4].as_slice());
	///let _ = fragment_sender.finish(vec![4u8, 2, 68, 24]);
	///# }
	/// ```
	pub fn frag_send_bytes<'a, T: BytesContainer>(&'a mut self, data: T) -> WebSocketResult<BinaryFragmentSender<'a, S, W>> {
		BinaryFragmentSender::new(self.sender.lock(), data)
	}
	
	/// Sends a WebSocketMessage. Blocks the task until the message has been sent.
	/// 
	/// ```no_run
	///# extern crate url;
	///# extern crate websocket;
	///# fn main() {
	///# use websocket::{WebSocketRequest, WebSocketMessage};
	///# use url::Url;
	///# let url = Url::parse("ws://127.0.0.1:1234").unwrap();
	///# let request = WebSocketRequest::connect(url).unwrap();
	///# let response = request.send().unwrap();
	///# let mut client = response.begin();
	///let message = WebSocketMessage::Text("Hello, server!".to_string());
	///let _ = client.send_message(message);
	///# }
	/// ```
	pub fn send_message(&mut self, message: M) -> WebSocketResult<()> {
		let dataframe = try!(message.into_dataframe());
		self.send_dataframe(&dataframe)
	}
	
	/// Receives a WebSocketMessage. Blocks the task until a full message is received.
	/// 
	/// ```no_run
	///# extern crate url;
	///# extern crate websocket;
	///# fn main() {
	///# use websocket::{WebSocketRequest, WebSocketMessage};
	///# use url::Url;
	///# let url = Url::parse("ws://127.0.0.1:1234").unwrap();
	///# let request = WebSocketRequest::connect(url).unwrap();
	///# let response = request.send().unwrap();
	///# let mut client = response.begin();
	///let message = client.recv_message().unwrap();
	///match message {
	///    WebSocketMessage::Text(text) => { println!("Text: {}", text); },
	///    WebSocketMessage::Binary(data) => { println!("Binary data received"); },
	///    _ => { }
	///}
	///# }
	/// ```
	pub fn recv_message(&mut self) -> WebSocketResult<M> {
		let mut receiver = self.receiver.lock();
		loop {
			let dataframe = try!(receiver.0.recv_dataframe());
			try!(receiver.1.push(dataframe));
			match receiver.1.pop() {
				Some(message) => { return Ok(message); }
				None => { }
			}
		}
	}
}

impl<S: DataFrameSender<W>, R: DataFrameReceiver<E>, C: DataFrameConverter<M>, E: Reader + Send, W: Writer + Send, M: WebSocketMessaging> Clone for WebSocketClient<S, R, C, E, W, M> {
	/// Clone this WebSocketClient, allowing for concurrent operations on a single stream.
	/// 
	/// All cloned clients refer to the same underlying stream. Simultaneous reads will not
	/// return the same data; the first read will obtain one WebSocketMessage/WebSocketDataFrame,	
	/// the second will obtain the next WebSocketMessage/WebSocketDataFrame, etc.
	#[stable]
	fn clone(&self) -> WebSocketClient<S, R, C, E, W, M> {
		WebSocketClient {
			sender: self.sender.clone(),
			receiver: self.receiver.clone(),
		}
	}
}