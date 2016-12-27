extern crate mio;
extern crate futures;

#[macro_use]
extern crate log;

#[macro_use]
extern crate tokio_core;
extern crate zmq;
extern crate zmq_mio;

pub mod codec;

use std::io;

use futures::{Async, AsyncSink, Poll, StartSend};
use futures::stream::Stream;
use futures::sink::Sink;
use mio::Ready;
use tokio_core::reactor::{Handle, PollEvented};

use zmq::Message;

use codec::MessageCodec;

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

pub struct Socket {
    io: PollEvented<zmq_mio::Socket>,
}

impl Socket {
    fn new(socket: zmq::Socket, handle: &Handle) -> io::Result<Socket> {
        let io = try!(PollEvented::new(zmq_mio::Socket::new(socket), handle));
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

    pub fn framed<C>(self, codec: C) -> SocketFramed<C> {
        SocketFramed::new(self, codec)
    }
}

pub struct SocketFramed<C> {
    socket: Socket,
    codec: C,
    rd: Vec<Message>,
    wr: Option<Message>,
}

impl<C> SocketFramed<C> {
    fn new(socket: Socket, codec: C) -> Self {
        SocketFramed {
            socket: socket,
            codec: codec,
            rd: Vec::new(),
            wr: None,
        }
    }
}

// TODO: Make this generic using a codec
impl<C: MessageCodec> Sink for SocketFramed<C> {
    type SinkItem = C::Out;
    type SinkError = io::Error;

    fn start_send(&mut self, item: C::Out) -> StartSend<C::Out, Self::SinkError> {
        //trace!("start send ({:?})", item);
        if self.wr.is_some() {
            try!(self.poll_complete());
            if self.wr.is_some() {
                return Ok(AsyncSink::NotReady(item));
            }
        }
        self.wr = Some(try!(self.codec.encode(item)));
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        //trace!("polling sink for completion ({:?})", self.wr.map(|msg| msg.len()));
        match self.wr.take() {
            None => Ok(Async::Ready(())),
            Some(msg) => {
                let r = try_nb!(self.socket.send(&msg[..]));
                trace!("send complete: {:?}", r);
                Ok(Async::Ready(()))
            },
        }
    }
}

// TODO: Make this generic using a codec
impl<C> Stream for SocketFramed<C> {
    type Item = Vec<Message>;
    type Error = io::Error;

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
        let mut more = msg.get_more();
        self.rd.push(msg);
        while more {
            let r = self.socket.recv();
            let msg = try!(r);
            more = msg.get_more();
            self.rd.push(msg);
        }
        return Ok(Async::Ready(Some(self.rd.split_off(0))))
    }
}
