//! Asynchronous `ØMQ`, a.k.a.`(ZeroMQ)` in `Rust` with `mio`.
//!
//! Run ØMQ sockets that implement `mio::Evented`, as well as non-blocking
//! implementations of `io::Write` and `io::Read`.
//!
//! # Example
//! ```
//! extern crate mio;
//! extern crate zmq;
//! extern crate zmq_mio;
//!
//! use std::io;
//! use mio::{Events, Poll, PollOpt, Ready, Token};
//! use zmq_mio::{Context, Socket};
//!
//! // We use ØMQ's `inproc://` scheme for intelligent and ready-to-use
//! // inter-process communications (IPC).
//! const EXAMPLE_ADDR: &str = "inproc://example_addr";
//! const LISTENER: Token = Token(0);
//! const SENDER: Token = Token(1);
//!
//! // An example of a typical ZMQ-flow, using asynchronous mode.
//! fn main() {
//!     // Create the context.
//!     let context = Context::new();
//!     // Use the context to generate sockets.
//!     let listener = context.socket(zmq::PAIR).unwrap();
//!     let sender = context.socket(zmq::PAIR).unwrap();
//!
//!     // Bind and connect our sockets.
//!     let _ = listener.bind(EXAMPLE_ADDR).unwrap();
//!     let _ = sender.connect(EXAMPLE_ADDR).unwrap();
//!
//!     // Now, for the asynchronous stuff...
//!     // First, we setup a `mio::Poll` instance.
//!     let poll = Poll::new().unwrap();
//!
//!     // Then we register our sockets for polling the events that
//!     // interest us.
//!     poll.register(&listener, LISTENER, Ready::readable(),
//!                 PollOpt::edge()).unwrap();
//!     poll.register(&sender, SENDER, Ready::writable(),
//!                 PollOpt::edge()).unwrap();
//!
//!     // We setup a loop which will poll our sockets at every turn,
//!     // handling the events just the way we want them to be handled.
//!     let mut events = Events::with_capacity(1024);
//!
//!     // We also setup some variables to control the main loop flow.
//!     let mut msg_sent = false;
//!     let mut msg_received = false;
//!
//!     // will loop until the listener gets a message.
//!     while !msg_received {
//!         // Poll for our registered events.
//!         poll.poll(&mut events, None).unwrap();
//!
//!         // Handle each event accordingly...
//!         for event in &events {
//!             match event.token() {
//!                 SENDER => {
//!                     // if the sender is writable and the message hasn't
//!                     // been sent, then we try to send it. If sending
//!                     // is not possible because the socket would block,
//!                     // then we just continue with handling polled events.
//!                     if event.readiness().is_writable() && !msg_sent {
//!                         if let Err(e) = sender.send("hello", 0) {
//!                            if e.kind() == io::ErrorKind::WouldBlock {
//!                                continue;
//!                            }
//!                            panic!("trouble sending message");
//!                         }
//!                         msg_sent = true;
//!                     }
//!                 }
//!                 LISTENER => {
//!                     // if the listener is readable, then we try to receive
//!                     // it. If reading is not possible because of blocking, then
//!                     // we continue handling events.
//!                     let msg = match listener.recv_msg(0) {
//!                         Ok(m) => m,
//!                         Err(e) => {
//!                             if e.kind() == io::ErrorKind::WouldBlock {
//!                                 continue;
//!                             }
//!                             panic!("trouble receiving message");
//!                         }
//!                     };
//!                     msg_received = true;
//!                 }
//!                 _ => unreachable!(),
//!             }
//!         }
//!     }
//! }
//! ```
#[macro_use]
extern crate log;
extern crate mio;
extern crate zmq;

use std::io;
use std::io::{Read, Write};
use std::fmt;
use std::os::unix::io::RawFd;

use mio::unix::EventedFd;
use mio::{PollOpt, Ready, Token};

/// Wrapper for ØMQ context.
#[derive(Clone, Default)]
pub struct Context {
    // Wrapper for `zmq::Context`
    inner: zmq::Context,
}

impl fmt::Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<zmq_mio::Context>")
    }
}

impl Context {
    /// Create a new `Context` instance. Use the `Context::socket` method
    /// to create sockets that can talk via `inproc://*` addresses.
    pub fn new() -> Self {
        Context {
            inner: zmq::Context::new(),
        }
    }

    /// Create a new `Socket` instance for asynchronous communications.
    pub fn socket(&self, typ: zmq::SocketType) -> io::Result<Socket> {
        Ok(Socket::new(try!(self.inner.socket(typ))))
    }

    /// Try to destroy the underlying context. This is different than the destructor;
    /// the destructor will loop when zmq_ctx_destroy returns EINTR.
    pub fn destroy(&mut self) -> io::Result<()> {
        self.inner.destroy().map_err(|e| e.into())
    }
}

// mio integration, should probably be put into its own crate eventually
/// Asynchronous ØMQ socket.
pub struct Socket {
    inner: zmq::Socket,
}

impl fmt::Debug for Socket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Socket<{:?}>", self.inner.get_socket_type())
    }
}

impl Socket {
    /// Create a new event-wrapped ØMQ socket. Takes an existing `zmq::Socket`
    /// instance as an only argument.
    pub fn new(socket: zmq::Socket) -> Self {
        Socket { inner: socket }
    }

    /// Returns an `io::Result` with the raw socket file-descriptor.
    pub fn as_raw_fd(&self) -> io::Result<RawFd> {
        let fd = try!(self.inner.get_fd());
        trace!("socket raw FD: {}", fd);
        Ok(fd)
    }

