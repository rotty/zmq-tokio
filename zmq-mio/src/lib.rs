extern crate mio;
extern crate zmq;

use std::io;
#[macro_use] extern crate log;

use std::os::unix::io::RawFd;

use mio::unix::EventedFd;
use mio::{PollOpt, Ready, Token};

#[derive(Clone)]
pub struct Context {
    inner: zmq::Context,
}

impl Context {
    pub fn new() -> Self {
        Context { inner: zmq::Context::new() }
    }

    pub fn socket(&self, typ: zmq::SocketType) -> io::Result<Socket> {
        Ok(Socket::new(try!(self.inner.socket(typ))))
    }
}

// mio integration, should probably be put into its own crate eventually
pub struct Socket {
    inner: zmq::Socket,
}

impl Socket {
    pub fn new(socket: zmq::Socket) -> Self {
        Socket { inner: socket }
    }

    pub fn as_raw_fd(&self) -> io::Result<RawFd> {
        let fd = try!(self.inner.get_fd());
        trace!("socket raw FD: {}", fd);
        Ok(fd)
    }

    pub fn bind(&mut self, address: &str) -> io::Result<()> {
         self.inner.bind(address).map_err(|e| e.into())
    }

    pub fn connect(&mut self, address: &str) -> io::Result<()> {
         self.inner.connect(address).map_err(|e| e.into())
    }

    pub fn set_subscribe(&self, prefix: &[u8]) -> io::Result<()> {
         self.inner.set_subscribe(prefix).map_err(|e| e.into())
    }

    pub fn poll_events(&self) -> io::Result<Ready> {
        let events = try!(self.inner.get_events());
        let ready = |mask: zmq::PollEvents, value| {
            if mask.contains(events) { value } else { Ready::empty() }
        };
        Ok(ready(zmq::POLLOUT, Ready::writable()) |
           ready(zmq::POLLIN, Ready::readable()))
    }

    pub fn send<T>(&self, item: T) -> io::Result<()>
        where T: zmq::Sendable
    {
        let r = self.inner.send(item, zmq::DONTWAIT).map_err(|e| e.into());
        r
    }

    pub fn recv(&self) -> io::Result<zmq::Message> {
        let r = self.inner.recv_msg(zmq::DONTWAIT).map_err(|e| e.into());
        r
    }
}

impl mio::Evented for Socket {
    fn register(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        trace!("ZmqSocket::register: fd={}", fd);
        EventedFd(&fd).register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
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
