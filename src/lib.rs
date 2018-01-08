//! ØMQ (ZeroMQ) for tokio
//! ======================
//!
//! Run ØMQ sockets using `tokio` reactors, futures, etc.
//!
//! # Examples
//!
//! ## Sending and receiving simple messages with `Future`
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
//! fn test_pair() -> (Socket, Socket, Core) {
//!     let reactor = Core::new().unwrap();
//!     let handle = reactor.handle();
//!     let ctx = Context::new();
//!
//!     let recvr = ctx.socket(PAIR, &handle).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let sendr = ctx.socket(PAIR, &handle).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!     (recvr, sendr, reactor)
//! }
//!
//! fn main() {
//!     let (mut recvr, mut sendr, mut reactor) = test_pair();
//!     let msg = zmq::Message::from_slice(b"this message will be sent");
//!
//!     // Step 1: send the message
//!     let send_future = sendr.send(msg);
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
//! fn test_pair() -> (Socket, Socket, Core) {
//!     let reactor = Core::new().unwrap();
//!     let handle = reactor.handle();
//!     let ctx = Context::new();
//!
//!     let recvr = ctx.socket(PAIR, &handle).unwrap();
//!     let _ = recvr.bind(TEST_ADDR).unwrap();
//!
//!     let sendr = ctx.socket(PAIR, &handle).unwrap();
//!     let _ = sendr.connect(TEST_ADDR).unwrap();
//!
//!     (recvr, sendr, reactor)
//! }
//!
//! fn main() {
//!     let (recvr, sendr, mut reactor) = test_pair();
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
extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
#[macro_use]
extern crate log;
extern crate mio;
extern crate tokio_core;
extern crate tokio_io;
extern crate zmq;
extern crate zmq_mio;

#[path = "futures.rs"]
pub mod zmq_futures;

use std::io;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};

use bytes::BytesMut;
use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_core::reactor::{Handle, PollEvented};
use tokio_io::{AsyncRead, AsyncWrite};
use mio::Ready;

use self::zmq_futures::{ReceiveMessage, SendMessage};
/// The possible socket types.
pub use zmq::SocketType::{DEALER, PAIR, PUB, PULL, PUSH, REP, REQ, ROUTER, STREAM, SUB, XPUB, XSUB};

// Convenience function to determine if an I/O operation would block
// if the error kind is `io::ErrorKind::WouldBlock`. Returns a `boolean`.
fn is_wouldblock<T>(r: &io::Result<T>) -> bool {
    match *r {
        Ok(_) => false,
        Err(ref e) => e.kind() == io::ErrorKind::WouldBlock,
    }
}

/// Wrapper for `zmq::Context`.
// TODO: maybe we don't need this
#[derive(Clone, Default)]
pub struct Context {
    inner: zmq::Context,
}

impl Context {
    /// Create a new ØMQ context for the `tokio` framework.
    pub fn new() -> Context {
        Context {
            inner: zmq::Context::new(),
        }
    }

    /// Create a new ØMQ socket for the `tokio` framework.
    pub fn socket(&self, typ: zmq::SocketType, handle: &Handle) -> io::Result<Socket> {
        let socket = try!(self.inner.socket(typ));
        let new_sock: Socket = Socket::new(socket, handle)?;
        Ok(new_sock)
    }

    /// Try to destroy the underlying context. This is different than the destructor;
    /// the destructor will loop when zmq_ctx_destroy returns EINTR.
    pub fn destroy(&mut self) -> io::Result<()> {
        self.inner.destroy().map_err(|e| e.into())
    }

    /// Get a cloned instance of the underlying `zmq::Context`.
    pub fn get_inner(&self) -> zmq::Context {
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
    pub fn new(socket: zmq::Socket, handle: &Handle) -> io::Result<Self> {
        let io = try!(PollEvented::new(zmq_mio::Socket::new(socket), handle));
        let socket = Socket { io };
        Ok(socket)
    }

    /// Bind the underlying socket to the given address.
    pub fn bind(&self, address: &str) -> io::Result<()> {
        self.io.get_ref().bind(address)
    }

    /// Connect the underlying socket to the given address.
    pub fn connect(&self, address: &str) -> io::Result<()> {
        self.io.get_ref().connect(address)
    }

    /// Subscribe the underlying socket to the given prefix.
    pub fn set_subscribe(&self, prefix: &[u8]) -> io::Result<()> {
        self.io.get_ref().set_subscribe(prefix)
    }

    /// Non-blocking send a `zmq::Message`.
    pub fn _send(&mut self, item: &[u8], _flags: i32) -> io::Result<usize> {
        trace!("entering send");
        if !try!(self.poll_events()).is_writable() {
            trace!("send - not ready");
            return Err(mio::would_block());
        }
        trace!("attempting send");
        let r = self.io.write(item);
        if is_wouldblock(&r) {
            self.io.need_write();
            return Err(mio::would_block());
        }
        trace!("send - {:?}", r);
        r
    }

    /// Sends a type implementing `Into<zmq::Message>` as a `Future`.
    pub fn send<T: Into<zmq::Message>>(&mut self, message: T) -> SendMessage {
        SendMessage::new(self, message.into())
    }

    /// Returns a `Future` that resolves into a `zmq::Message`
    pub fn recv(&mut self) -> ReceiveMessage {
        ReceiveMessage::new(self)
    }

    /// Non-blocking recv a `zmq::Message`.
    pub fn _recv(&mut self, _flags: i32) -> io::Result<zmq::Message> {
        trace!("entering recv");
        if !try!(self.poll_events()).is_readable() {
            trace!("recv - not ready");
            return Err(mio::would_block());
        }
        let mut buf = BytesMut::new();
        let r = self.io.read(&mut buf);
        if is_wouldblock(&r) {
            self.io.need_read();
            return Err(mio::would_block());
        }
        trace!("recv - {:?}", buf);
        let msg = zmq::Message::from_slice(&buf);
        Ok(msg)
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
