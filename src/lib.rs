extern crate mio;
extern crate futures;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio_core;
extern crate zmq;

use std::io;
use std::os::unix::io::RawFd;

use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;

use tokio_core::reactor::{Handle, PollEvented};
use mio::{PollOpt, Ready, Token};
use mio::unix::EventedFd;

fn is_wouldblock<T>(r: &io::Result<T>) -> bool {
    match *r {
        Ok(_) => false,
        Err(ref e) => e.kind() == io::ErrorKind::WouldBlock,
    }
}

#[derive(Clone)]
pub struct Context {
    ctx: zmq::Context,
}

impl Context {
    pub fn new() -> Context {
        Context { ctx: zmq::Context::new() }
    }

    pub fn socket(&self, typ: zmq::SocketType, handle: &Handle) -> io::Result<Socket> {
        let socket = try!(self.ctx.socket(typ));
        Socket::new(socket, handle)
    }
}

// mio integration, should probably be put into its own crate eventually
struct ZmqSocket {
    inner: zmq::Socket,
}

impl ZmqSocket {
    fn new(socket: zmq::Socket) -> Self {
        ZmqSocket { inner: socket }
    }

    fn as_raw_fd(&self) -> io::Result<RawFd> {
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
            if mask.contains(events) { value } else { Ready::none() }
        };
        Ok(ready(zmq::POLLOUT, Ready::writable()) |
           ready(zmq::POLLIN, Ready::readable()))
    }

    pub fn send<T>(&self, item: T) -> io::Result<()>
        where T: Into<zmq::Message>
    {
        let r = self.inner.send(item, zmq::DONTWAIT).map_err(|e| e.into());
        r
    }

    pub fn recv(&self) -> io::Result<zmq::Message> {
        let r = self.inner.recv_msg(zmq::DONTWAIT).map_err(|e| e.into());
        r
    }
}

pub struct Socket {
    io: PollEvented<ZmqSocket>,
}

impl Socket {
    fn new(socket: zmq::Socket, handle: &Handle) -> io::Result<Socket> {
        let io = try!(PollEvented::new(ZmqSocket::new(socket), handle));
        Ok(Socket { io: io })
    }

    pub fn bind(&mut self, address: &str) -> io::Result<()> {
         self.io.get_mut().bind(address)
    }

    pub fn connect(&mut self, address: &str) -> io::Result<()> {
         self.io.get_mut().connect(address)
    }

    pub fn set_subscribe(&mut self, prefix: &[u8]) -> io::Result<()> {
         self.io.get_ref().set_subscribe(prefix)
    }

    pub fn send<T>(&mut self, item: T) -> io::Result<()>
        where T: Into<zmq::Message>
    {
        trace!("entering send");
        if !try!(self.poll_events()).is_writable() {
            trace!("send - not ready");
            return Err(mio::would_block());
        }
        trace!("attempting send");
        let r = self.io.get_ref().send(item);
        if is_wouldblock(&r) {
            self.io.need_read();
        }
        trace!("send - {:?}", r);
        r
    }

    pub fn recv(&mut self) -> io::Result<zmq::Message> {
        trace!("entering recv");
        if !try!(self.io.get_ref().poll_events()).is_readable() {
            trace!("recv - not ready");
            return Err(mio::would_block());
        }
        let r = self.io.get_ref().recv();
        if is_wouldblock(&r) {
            self.io.need_read();
        }
        trace!("recv - {:?}", r);
        r
    }

    fn poll_events(&mut self) -> io::Result<Ready> {
        self.io.get_ref().poll_events()
    }

    pub fn framed(self) -> SocketFramed {
        SocketFramed::new(self)
    }

    // pub fn recv_iter(&self) -> io::Result<MultiRecv> {
    //     if let Async::NotReady = self.io.poll_read() {
    //         return Err(mio::would_block());
    //     }
    //     let mut multi = self.io.get_ref().0.recv_iter(zmq::DONTWAIT);
    //     match multi.next() {
    //         None => Ok(MultiRecv { msg: None, inner: multi }),
    //         Some(Ok(msg)) => Ok(MultiRecv { msg: Some(msg), inner: multi }),
    //         Some(Err(zmq::Error::EAGAIN)) => {
    //             self.io.need_read();
    //             Err(mio::would_block())
    //         }
    //         Some(Err(e)) => Err(zmq_io_error(e)),
    //     }
    // }
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
        let r = try_nb!(self.socket.send(&self.wr[..]));
        trace!("send complete: {:?}", r);
        self.wr.clear();
        Ok(Async::Ready(()))
    }
}

// TODO: Make this generic using a codec
impl Stream for SocketFramed {
    type Item = Vec<Vec<u8>>;
    type Error = io::Error;

    // fn poll_read(&mut self) -> Async<()> {
    //     self.io.poll_read()
    // }

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("SocketFramed::poll()");
        let r = self.socket.recv();
        if is_wouldblock(&r) {
            return Ok(Async::NotReady);
        }
        let msg = try!(r);
        if self.rd.len() == 0 && msg.len() == 0 {
            return Ok(Async::Ready(None));
        }
        self.rd.push(Vec::from(&msg[..]));
        while msg.get_more() {
            let r = self.socket.recv();
            let msg = try!(r);
            self.rd.push(Vec::from(&msg[..]));
        }
        return Ok(Async::Ready(Some(self.rd.split_off(0))))
    }

    // fn poll_write(&mut self) -> Async<()> {
    //     self.io.poll_write()
    // }

    // fn write(&mut self, msg: Self::In) -> Poll<(), io::Error> {
    //     match self.send(&msg) {
    //         Ok(_) => Ok(Async::Ready((    //         Err(e) => match e.kind() {
    //             io::ErrorKind::WouldBlock => Ok(Async::NotReady),
    //             _ => Err(e)
    //         }
    //     }
    // }

    // fn flush(&mut self) -> Poll<(), io::Error> {
    //     // not yet sure if we can or should do anything here
    //     Ok(Async::Ready(()))
    // }
}

// // Not sure if this should directly be trait of Socket
// impl Stream for Socket {
//     type Item = Vec<u8>;
//     type Error = io::Error;

//     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
//         FramedIo::read(self)
//     }
// }

impl mio::Evented for ZmqSocket {
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
