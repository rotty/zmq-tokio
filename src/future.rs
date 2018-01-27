//! Futures for ØMQ sockets.
use std::io;

use futures::{Async, Future, Poll};

use super::{SocketSend, SocketRecv};
use super::{Message, Socket};

/// A Future that sends a `Message` asynchronously. This is returned by `Socket::send`
pub struct SendMessage<'a> {
    socket: &'a Socket,
    message: Message,
}

impl<'a> SendMessage<'a> {
    pub fn new(socket: &'a Socket, message: Message) -> SendMessage {
        SendMessage { socket, message }
    }
}

impl<'a> Future for SendMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match SocketSend::send(self.socket.get_ref(), &*self.message, 0) {
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

/// A Future that sends a multi-part `Message` asynchronously.
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
        match SocketSend::send_multipart(self.socket.get_ref(), &self.messages, 0) {
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

/// A Future that receives a multi-part `Message` asynchronously.
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
    type Item = Vec<Message>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match SocketRecv::recv_multipart(self.socket.get_ref(), 0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(msgs) => {
                let m_out = msgs.iter().map(|v| v.into()).collect::<Vec<Message>>();
                Ok(Async::Ready(m_out))
            }
        }
    }
}

/// A Future that receives a `Message` asynchronously. This is returned by `Socket::recv`
pub struct ReceiveMessage<'a> {
    socket: &'a Socket,
}

impl<'a> ReceiveMessage<'a> {
    pub fn new(socket: &'a Socket) -> ReceiveMessage {
        ReceiveMessage { socket }
    }
}

impl<'a> Future for ReceiveMessage<'a> {
    type Item = Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match SocketRecv::recv_msg(self.socket.get_ref(), 0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(msg) => Ok(Async::Ready(msg)),
        }
    }
}