    /// Returns a reference to the underlying `zmq::Socket`.
    /// Useful for setting socket options at runtime.
    pub fn get_ref(&self) -> &zmq::Socket {
        &self.inner
    }

    /// Bind the socket to the given address.
    pub fn bind(&self, address: &str) -> io::Result<()> {
        self.inner.bind(address).map_err(|e| e.into())
    }

    /// Connect the socket to the given address.
    pub fn connect(&self, address: &str) -> io::Result<()> {
        self.inner.connect(address).map_err(|e| e.into())
    }

    /// Subscribe this socket to the given `prefix`.
    pub fn set_subscribe(&self, prefix: &[u8]) -> io::Result<()> {
        self.inner.set_subscribe(prefix).map_err(|e| e.into())
    }

    /// Send a message.
    ///
    /// Due to the provided From implementations, this works for
    /// `&[u8]`, `Vec<u8>` and `&str`, as well as `zmq::Message`
    /// itself.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn send<T>(&self, item: T, flags: i32) -> io::Result<()>
    where
        T: zmq::Sendable,
    {
        let r = self.inner.send(item, zmq::DONTWAIT | flags).map_err(|e| e.into());
        r
    }

    /// Send a multi-part message. Takes any iterator of valid message
    /// types.
    ///
    /// Due to the provided From implementations, this works for
    /// `&[u8]`, `Vec<u8>` and `&str`, as well as `zmq::Message`
    /// itself.
    ///
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn send_multipart<I, T>(&self, iter: I, flags: i32) -> io::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<zmq::Message>,
    {
        let r = self.inner
            .send_multipart(iter, zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Read a single `zmq::Message` from the socket.
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv(&self, msg: &mut zmq::Message, flags: i32) -> io::Result<()> {
        let r = self.inner
            .recv(msg, zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Receive bytes into a slice. The length passed to `zmq_recv` is the length
    /// of the slice. The return value is the number of bytes in the message,
    /// which may be larger than the length of the slice, indicating truncation.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv_into(&self, msg: &mut [u8], flags: i32) -> io::Result<usize> {
        let r = self.inner
            .recv_into(msg, zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Receive a message into a fresh `zmq::Message`.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv_msg(&self, flags: i32) -> io::Result<zmq::Message> {
        let r = self.inner
            .recv_msg(zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Receive a message as a byte vector.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv_bytes(&self, flags: i32) -> io::Result<Vec<u8>> {
        let r = self.inner
            .recv_bytes(zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Receive a `String` from the socket.
    ///
    /// If the received message is not valid UTF-8, it is returned as the
    /// original Vec in the `Err` part of the inner result.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv_string(&self, flags: i32) -> io::Result<Result<String, Vec<u8>>> {
        let r = self.inner
            .recv_string(zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }

    /// Receive a multipart message from the socket.
    ///
    /// Note that this will allocate a new vector for each message part;
    /// for many applications it will be possible to process the different
    /// parts sequentially and reuse allocations that way.
    ///
    /// Any flags set will be combined with `zmq::DONTWAIT`, which is
    /// needed for non-blocking mode. The internal `zmq::Error::EAGAIN`
    /// is automatically translated to `io::ErrorKind::WouldBlock`,
    /// which you MUST handle without failing.
    pub fn recv_multipart(&self, flags: i32) -> io::Result<Vec<Vec<u8>>> {
        let r = self.inner
            .recv_multipart(zmq::DONTWAIT | flags)
            .map_err(|e| e.into());
        r
    }
}

unsafe impl Send for Socket {}

/// This implementation is meant for asynchronous `Read`. It might fail
/// if not handled via polling.
impl Read for Socket {
    /// Asynchronously read a byte buffer from the `Socket`.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let r = self.recv_into(buf, 0)?;
        Ok(r)
    }
}

/// This implementation is meant for asynchronous `Write`. It might fail
/// if not handled via polling.
impl Write for Socket {
    /// Asynchronously write a byte buffer to the `Socket`.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let sent = buf.len();
        let _ = self.send(buf, 0)?;
        Ok(sent)
    }

    /// Flush is not implemented since ØMQ guarantees that a message is
    /// either fully sent, or not sent at all.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl mio::Evented for Socket {
    fn register(
        &self,
        poll: &mio::Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        trace!("ZmqSocket::register: fd={}", fd);
        EventedFd(&fd).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        trace!("ZmqSocket::reregister: fd={}", fd);
        EventedFd(&fd).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        trace!("ZmqSocket::deregister: fd={}", fd);
        EventedFd(&fd).deregister(poll)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use zmq;
    use super::*;

    const TEST_STR: &[u8] = b"test-test-one-2-3";
    const TEST_ADDR: &str = "inproc://test";
    const TEST_BUFFER_SIZE: usize = 64;

    // Returns a `Socket` pair ready to talk to each other.
    fn get_async_test_pair() -> (Socket, Socket) {
        let ctx = Context::new();
        let bound = ctx.socket(zmq::PAIR).unwrap();
        let _ = bound.bind(TEST_ADDR).unwrap();
        let connected = ctx.socket(zmq::PAIR).unwrap();
        let _ = connected.connect(TEST_ADDR).unwrap();
        (bound, connected)
    }

    #[test]
    fn socket_sends_and_receives_a_byte_buffer() {
        let (mut receiver, mut sender) = get_async_test_pair();

        let sent = sender.write(TEST_STR).unwrap();
        assert_eq!(sent, TEST_STR.len());

        let mut buf = vec![0; TEST_BUFFER_SIZE];
        let recvd = receiver.read(&mut buf).unwrap();
        assert_eq!(&buf[..recvd], TEST_STR);
    }
}
