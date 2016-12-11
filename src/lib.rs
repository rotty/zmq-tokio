extern crate mio;
extern crate futures;
extern crate tokio_core;
extern crate zmq;

use std::io;
use std::os::unix::io::RawFd;

use futures::{Async,Poll};
use futures::stream::Stream;

use tokio_core::reactor::{Handle, PollEvented};
use mio::{PollOpt, Ready, Token};
use mio::unix::EventedFd;

#[derive(Clone)]
pub struct Context {
    ctx: zmq::Context,
}

impl Context {
    pub fn new() -> Context {
        Context { ctx: zmq::Context::new() }
    }

    pub fn socket(&self, typ: zmq::SocketType, handle: &Handle) -> io::Result<Socket> {
        let socket = try!(self.ctx.socket(typ).map_err(zmq_io_error));
        Socket::new(socket, handle)
    }
}

// mio integration, might be pushed down to rust-zmq itself?
struct ZmqSocket(zmq::Socket);

impl ZmqSocket {
    fn as_raw_fd(&self) -> io::Result<RawFd> {
        Ok(try!(self.0.get_fd().map_err(zmq_io_error)))
    }
}

pub struct Socket {
    io: PollEvented<ZmqSocket>,
}

impl Socket {
    fn new(socket: zmq::Socket, handle: &Handle) -> io::Result<Socket> {
        let io = try!(PollEvented::new(ZmqSocket(socket), handle));
        Ok(Socket { io: io })
    }

    pub fn bind(&mut self, address: &str) -> io::Result<()> {
         self.io.get_mut().0.bind(address).map_err(zmq_io_error)
    }

    pub fn connect(&mut self, address: &str) -> io::Result<()> {
         self.io.get_mut().0.connect(address).map_err(zmq_io_error)
    }

    pub fn set_subscribe(&mut self, prefix: &[u8]) -> io::Result<()> {
         self.io.get_mut().0.set_subscribe(prefix).map_err(zmq_io_error)
    }

    pub fn send(&self, msg: &[u8]) -> io::Result<()> {
        if let Async::NotReady = self.io.poll_write() {
            return Err(mio::would_block());
        }
        match self.io.get_ref().0.send(msg, zmq::DONTWAIT) {
            Ok(()) => Ok(()),
            Err(zmq::Error::EAGAIN) => {
                self.io.need_write();
                Err(mio::would_block())
            }
            Err(e) => Err(zmq_io_error(e))
        }
    }

    pub fn recv(&self) -> io::Result<zmq::Message> {
        if let Async::NotReady = self.io.poll_read() {
            return Err(mio::would_block());
        }
        match self.io.get_ref().0.recv_msg(zmq::DONTWAIT) {
            Ok(msg) => Ok(msg),
            Err(zmq::Error::EAGAIN) => {
                self.io.need_read();
                Err(mio::would_block())
            }
            Err(e) => Err(zmq_io_error(e)),
        }
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

struct SocketStream<'a> {
    socket: &'a Socket,
    buffer: Vec<Vec<u8>>,
}

// TODO: Make this generic using tokio_core::easy::{Decode,Encode}.
impl<'a> Stream for SocketStream<'a> {
    type Item = Vec<Vec<u8>>;
    type Error = io::Error;

    // fn poll_read(&mut self) -> Async<()> {
    //     self.io.poll_read()
    // }

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.socket.recv() {
                Ok(msg) => {
                    self.buffer.push(Vec::from(&msg[..]));
                    if !msg.get_more() {
                        return Ok(Async::Ready(Some(self.buffer.split_off(0))))
                    }
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => return Ok(Async::NotReady),
                    _ => return Err(e),
                },
            }
        }
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

// As ZMQ uses POSIX errno values (mostly), `zmq:Error` should
// some API that makes this function obsolete.
fn zmq_io_error(e: zmq::Error) -> io::Error {
    match e {
        zmq::Error::EAGAIN => mio::would_block(),
        _ => io::Error::new(io::ErrorKind::Other, e),
    }
}

impl mio::Evented for ZmqSocket {
    fn register(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        poll.register(&EventedFd(&fd), token, interest, opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        poll.reregister(&EventedFd(&fd), token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        let fd = try!(self.as_raw_fd());
        poll.deregister(&EventedFd(&fd))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
