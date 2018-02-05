//! ØMQ (ZeroMQ) for tokio
//! ======================
//!
//! Run ØMQ sockets using `tokio` reactors, futures, etc.
//!
//! Examples
//! ========
//!
//! Sending and receiving simple messages with futures
//! --------------------------------------------------
//!
//! A PAIR of sockets is created. The `sender` socket sends
//! a message, and the `receiver` gets it.
//!
//! Everything runs within on a tokio reactor.
//!
//! ```
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate zmq_tokio;
//!
//! use futures::Future;
//! use tokio_core::reactor::Core;
//!
//! use zmq_tokio::{Context, Socket, PAIR};
//!
//! const TEST_ADDR: &str = "inproc://test";
//!
//!
//! fn main() {
//!     let mut reactor = Core::new().unwrap();
//!     let context = Context::new();
//!
//!     let recvr = context.socket(PAIR, &reactor.handle()).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let sendr = context.socket(PAIR, &reactor.handle()).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!     // Step 1: send any type implementing `Into<zmq::Message>`,
//!     //         meaning `&[u8]`, `Vec<u8>`, `String`, `&str`,
//!     //         and `zmq::Message` itself.
//!     let send_future = sendr.send("this message will be sent");
//!
//!     // Step 2: receive the message on the pair socket
//!     let recv_msg = send_future.and_then(|_| {
//!         recvr.recv()
//!     });
//!
//!     // Step 3: process the message and exit
//!     let process_msg = recv_msg.and_then(|msg| {
//!         assert_eq!(msg.as_str(), Some("this message will be sent"));
//!         Ok(())
//!     });
//!
//!     let _ = reactor.run(process_msg).unwrap();
//!
//!     // Exit our program, playing nice.
//!     ::std::process::exit(0);
//! }
//! ```
//!
//! Sending and receiving multi-part messages with futures
//! ------------------------------------------------------
//!
//! This time we use `PUSH` and `PULL` sockets to move multi-part messages.
//!
//! Remember that ZMQ will either send all parts or none at all.
//! Save goes for receiving.
//!
//! ```
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate zmq_tokio;
//!
//! use futures::Future;
//! use tokio_core::reactor::Core;
//!
//! use zmq_tokio::{Context, Socket, PULL, PUSH};
//!
//! const TEST_ADDR: &str = "inproc://test";
//!
//! fn main() {
//!     let mut reactor = Core::new().unwrap();
//!     let context = Context::new();
//!
//!     let recvr = context.socket(PULL, &reactor.handle()).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let sendr = context.socket(PUSH, &reactor.handle()).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!     let msgs: Vec<Vec<u8>> = vec![b"hello".to_vec(), b"goodbye".to_vec()];
//!     // Step 1: send a vector of byte-vectors, `Vec<Vec<u8>>`
//!     let send_future = sendr.send_multipart(msgs);
//!
//!     // Step 2: receive the complete multi-part message
//!     let recv_msg = send_future.and_then(|_| {
//!         recvr.recv_multipart()
//!     });
//!
//!     // Step 3: process the message and exit
//!     let process_msg = recv_msg.and_then(|msgs| {
//!         assert_eq!(msgs[0].as_str(), Some("hello"));
//!         assert_eq!(msgs[1].as_str(), Some("goodbye"));
//!         Ok(())
//!     });
//!
//!     let _ = reactor.run(process_msg).unwrap();
//!
//!     // Exit our program, playing nice.
//!     ::std::process::exit(0);
//! }
//! ```
//!
//! Manual use of tokio tranports with `Sink` and `Stream`
//! ------------------------------------------------------
//!
//! This time, we use `PUB`-`SUB` sockets to send and receive a message.
//!
//! ```
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate zmq_tokio;
//!
//! use futures::{Future, Sink, Stream, stream};
//! use tokio_core::reactor::Core;
//!
//! use zmq_tokio::{Context, Message, Socket, PUB, SUB};
//!
//! const TEST_ADDR: &str = "inproc://test";
//!
//!
//! fn main() {
//!     let mut reactor = Core::new().unwrap();
//!     let context = Context::new();
//!
//!     let recvr = context.socket(SUB, &reactor.handle()).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!     let _ = recvr.set_subscribe(b"").unwrap();
//!
//!     let sendr = context.socket(PUB, &reactor.handle()).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!
//!     let (_, recvr_split_stream) = recvr.framed().split();
//!     let (sendr_split_sink, _) = sendr.framed().split();
//!
//!     let msg = Message::from_slice(b"hello there");
//!
//!     // Step 1: start a stream with only one item.
//!     let start_stream = stream::iter_ok::<_, ()>(vec![(sendr_split_sink, recvr_split_stream, msg)]);
//!
//!     // Step 2: send the message
//!     let send_msg = start_stream.and_then(|(sink, stream, msg)| {
//!             // send a message to the receiver.
//!             // return a future with the receiver
//!             let _ = sink.send(msg);
//!             Ok(stream)
//!         });
//!
//!     // Step 3: read the message
//!     let fetch_msg = send_msg.for_each(|stream| {
//!             // process the first response that the
//!             // receiver gets.
//!             // Assert that it equals the message sent
//!             // by the sender.
//!             // returns `Ok(())` when the stream ends.
//!             let _ = stream.into_future().and_then(|(response, _)| {
//!                 match response {
//!                     Some(msg) => assert_eq!(msg.as_str(), Some("hello there")),
//!                     None => panic!("expected a response"),
//!                 }
//!                 Ok(())
//!             });
//!             Ok(())
//!         });
//!
//!     // Run the stream
//!     let _ = reactor.run(fetch_msg).unwrap();
//!
//!     // Exit our program, playing nice.
//!     ::std::process::exit(0);
//! }
//! ```
extern crate futures;
extern crate futures_cpupool;
#[macro_use]
extern crate log;
extern crate mio;
extern crate tokio_core;
extern crate tokio_io;
pub extern crate zmq;
extern crate zmq_mio;

