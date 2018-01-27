//! Sinks for sockets.
use std::io;
use std::ops::Deref;

use futures::{Async, AsyncSink, Poll, Sink, StartSend};
use tokio_io::AsyncWrite;
use zmq;

use super::SocketSend;

/// Single-message sink for sockets.
pub struct MessageSink<'a, T: 'a> {
    socket: &'a T,
}

impl<'a, T> MessageSink<'a, T>
where
    T: AsyncWrite + SocketSend + 'a,
{
    pub fn new(socket: &'a T) -> MessageSink<'a, T> {
        MessageSink { socket }
    }
}

impl<'a, T> Sink for MessageSink<'a, T>
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

/// Single-message sink for sockets.
pub struct MultipartMessageSink<'a, T: 'a> {
    socket: &'a T,
}

impl<'a, T> MultipartMessageSink<'a, T>
where
    T: AsyncWrite + SocketSend + 'a,
{
    pub fn new(socket: &'a T) -> MultipartMessageSink<'a, T> {
        MultipartMessageSink { socket }
    }
}

impl<'a, T> Sink for MultipartMessageSink<'a, T>
where
    T: AsyncWrite + SocketSend + 'a,
{
    type SinkItem = Vec<Vec<u8>>;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Vec<Vec<u8>>) -> StartSend<Vec<Vec<u8>>, Self::SinkError> {
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
