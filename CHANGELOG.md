# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `zmq_tokio::Socket::outgoing_multipart` returns a `MultiMessageSink`.
- `zmq_tokio::Socket::outgoing` returns a `MessageSink`.
- Added `MultipartMessageSink` for multi-part message sink.
- Added `MessageSink` for a single-part message sink.
- Added `zmq_tokio::convert_into_tokio_socket` function as a convenience for developers.
- Added `examples/requester-multipart.rs`, a REQ client that sends a couple of multi-part messages, always getting a response.
- Added `examples/responder-multipart.rs`, a REP server that listens for incoming multi-part messages, responding with another message.
- Added `examples/requester.rs`, a REQ client that sends a couple of single-part messages, always getting a response.
- Added `examples/responder.rs`, a REP server that listens for incoming single-part messages, responding with another message.
- Added `zmq_tokio::transport::MessageTransport`, which implements both `Sink` and `Stream`, for handling single-part messages.
- Added `zmq_tokio::transport::MultipartMessageTransport`, which implements both `Sink` and `Stream`, for handling multi-part messages.
- `zmq_tokio::Socket::incoming_multipart` returns a `MultiMessageStream`.
- `zmq_tokio::Socket::incoming` returns a `MessageStream`.
- Added `MultipartMessageStream` for multi-part message streaming.
- Added `MessageStream` for single-part message streaming.
- Defined the `SocketRecv` trait to have a method API for receiving messages with ZeroMQ.
- Defined the `SocketSend` trait to have a method API for sending messages with ZeroMQ.

### Changed
- Cleaned-up the prelude by removing piecewise re-exports from `zmq`, in favor of re-exporiting the whole crate.
- Remove paragraph mentioning non-existing example in `README.md`.

### Fixed
- `zmq_tokio::Socket::get_ref` replaces `zmq_tokio::Socket_get_mio_ref`. The new `get_ref` method returns the inner `&PollEvented<zmq_mio::Socket>`. `get_mio_ref` is now private, pending removal.
- Future types now use `SocketRecv + AsyncRead` and `SocketSend + AsyncWrite` trait boundaries. Previously, the underlying `zmq_mio::Socket` from `PollEvented<zmq_mio::Socket>` was being used, instead of the poll-evented socket itself. The fix is made by implementing `SocketRecv` and `SocketSend` for `PollEvented<zmq_mio::Socket>`, and having the trait methods use the proper tokio polling-mechanisms (particularly using `need_read()` and `need_write()` from the poll-evented socket)..

## [Unreleased] 0.0.1-future 2018-01-10
### Added
- `CHANGELOG.md`, is this file.
- Example for `README.md` echoing a single message with futures.
- Example for `README.md` echoing a multipart message with futures.
- Example for `README.md` echoing a single message with tokio transports.
- Doctest in  `src/lib.rs` echoing a single message with futures.
- Doctest in  `src/lib.rs` echoing a multipart message with futures.
- Doctest in  `src/lib.rs` echoing a single message with tokio transports.
- Re-export `SocketType`, `Message`, `SNDMORE`, `Error`, `SocketType::*`, from the `rust-zmq` crate, for convenience. This should make `extern crate zmq_tokio` more attractive than `extern crate zmq`, when dealing with async communications.
- `zmq_tokio::Context` mirrors the traits from `zmq_mio::Context`.
- `zmq_tokio::Context::get_inner` method gets a clone of the underlying `zmq_mio::Context`
- `zmq_tokio::Socket` implements `Read`, `Write` from `std::io`.
- `zmq_tokio::Socket` implements `AsyncRead` and `AsyncWrite` from `tokio_io::io`.
- `zmq_tokio::Socket` mirrors the traits from `zmq_mio::Socket`.
- `zmq_tokio::Socket` methods `send`, `send_multipart`, `recv`, `recv_multipart`. These methods return futures, and will not work outside of a tokio task. This expands our coverage of the `zmq::Socket` API.
- `zmq_tokio::future::SendMessage` is a future type returned by `zmq_tokio::Socket::send`.
- `zmq_tokio::future::SendMultipartMessage` is a future type returned by `zmq_tokio::Socket::send_multipart`.
- `zmq_tokio::future::ReceiveMessage` is a future type returned by `zmq_tokio::Socket::recv`.
- `zmq_tokio::future::ReceiveMultipartMessage` is a future type returned by `zmq_tokio::Socket::recv_multipart`.

### Changed
- Replaced `&mut self` arguments that are no longer needed.
- Updated cargo dependencies.

## [0.0.1] - 2017-02-22
### Added
- `README.md`, a basic description about this library.
- `tests/smoke.rs`: integration test for poll-driven, asynchronous, sockets.
- `zmq_tokio::Context`, wrapper for `zmq_mio::Context`, which builds `zmq_tokio::Socket` instances.
- `zmq_tokio::Socket`, wrapper for `tokio_core::reactor::PollEvented<zmq_mio::Socket>`, capable of running on a tokio reactor.
- `zmq_tokio::SocketFramed`, a type that implements `futures::Sink` and `futures::Stream`.
