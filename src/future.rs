//! Futures for ØMQ tasks.
use std::io;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};

use futures::{Async, Future, Poll};
use zmq;

use super::Socket;

/// A Future that sends a `zmq::Message` asynchronously. This is returned by `Socket::send`
pub struct SendMessage<'a> {
    socket: &'a mut Socket,
    message: zmq::Message,
}

impl<'a> SendMessage<'a> {
    pub fn new(socket: &'a mut Socket, message: zmq::Message) -> SendMessage {
        SendMessage { socket, message }
    }
}

impl<'a> Future for SendMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.socket.write(self.message.deref()) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(_) => {
                Ok(Async::Ready(()))
            }
        }
    }
}

/// A Future that receives a `zmq::Message` asynchronously. This is returned by `Socket::recv`
pub struct ReceiveMessage<'a> {
    socket: &'a mut Socket,
}

impl<'a> ReceiveMessage<'a> {
    pub fn new(socket: &'a mut Socket) -> ReceiveMessage {
        ReceiveMessage { socket }
    }
}

impl<'a> Future for ReceiveMessage<'a> {
    type Item = zmq::Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut buf = zmq::Message::with_capacity(1024);
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
                Ok(Async::Ready(buf))
            }
        }
    }
}
