//! Tokio transports for sockets.
use std::io;
use std::ops::Deref;

use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use tokio_core::reactor::PollEvented;
use tokio_io::{AsyncRead, AsyncWrite};
use zmq::{self, Message, Sendable};
use zmq_mio;

use super::{SocketRecv, SocketSend};

/// This implementation uses `PollEvented<_>` polling mechanism to properly send messages with
/// tokio.
impl SocketSend for PollEvented<zmq_mio::Socket> {
    fn send<T>(&self, msg: T, flags: i32) -> io::Result<()>
    where
        T: Sendable,
    {
        if let Async::NotReady = self.poll_write() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().send(msg, flags);
        if is_wouldblock(&r) {
            self.need_write();
        }
        return r;
    }

    fn send_multipart<I, T>(&self, iter: I, flags: i32) -> io::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<Message>,
    {
        if let Async::NotReady = self.poll_write() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().send_multipart(iter, flags);
        if is_wouldblock(&r) {
            self.need_write();
        }
        return r;
    }
}

/// This implementation uses `PollEvented<_>` polling mechanism to properly receive messages with
/// tokio.
impl SocketRecv for PollEvented<zmq_mio::Socket> {
    /// Return true if there are more frames of a multipart message to receive.
    fn get_rcvmore(&self) -> io::Result<bool> {
        let r = self.get_ref().get_rcvmore();
        return r;
    }

    /// Receive a message into a `Message`. The length passed to `zmq_msg_recv` is the length
    /// of the buffer.
    fn recv(&self, buf: &mut Message, flags: i32) -> io::Result<()> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv(buf, flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }

    /// Receive bytes into a slice. The length passed to `zmq_recv` is the length of the slice. The
    /// return value is the number of bytes in the message, which may be larger than the length of
    /// the slice, indicating truncation.
    fn recv_into(&self, buf: &mut [u8], flags: i32) -> io::Result<usize> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv_into(buf, flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }

    /// Receive a message into a fresh `Message`.
    fn recv_msg(&self, flags: i32) -> io::Result<Message> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv_msg(flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }

    /// Receive a message as a byte vector.
    fn recv_bytes(&self, flags: i32) -> io::Result<Vec<u8>> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv_bytes(flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }

    /// Receive a `String` from the socket.
    ///
    /// If the received message is not valid UTF-8, it is returned as the original `Vec` in the `Err`
    /// part of the inner result.
    fn recv_string(&self, flags: i32) -> io::Result<Result<String, Vec<u8>>> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv_string(flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }

    /// Receive a multipart message from the socket.
    ///
    /// Note that this will allocate a new vector for each message part; for many applications it
    /// will be possible to process the different parts sequentially and reuse allocations that
    /// way.
    fn recv_multipart(&self, flags: i32) -> io::Result<Vec<Vec<u8>>> {
        if let Async::NotReady = self.poll_read() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let r = self.get_ref().recv_multipart(flags);
        if is_wouldblock(&r) {
            self.need_read();
        }
        return r;
    }
}

// Convenience function to check if messaging will block or not.
fn is_wouldblock<T>(r: &io::Result<T>) -> bool {
    match *r {
        Ok(_) => false,
        Err(ref e) => e.kind() == io::ErrorKind::WouldBlock,
    }
}

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
