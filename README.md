ØMQ (ZeroMQ) for tokio
======================

Run asynchronous ØMQ sockets with Rust's `tokio` framework.

>   [Rust](https://www.rust-lang.org/) is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.

>   [ZeroMQ](http://zeromq.org/) (also known as ØMQ, 0MQ, or zmq) looks like an embeddable networking library but acts like a concurrency framework. It gives you sockets that carry atomic messages across various transports like in-process, inter-process, TCP, and multicast. You can connect sockets N-to-N with patterns like fan-out, pub-sub, task distribution, and request-reply. It's fast enough to be the fabric for clustered products. Its asynchronous I/O model gives you scalable multicore applications, built as asynchronous message-processing tasks. It has a score of language APIs and runs on most operating systems. ZeroMQ is from iMatix and is LGPLv3 open source.

>   [mio](https://github.com/carllerche/mio). Lightweight non-blocking IO.

>   [futures](https://github.com/alexcrichton/futures-rs). Zero-cost futures and streams in Rust.

>   [Tokio](https://tokio.rs/). A platform for writing fast networking code with Rust.


This crate uses [rust-zmq](https://github.com/erickt/rust-zmq)'s bindings.

Status
------

[![Build Status-dev](https://travis-ci.org/saibatizoku/zmq-tokio.svg?branch=saiba-dev)](https://travis-ci.org/saibatizoku/zmq-tokio)

* [`zmq-futures` documentation](https://saibatizoku.github.io/zmq-tokio/zmq_futures/index.html)
* [`zmq-mio` documentation](https://saibatizoku.github.io/zmq-tokio/zmq_mio/index.html)
* [`zmq-tokio` documentation](https://saibatizoku.github.io/zmq-tokio/zmq_tokio/index.html)


This is the barely budding seed of providing access to
ZeroMQ via the tokio async I/O abstraction.

This project is in its very infancy. Do not expect to be able to build
something useful on top of this (yet). The API will certainly change
wildly before approaching some kind of stability.

The underlying library API is still not complete.

Justification
-------------

ZeroMQ is a [fully-documented](http://zguide.zeromq.org/page:all) library that is meant to act as an _intelligent transport layer_ for messaging patterns. It's portable across OS platforms, as well as across programming languages.

Rust's bindings for the ZeroMQ library, [rust-zmq](https://github.com/erickt/rust-zmq), wrap the C library into a very ergonomic implementation of **most** of the API.

ZeroMQ's model is asynchronous by nature, however, it is handled automatically unless the `DONTWAIT` flag is specified when sending/receiving. This flag configures the socket to non-blocking mode, for which the user API states that there is a need for IO error handling, specifically for the case of `std::io::ErrorKind::WouldBlock`.

So, to properly use ZMQ sockets asynchronously, it is necessary to have some higher-level code that can enforce correct handling of non-blocking messaging. In Rust, this is what `tokio` can do, with certain adaptations to the underlying ZMQ socket.

These adaptions are carried out by `mio`, which helps build the bridge that connects the synchronous with the asynchronous, by adding non-blocking compatibility in the form of `std::io::Error`, as well as a polling mechanism that is meant to be cross-platform, and which is implemented via the `mio::Evented` trait. Since `rust-zmq` provides a `RawFd` reference to the ZMQ socket, and since `mio` provides the wrapper type `EventedFd`, making a ZMQ socket `mio`-compatible, is basically a matter of using the `DONTWAIT` flag on the socket, making sure that our functions and methods return `Result<_, io::Error>`, and wrap it into a new type that uses `EventedFd` to automatically make everything work for polling.

Finally, `futures` in Rust, which aim to `provide a robust implementation of handling asynchronous computations, ergonomic composition and usage, and zero-cost abstractions over what would otherwise be written by hand`, are way that data is handled and processed in `tokio`.

It can be argued that integrating ZeroMQ into the `tokio` ecosystem is a perfect match.

It can be used to extend existing ZeroMQ infrastructure, as well as to create new network components that are guaranteed to be safe, fast, and concurrent. The Rust way.

Goals
-----

- [X] The main goal, is to bring in ZeroMQ's messaging patterns, via their sockets, into the `tokio` ecosystem.
- [X] To bridge the existing asynchronous support provided by ZeroMQ, via `rust-zmq`, with tokio's `polling mechanism.
- [X] To create `Future`, `Stream`, and `Sink` types that blend in with the existing `zmq::Socket` API.
- [X] To provide full technical documentation.
- [ ] To provide end-user documentation.
- [ ] To provide end-user examples.


Usage
=====

For a typical crate, you need at least `futures`, `tokio\_core` and `zmq\_tokio`.

Add this to your `Cargo.toml`:

```cargo
[dependencies]
futures = "0.1"
tokio_core = "0.1"
zmq_tokio = { git = "https://github.com/saibatizoku/zmq-tokio.git" }
```

Then, on your crates main module, for example `src/main.rs`:

```rust
extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::Future;
use tokio_core::reactor::Core;
use zmq_tokio::{Context, Socket, PUB};


fn main() {
    // Start the ZMQ context and the tokio reactor.
    let context = Context::new();
    let mut reactor = Core::new().unwrap();

    // Create a publisher socket, connected to a given address.
    let socket = context.socket(PUB, &reactor.handle()).unwrap();
    let _ = socket.connect("inproc://test").unwrap();

    // Create a future that will send a simple message to subscribers.
    let send_msg = socket.send(b"HELLO");

    // Execute the future on the reactor.
    let _ = core.run(run_pub).unwrap();
}
```

Examples
========

Please consult the [examples](./examples) directory.
