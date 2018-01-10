# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `CHANGELOG.md`, is this file.
- `README.md`, a basic description about this library.
- Example for `README.md` echoing a single message `mio::Poll`.
- Doctest in `src/lib.rs` echoing a single message `mio::Poll`.
- `zmq_mio::Context` mirrors the traits from `zmq::Context`.
- `zmq_mio::Context::get_inner` method gets a clone of the underlying `zmq::Context`
- `zmq_mio::Socket` implements non-blocking `Read` and `Write` from `std::io`
- `zmq_mio::Socket` mirrors the traits from `zmq::Socket`.
- `zmq_mio::Socket` has `send`, `send_multipart`, `recv`, `recv_multipart`, `recv_bytes`, `recv_string`, `recv_into`, and `recv_msg`. This expands our coverage of the `zmq::Socket` API.

### Changed
- Replaced `&mut self` arguments that are no longer needed.
- Updated cargo dependencies.

## [0.0.1] - 2017-02-22
### Added
- `tests/echo.rs`, integration test for poll-driven message echoing.
- `zmq_mio::Socket`, basic implementation of a `mio::Evented` wrapper of `zmq::Socket`.
- `zmq_mio::Context`, a wrapper of `zmq::Context` that builds `zmq_mio::Socket` instances.
