# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
