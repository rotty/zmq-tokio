//! ØMQ (ZeroMQ) for tokio
//! ======================
//!
//! Run ØMQ sockets using `tokio` reactors, futures, etc.
extern crate futures;
extern crate mio;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio_core;
extern crate zmq;
extern crate zmq_mio;

use std::io;

use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_core::reactor::{Handle, PollEvented};
use mio::Ready;

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
#[derive(Clone)]
pub struct Context {
    ctx: zmq::Context,
}

impl Context {
    pub fn new() -> Context {
        Context {
            ctx: zmq::Context::new(),
        }
    }

    pub fn socket(&self, typ: zmq::SocketType, handle: &Handle) -> io::Result<Socket> {
        let socket = try!(self.ctx.socket(typ));
        Socket::new(socket, handle)
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
    pub fn new(socket: zmq::Socket, handle: &Handle) -> io::Result<Socket> {
        let io = try!(PollEvented::new(zmq_mio::Socket::new(socket), handle));
        Ok(Socket { io: io })
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
    pub fn send<T>(&self, item: T, flags: i32) -> io::Result<()>
    where
        T: Into<zmq::Message>,
    {
        trace!("entering send");
        if !try!(self.poll_events()).is_writable() {
            trace!("send - not ready");
            return Err(mio::would_block());
        }
        trace!("attempting send");
        let r = self.io.get_ref().send(item, flags);
        if is_wouldblock(&r) {
            self.io.need_read();
        }
        trace!("send - {:?}", r);
        r
    }

    /// Non-blocking recv a `zmq::Message`.
    pub fn recv(&self, flags: i32) -> io::Result<zmq::Message> {
        trace!("entering recv");
        if !try!(self.io.get_ref().poll_events()).is_readable() {
            trace!("recv - not ready");
            return Err(mio::would_block());
        }
        let r = self.io.get_ref().recv(flags);
        if is_wouldblock(&r) {
            self.io.need_read();
        }
        trace!("recv - {:?}", r);
        r
    }

    fn poll_events(&self) -> io::Result<Ready> {
        self.io.get_ref().poll_events()
    }

    pub fn framed(self) -> SocketFramed {
        SocketFramed::new(self)
    }
}

pub struct SocketFramed {
    socket: Socket,
    rd: Vec<Vec<u8>>,
    wr: Vec<u8>,
}

impl SocketFramed {
    fn new(socket: Socket) -> Self {
        SocketFramed {
            socket: socket,
            rd: Vec::new(),
            wr: Vec::new(),
        }
    }
}

// TODO: Make this generic using a codec
impl Sink for SocketFramed {
    type SinkItem = Vec<u8>;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Vec<u8>) -> StartSend<Vec<u8>, Self::SinkError> {
        trace!("start send ({:?})", item);
        if self.wr.len() > 0 {
            try!(self.poll_complete());
            if self.wr.len() > 0 {
                return Ok(AsyncSink::NotReady(item));
            }
        }
        self.wr = item;
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        trace!("polling sink for completion ({} bytes)", self.wr.len());
        if self.wr.is_empty() {
            return Ok(Async::Ready(()));
        }
        let r = try_nb!(self.socket.send(&self.wr[..], 0));
        trace!("send complete: {:?}", r);
        self.wr.clear();
        Ok(Async::Ready(()))
    }
}

// TODO: Make this generic using a codec
impl Stream for SocketFramed {
    type Item = Vec<Vec<u8>>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("SocketFramed::poll()");
        let r = self.socket.recv(0);
        if is_wouldblock(&r) {
            return Ok(Async::NotReady);
        }
        let msg = try!(r);
        if self.rd.len() == 0 && msg.len() == 0 {
            return Ok(Async::Ready(None));
        }
        self.rd.push(Vec::from(&msg[..]));
        while msg.get_more() {
            let r = self.socket.recv(0);
            let msg = try!(r);
            self.rd.push(Vec::from(&msg[..]));
        }
        return Ok(Async::Ready(Some(self.rd.split_off(0))));
    }
}
