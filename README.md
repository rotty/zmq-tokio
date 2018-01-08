This is the barely budding seed of providing access to
[ZeroMQ](http://zeromq.org/) via the tokio async I/O abstraction.

Status
------

This project is in its very infancy. Do not expect to be able to build
something useful on top of this (yet). The API will certainly change
wildly before approaching some kind of stability.

Currently this repo provides a rough proof-of-concept implementation
of a client-server (`ZMQ_REQ`/`ZMQ_REP`) interaction in
`examples/req-rep-single-threaded.rs`. The underlying library API is
sketched just as far as needed to meet the needs of this example.

# Example

## Send/Recv with futures.

```rust
extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;
extern crate zmq;

use futures::stream;
use futures::{Future, Sink, Stream};
use tokio_core::reactor::Core;

use zmq_tokio::{Context, Socket, PAIR};

const TEST_ADDR: &str = "inproc://test";

fn test_pair() -> (Socket, Socket, Core) {
    let reactor = Core::new().unwrap();
    let handle = reactor.handle();
    let ctx = Context::new();

    let recvr = ctx.socket(PAIR, &handle).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = ctx.socket(PAIR, &handle).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();

    (recvr, sendr, reactor)
}

fn main() {
    let (mut recvr, mut sendr, mut reactor) = test_pair();
    let msg = zmq::Message::from_slice(b"this message will be sent");

    let send_future = sendr.send(msg).and_then(|_| {
        recvr.recv()
    }).and_then(|msg| {
        assert_eq!(msg.as_str(), Some("this message will be sent"));
        Ok(())
    });

    let _ = reactor.run(send_future).unwrap();
}
```


## Using tokio transports.
Here's a working example with two `PAIR` sockets interacting, using
tokio transports directly.

A `sender` socket sends a message, and a `receiver` socket reads it.
Then, the program stops.

```rust
extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;
extern crate zmq;

use futures::stream;
use futures::{Future, Sink, Stream};
use tokio_core::reactor::Core;

use zmq_tokio::{Context, Socket, PAIR};

const TEST_ADDR: &str = "inproc://test";

fn test_pair() -> (Socket, Socket, Core) {
    let reactor = Core::new().unwrap();
    let handle = reactor.handle();
    let ctx = Context::new();

    let recvr = ctx.socket(PAIR, &handle).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = ctx.socket(PAIR, &handle).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();

    (recvr, sendr, reactor)
}

fn main() {
    let (recvr, sendr, mut reactor) = test_pair();

    let (_, rx) = recvr.framed().split();
    let (tx, _) = sendr.framed().split();

    let msg = zmq::Message::from_slice(b"hello there");

    let start_stream = stream::iter_ok::<_, ()>(vec![(tx, rx, msg)]);
    let send_msg = start_stream.and_then(|(tx, rx, msg)| {
            // send a message to the receiver.
            // return a future with the receiver
            let _ = tx.send(msg);
            Ok(rx)
        });
    let fetch_response = send_msg.for_each(|rx| {
            // process the first response that the
            // receiver gets.
            // Assert that it equals the message sent
            // by the sender.
            // returns `Ok(())` when the stream ends.
            let _ = rx.into_future().and_then(|(response, _)| {
                match response {
                    Some(msg) => assert_eq!(msg.as_str(), Some("hello there")),
                    None => panic!("expected a response"),
                }
                Ok(())
            });
            Ok(())
        });

    let _ = reactor.run(fetch_response).unwrap();
}
```
