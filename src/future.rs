//! Futures for ØMQ tasks.
use std::io;

use futures::{Async, Future, Poll};
use zmq;

use super::Socket;

/// A Future that sends a `zmq::Message` asynchronously. This is returned by `Socket::send`
pub struct SendMessage<'a> {
    socket: &'a Socket,
    message: zmq::Message,
}

impl<'a> SendMessage<'a> {
    pub fn new(socket: &'a Socket, message: zmq::Message) -> SendMessage {
        SendMessage { socket, message }
    }
}

impl<'a> Future for SendMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.socket.get_mio_ref().send(&*self.message, 0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(_) => Ok(Async::Ready(())),
        }
    }
}

/// A Future that sends a multi-part `zmq::Message` asynchronously.
/// This is returned by `Socket::send_multipart`
pub struct SendMultipartMessage<'a> {
    socket: &'a Socket,
    messages: Vec<Vec<u8>>,
}

impl<'a> SendMultipartMessage<'a> {
    pub fn new<I, T>(socket: &'a Socket, iter: I) -> SendMultipartMessage
    where
        I: IntoIterator<Item = T>,
        T: Into<Vec<u8>>,
    {
        let messages: Vec<Vec<u8>> = iter.into_iter().map(|m| m.into()).collect();
        SendMultipartMessage { socket, messages }
    }
}

impl<'a> Future for SendMultipartMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.socket.get_mio_ref().send_multipart(&self.messages, 0) {
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

/// A Future that receives a multi-part `zmq::Message` asynchronously.
/// This is returned by `Socket::recv_multipart`
pub struct ReceiveMultipartMessage<'a> {
    socket: &'a Socket,
}

impl<'a> ReceiveMultipartMessage<'a> {
    pub fn new(socket: &'a Socket) -> ReceiveMultipartMessage {
        ReceiveMultipartMessage { socket }
    }
}

impl<'a> Future for ReceiveMultipartMessage<'a> {
    type Item = Vec<zmq::Message>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.socket.get_mio_ref().recv_multipart(0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(msgs) => {
                let m_out = msgs.iter().map(|v| v.into()).collect::<Vec<zmq::Message>>();
                Ok(Async::Ready(m_out))
            }
        }
    }
}

/// A Future that receives a `zmq::Message` asynchronously. This is returned by `Socket::recv`
pub struct ReceiveMessage<'a> {
    socket: &'a Socket,
}

impl<'a> ReceiveMessage<'a> {
    pub fn new(socket: &'a Socket) -> ReceiveMessage {
        ReceiveMessage { socket }
    }
}

impl<'a> Future for ReceiveMessage<'a> {
    type Item = zmq::Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.socket.get_mio_ref().recv_msg(0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(msg) => {
                Ok(Async::Ready(msg))
            }
        }
    }
}