pub mod future;
mod poll_evented;
pub mod sink;
pub mod stream;
pub mod transport;

use std::io;
use std::io::{Read, Write};

use futures::Poll;

use tokio_core::reactor::{Handle, PollEvented};
use tokio_io::{AsyncRead, AsyncWrite};

use self::future::{ReceiveMessage, ReceiveMultipartMessage, SendMessage, SendMultipartMessage};
use self::stream::{MessageStream, MultipartMessageStream};
use self::sink::{MessageSink, MultipartMessageSink};

pub use io::Error;
pub use zmq::Message;
/// Supported socket types are: `DEALER`, `PAIR`, `PUB`, `PULL`, `PUSH`, `REP`, `REQ`, `ROUTER`, `STREAM`, `SUB`, `XPUB`, `XSUB`.
pub use zmq::SocketType::*;

// Re-export custom transport to keep backwards-compatibility with examples
// TODO: move this someplace else once the API is stable
pub use self::transport::SocketFramed;

/// Wrapper for `zmq::Context`.
#[derive(Clone, Default)]
pub struct Context {
    inner: zmq_mio::Context,
}

impl Context {
    /// Create a new ØMQ context for the `tokio` framework.
    pub fn new() -> Context {
        Context {
            inner: zmq_mio::Context::new(),
        }
    }

    /// Create a new ØMQ socket for the `tokio` framework.
    pub fn socket(&self, typ: zmq::SocketType, handle: &Handle) -> io::Result<Socket> {
        Ok(Socket::new(try!(self.inner.socket(typ)), handle)?)
    }

    /// Try to destroy the underlying context. This is different than the destructor;
    /// the destructor will loop when zmq_ctx_destroy returns EINTR.
    pub fn destroy(&mut self) -> io::Result<()> {
        self.inner.destroy()
    }

    /// Get a cloned instance of the underlying `zmq_mio::Context`.
    pub fn get_inner(&self) -> zmq_mio::Context {
        self.inner.clone()
    }
}

/// Poll-evented ØMQ socket. Can be used directly on transports implementing
/// `futures::stream::Stream` and `futures::sink::Sink`.
pub struct Socket {
    io: PollEvented<zmq_mio::Socket>,
}

impl Socket {
    /// Create a new poll-evented ØMQ socket, along with a tokio reactor handle
    /// to drive its event-loop.
    fn new(socket: zmq_mio::Socket, handle: &Handle) -> io::Result<Self> {
        let io = try!(PollEvented::new(socket, handle));
        let socket = Socket { io };
        Ok(socket)
    }

    /// A reference to the underlying `zmq_mio::Socket`. Useful
    /// for building futures.
    pub fn get_ref(&self) -> &PollEvented<zmq_mio::Socket> {
        &self.io
    }

    /// A reference to the underlying `zmq_mio::Socket`. Useful
    /// for building futures.
    fn get_mio_ref(&self) -> &zmq_mio::Socket {
        self.io.get_ref()
    }

    /// Bind the underlying socket to the given address.
    pub fn bind(&self, address: &str) -> io::Result<()> {
        self.get_mio_ref().bind(address)
    }

    /// Connect the underlying socket to the given address.
    pub fn connect(&self, address: &str) -> io::Result<()> {
        self.get_mio_ref().connect(address)
    }

    /// Subscribe the underlying socket to the given prefix.
    pub fn set_subscribe(&self, prefix: &[u8]) -> io::Result<()> {
        self.get_mio_ref().set_subscribe(prefix)
    }

