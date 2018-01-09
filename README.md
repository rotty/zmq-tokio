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

# Examples

## Sending and receiving simple messages with futures

A PAIR of sockets is created. The `sender` socket sends
a message, and the `receiver` gets it.

Everything runs within on a tokio reactor.

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


fn main() {
    let mut reactor = Core::new().unwrap();
    let context = Context::new();

    let recvr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();

    // Step 1: send any type implementing `Into<zmq::Message>`,
    //         meaning `&[u8]`, `Vec<u8>`, `String`, `&str`,
    //         and `zmq::Message` itself.
    let send_future = sendr.send("this message will be sent");

    // Step 2: receive the message on the pair socket
    let recv_msg = send_future.and_then(|_| {
        recvr.recv()
    });

    // Step 3: process the message and exit
    let process_msg = recv_msg.and_then(|msg| {
        assert_eq!(msg.as_str(), Some("this message will be sent"));
        Ok(())
    });

    let _ = reactor.run(process_msg).unwrap();
}
```

## Sending and receiving multi-part messages with futures

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

fn main() {
    let mut reactor = Core::new().unwrap();
    let context = Context::new();

    let recvr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();

    let msgs: Vec<Vec<u8>> = vec![b"hello".to_vec(), b"goodbye".to_vec()];
    // Step 1: send a vector of byte-vectors, `Vec<Vec<u8>>`
    let send_future = sendr.send_multipart(msgs);

    // Step 2: receive the message on the pair socket
    let recv_msg = send_future.and_then(|_| {
        recvr.recv_multipart()
    });

    // Step 3: process the message and exit
    let process_msg = recv_msg.and_then(|msgs| {
        assert_eq!(msgs[0].as_str(), Some("hello"));
        assert_eq!(msgs[1].as_str(), Some("goodbye"));
        Ok(())
    });

    let _ = reactor.run(process_msg).unwrap();
}
```

## Manual handling using tranports with `Sink` and `Stream`

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


fn main() {
    let mut reactor = Core::new().unwrap();
    let context = Context::new();

    let recvr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = context.socket(PAIR, &reactor.handle()).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();


    let (_, rx) = recvr.framed().split();
    let (tx, _) = sendr.framed().split();

    let msg = zmq::Message::from_slice(b"hello there");

    // Step 1: start a stream with only one item.
    let start_stream = stream::iter_ok::<_, ()>(vec![(tx, rx, msg)]);

    // Step 2: send the message
    let send_msg = start_stream.and_then(|(tx, rx, msg)| {
            // send a message to the receiver.
            // return a future with the receiver
            let _ = tx.send(msg);
            Ok(rx)
        });

    // Step 3: read the message
    let fetch_msg = send_msg.for_each(|rx| {
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

    // Run the stream
    let _ = reactor.run(fetch_msg).unwrap();

    // Exit our program, playing nice.
    ::std::process::exit(0);
}
```
