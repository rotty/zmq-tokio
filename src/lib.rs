//! ØMQ (ZeroMQ) for tokio
//! ======================
//!
//! Run ØMQ sockets using `tokio` reactors, futures, etc.
extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
extern crate mio;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio_core;
extern crate tokio_io;
extern crate zmq;
extern crate zmq_mio;

use std::io;
use std::io::{Read, Write};
use std::ops::Deref;

use bytes::BytesMut;
use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_core::reactor::{Handle, PollEvented};
use tokio_io::{AsyncRead, AsyncWrite};
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
        let new_sock: Socket = Socket::new(socket, handle)?;
        Ok(new_sock)
    }

    pub fn inner(&self) -> zmq::Context {
        self.ctx.clone()
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
    pub fn send(&mut self, item: &[u8], _flags: i32) -> io::Result<usize> {
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

    /// Non-blocking recv a `zmq::Message`.
    pub fn recv(&mut self, _flags: i32) -> io::Result<zmq::Message> {
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

impl io::Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read(buf)
    }
}

impl io::Write for Socket {
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
        let mut buf = vec![0; 1024];
        trace!("SocketFramed::poll()");
        match self.socket.read(&mut buf) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(c) => {
                let msg = zmq::Message::from_slice(&buf[..c]);
                Ok(Async::Ready(Some(msg)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use zmq;

    use futures::stream;
    use futures::{Future, Sink, Stream};
    use futures_cpupool::CpuPool;
    use tokio_core::reactor::Core;
    use super::*;

    const TEST_ADDR: &str = "inproc://test";

    fn test_pair() -> (Socket, Socket, Core) {
        let core = Core::new().unwrap();
        let handle = core.handle();
        let ctx = Context::new();

        let recvr = ctx.socket(zmq::PAIR, &handle).unwrap();
        let _ = recvr.bind(TEST_ADDR).unwrap();

        let sendr = ctx.socket(zmq::PAIR, &handle).unwrap();
        let _ = sendr.connect(TEST_ADDR).unwrap();

        (recvr, sendr, core)
    }

    #[test]
    fn socket_receives_byte_buffer() {
        let pool = CpuPool::new_num_cpus();
        let (recvr, sendr, mut _core) = test_pair();

        let s = pool.spawn_fn(move || {
            let (_, recvr_rx) = recvr.framed().split();
            let (sendr_tx, _) = sendr.framed().split();

            let msg = zmq::Message::from_slice(b"hello there");

            let test_stream = stream::iter_ok::<_, ()>(vec![(sendr_tx, recvr_rx, msg)])
                .and_then(|(tx, rx, msg)| {
                    // send a message to the receiver.
                    // return a future with the receiver
                    let _ = tx.send(msg);
                    Ok(rx)
                })
                .for_each(|rx| {
                    // process the first response that the
                    // receiver gets.
                    // Assert that it equals the message sent
                    // by the sender.
                    // returns `Ok(())` when the stream ends.
                    let _ = rx.into_future().and_then(|(response, _)| {
                        let msg = match response {
                            Some(m) => m,
                            None => panic!("expected a response"),
                        };
                        assert_eq!(msg.as_str(), Some("hello there"));
                        Ok(())
                    });
                    Ok(())
                });
            let _ = test_stream.wait().unwrap();
            let ok: std::result::Result<(), ()> = Ok(());
            ok
        });
        let _ = s.wait().unwrap();
    }
}