    /// Sends a type implementing `Into<zmq::Message>` as a `Future`.
    pub fn send<T: Into<zmq::Message>>(&self, message: T) -> SendMessage {
        SendMessage::new(self, message.into())
    }

    /// Sends a type implementing `Into<zmq::Message>` as a `Future`.
    pub fn send_multipart<I, T>(&self, messages: I) -> SendMultipartMessage
    where
        I: IntoIterator<Item = T>,
        T: Into<Vec<u8>>,
    {
        SendMultipartMessage::new(self, messages)
    }

    /// Returns a `Future` that resolves into a `zmq::Message`
    pub fn recv(&self) -> ReceiveMessage {
        ReceiveMessage::new(self)
    }

    /// Returns a `Future` that resolves into a `Vec<zmq::Message>`
    pub fn recv_multipart(&self) -> ReceiveMultipartMessage {
        ReceiveMultipartMessage::new(self)
    }

    /// Get the SocketType
    pub fn get_socket_type(&self) -> io::Result<zmq::SocketType> {
        self.get_mio_ref().get_socket_type()
    }

    pub fn framed(self) -> SocketFramed<Self> {
        SocketFramed::new(self)
    }

    /// Returns a `Stream` of incoming one-part messages.
    pub fn incoming<'a>(&'a self) -> MessageStream<'a, PollEvented<zmq_mio::Socket>> {
        MessageStream::new(self.get_ref())
    }

    /// Returns a `Stream` of incoming multipart-messages.
    pub fn incoming_multipart<'a>(&'a self) -> MultipartMessageStream<'a, PollEvented<zmq_mio::Socket>> {
        MultipartMessageStream::new(self.get_ref())
    }

    /// Returns a `Sink` for outgoing one-part messages.
    pub fn outgoing<'a>(&'a self) -> MessageSink<'a, PollEvented<zmq_mio::Socket>> {
        MessageSink::new(self.get_ref())
    }

    /// Returns a `Sink` for outgoing multipart-messages.
    pub fn outgoing_multipart<'a>(&'a self) -> MultipartMessageSink<'a, PollEvented<zmq_mio::Socket>> {
        MultipartMessageSink::new(self.get_ref())
    }
}

unsafe impl Send for Socket {}

impl Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read(buf)
    }
}

impl Write for Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for Socket {}

impl AsyncWrite for Socket {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        Ok(().into())
    }
}

/// Convert an `zmq::Socket` instance into `zmq_tokio::Socket`.
pub fn convert_into_tokio_socket(orig: zmq::Socket, handle: &Handle) -> io::Result<Socket> {
    let mio_socket = zmq_mio::Socket::new(orig);
    Socket::new(mio_socket, handle)
}

/// API methods for sending messages with sockets.
pub trait SocketSend {
    /// Send a message.
    ///
    /// Due to the provided From implementations, this works for `&[u8]`, `Vec<u8>` and `&str`,
    /// as well as on `Message` itself.
    fn send<T>(&self, T, i32) -> io::Result<()>
    where
        T: zmq::Sendable;
    /// Sends a multipart-message.
    fn send_multipart<I, T>(&self, I, i32) -> io::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<Message>;
}

/// API methods for receiving messages with sockets.
pub trait SocketRecv {
    /// Return true if there are more frames of a multipart message to receive.
    fn get_rcvmore(&self) -> io::Result<bool>;

    /// Receive a message into a `Message`. The length passed to `zmq_msg_recv` is the length
    /// of the buffer.
    fn recv(&self, &mut Message, i32) -> io::Result<()>;

    /// Receive bytes into a slice. The length passed to `zmq_recv` is the length of the slice. The
    /// return value is the number of bytes in the message, which may be larger than the length of
    /// the slice, indicating truncation.
    fn recv_into(&self, &mut [u8], i32) -> io::Result<usize>;

    /// Receive a message into a fresh `Message`.
    fn recv_msg(&self, i32) -> io::Result<Message>;

    /// Receive a message as a byte vector.
    fn recv_bytes(&self, i32) -> io::Result<Vec<u8>>;

    /// Receive a `String` from the socket.
    ///
    /// If the received message is not valid UTF-8, it is returned as the original `Vec` in the `Err`
    /// part of the inner result.
    fn recv_string(&self, i32) -> io::Result<Result<String, Vec<u8>>>;

    /// Receive a multipart message from the socket.
    ///
    /// Note that this will allocate a new vector for each message part; for many applications it
    /// will be possible to process the different parts sequentially and reuse allocations that
    /// way.
    fn recv_multipart(&self, i32) -> io::Result<Vec<Vec<u8>>>;
}
