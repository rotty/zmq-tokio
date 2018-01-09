//! ØMQ (ZeroMQ) for tokio
//! ======================
//!
//! Run ØMQ sockets using `tokio` reactors, futures, etc.
//!
//! # Examples
//!
//! ## Sending and receiving simple messages with `Future`
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
//! extern crate zmq;
//!
//! use futures::stream;
//! use futures::{Future, Sink, Stream};
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
//!     let mut recvr = context.socket(PAIR, &reactor.handle()).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let mut sendr = context.socket(PAIR, &reactor.handle()).unwrap();
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
//! }
//! ```
//!
//! ## Sending and receiving multi-part messages with `Future`
//! ```
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate zmq_tokio;
//! extern crate zmq;
//!
//! use futures::stream;
//! use futures::{Future, Sink, Stream};
//! use tokio_core::reactor::Core;
//!
//! use zmq_tokio::{Context, Socket, PAIR};
//!
//! const TEST_ADDR: &str = "inproc://test";
//!
//! fn main() {
//!     let mut reactor = Core::new().unwrap();
//!     let context = Context::new();
//!
//!     let mut recvr = context.socket(PAIR, &reactor.handle()).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let mut sendr = context.socket(PAIR, &reactor.handle()).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!     let msgs: Vec<Vec<u8>> = vec![b"hello".to_vec(), b"goodbye".to_vec()];
//!     // Step 1: send a vector of byte-vectors, `Vec<Vec<u8>>`
//!     let send_future = sendr.send_multipart(msgs);
//!
//!     // Step 2: receive the message on the pair socket
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
//! }
//! ```
//!
//! ## Manual handling using `Transport`
//! ```
//! extern crate futures;
//! extern crate tokio_core;
//! extern crate zmq_tokio;
//! extern crate zmq;
//!
//! use futures::stream;
//! use futures::{Future, Sink, Stream};
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
//!
//!     let (_, rx) = recvr.framed().split();
//!     let (tx, _) = sendr.framed().split();
//!
//!     let msg = zmq::Message::from_slice(b"hello there");
//!
//!     // Step 1: start a stream with only one item.
//!     let start_stream = stream::iter_ok::<_, ()>(vec![(tx, rx, msg)]);
//!
//!     // Step 2: send the message
//!     let send_msg = start_stream.and_then(|(tx, rx, msg)| {
//!             // send a message to the receiver.
//!             // return a future with the receiver
//!             let _ = tx.send(msg);
//!             Ok(rx)
//!         });
//!
//!     // Step 3: read the message
//!     let fetch_msg = send_msg.for_each(|rx| {
//!             // process the first response that the
//!             // receiver gets.
//!             // Assert that it equals the message sent
//!             // by the sender.
//!             // returns `Ok(())` when the stream ends.
//!             let _ = rx.into_future().and_then(|(response, _)| {
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
extern crate zmq;
extern crate zmq_mio;

pub mod future;

use std::io;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};

use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_core::reactor::{Handle, PollEvented};
use tokio_io::{AsyncRead, AsyncWrite};

use self::future::{ReceiveMessage, SendMessage};
/// The possible socket types.
pub use zmq::SocketType::{DEALER, PAIR, PUB, PULL, PUSH, REP, REQ, ROUTER, STREAM, SUB, XPUB, XSUB};


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
    pub fn socket(&self, typ: SocketType, handle: &Handle) -> io::Result<Socket> {
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
    pub fn get_mio_ref(&self) -> &zmq_mio::Socket {
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
    pub fn send<T: Into<zmq::Message>>(&mut self, message: T) -> SendMessage {
        SendMessage::new(self, message.into())
    }

    /// Returns a `Future` that resolves into a `zmq::Message`
    pub fn recv(&mut self) -> ReceiveMessage {
        ReceiveMessage::new(self)
    }

    /// Call to `poll_events` on the inner socket.
    fn poll_events(&self) -> io::Result<Ready> {
        self.io.get_ref().poll_events()
    }

    pub fn framed(self) -> SocketFramed<Self> {
        SocketFramed::new(self)
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

/// A custom transport type for `Socket`.
pub struct SocketFramed<T> {
    socket: T,
}

impl<T> SocketFramed<T>
where
    T: AsyncRead + AsyncWrite,
{
    fn new(socket: T) -> Self {
        SocketFramed { socket: socket }
    }
}

// TODO: Make this generic using a codec
impl<T> Sink for SocketFramed<T>
where
    T: AsyncRead + AsyncWrite,
{
    type SinkItem = zmq::Message;
    type SinkError = io::Error;

    fn start_send(&mut self, item: zmq::Message) -> StartSend<zmq::Message, Self::SinkError> {
        trace!("SocketFramed::start_send()");
        match self.socket.write(item.deref()) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    return Ok(AsyncSink::NotReady(item));
                } else {
                    return Err(e);
                }
            }
            Ok(_) => {
                return Ok(AsyncSink::Ready);
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}

// TODO: Make this generic using a codec
impl<T> Stream for SocketFramed<T>
where
    T: AsyncRead,
{
    type Item = zmq::Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut buf = zmq::Message::with_capacity(1024);
        trace!("SocketFramed::poll()");
        match self.socket.read(buf.deref_mut()) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(c) => {
                buf = zmq::Message::from_slice(&buf[..c]);
                Ok(Async::Ready(Some(buf)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // No unit tests yet. Checkout the integration and doctests.
}
