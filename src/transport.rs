//! Tokio transports for sockets.
use std::io;
use std::ops::{Deref, DerefMut};

use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use tokio_io::{AsyncRead, AsyncWrite};
use zmq;

use super::{SocketRecv, SocketSend};

/// Tokio transport for one-part messages.
pub struct MessageTransport<'a, T: 'a> {
    socket: &'a T,
}

impl<'a, T> MessageTransport<'a, T>
where
    T: AsyncRead + AsyncWrite + SocketRecv + SocketSend + 'a,
{
    pub fn new(socket: &'a T) -> MessageTransport<'a, T> {
        MessageTransport { socket }
    }
}

impl<'a, T> Sink for MessageTransport<'a, T>
where
    T: AsyncWrite + SocketSend + 'a,
{
    type SinkItem = zmq::Message;
    type SinkError = io::Error;

    fn start_send(&mut self, item: zmq::Message) -> StartSend<zmq::Message, Self::SinkError> {
        match SocketSend::send(self.socket, item.deref(), 0) {
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

impl<'a, T> Stream for MessageTransport<'a, T>
where
    T: AsyncRead + SocketRecv + 'a,
{
    type Item = zmq::Message;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let mut buf = zmq::Message::new();
        match SocketRecv::recv(self.socket, &mut buf, 0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(_) => Ok(Async::Ready(Some(buf))),
        }
    }
}

/// Tokio transport for multipart-messages.
pub struct MultipartMessageTransport<'a, T: 'a> {
    socket: &'a T,
}

impl<'a, T> MultipartMessageTransport<'a, T>
where
    T: AsyncRead + AsyncWrite + SocketRecv + SocketSend + 'a,
{
    pub fn new(socket: &'a T) -> MultipartMessageTransport<'a, T> {
        MultipartMessageTransport { socket }
    }
}

impl<'a, T> Sink for MultipartMessageTransport<'a, T>
where
    T: AsyncWrite + SocketSend + 'a,
{
    type SinkItem = Vec<Vec<u8>>;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Vec<Vec<u8>>) -> StartSend<Vec<Vec<u8>>, Self::SinkError> {
        trace!("SocketFramed::start_send()");
        match SocketSend::send_multipart(self.socket, &item, 0) {
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

impl<'a, T> Stream for MultipartMessageTransport<'a, T>
where
    T: AsyncRead + SocketRecv + 'a,
{
    type Item = Vec<zmq::Message>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match SocketRecv::recv_multipart(self.socket, 0) {
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    Ok(Async::NotReady)
                } else {
                    Err(e)
                }
            }
            Ok(vecs) => {
                let msgs = vecs.iter().map(|v| {
                    v.into()
                }).collect();
                Ok(Async::Ready(Some(msgs)))
            }
        }
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
    pub fn new(socket: T) -> Self {
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
        let mut buf = zmq::Message::with_capacity(1024);
        trace!("SocketFramed::poll()");
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
                Ok(Async::Ready(Some(buf)))
            }
        }
    }
}
