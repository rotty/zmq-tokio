# Asynchronous ØMQ, a.k.a.(ZeroMQ) in Rust with mio.

Run ØMQ sockets that implement mio::Evented, as well as non-blocking implementations of io::Write and io::Read.

# Example

## Send and receive a message with `mio::Poll`

```rust
extern crate mio;
extern crate zmq;
extern crate zmq_mio;

use std::io;
use mio::{Events, Poll, PollOpt, Ready, Token};
use zmq_mio::{Context, Socket};

// We use ØMQ's `inproc://` scheme for intelligent and ready-to-use
// inter-process communications (IPC).
const EXAMPLE_ADDR: &str = "inproc://example_addr";
const LISTENER: Token = Token(0);
const SENDER: Token = Token(1);

// An example of a typical ZMQ-flow, using asynchronous mode.
fn main() {
    // Create the context.
    let context = Context::new();
    // Use the context to generate sockets.
    let listener = context.socket(zmq::PAIR).unwrap();
    let sender = context.socket(zmq::PAIR).unwrap();

    // Bind and connect our sockets.
    let _ = listener.bind(EXAMPLE_ADDR).unwrap();
    let _ = sender.connect(EXAMPLE_ADDR).unwrap();

    // Now, for the asynchronous stuff...
    // First, we setup a `mio::Poll` instance.
    let poll = Poll::new().unwrap();

    // Then we register our sockets for polling the events that
    // interest us.
    poll.register(&listener, LISTENER, Ready::readable(),
                PollOpt::edge()).unwrap();
    poll.register(&sender, SENDER, Ready::writable(),
                PollOpt::edge()).unwrap();

    // We setup a loop which will poll our sockets at every turn,
    // handling the events just the way we want them to be handled.
    let mut events = Events::with_capacity(1024);

    // We also setup some variables to control the main loop flow.
    let mut msg_sent = false;
    let mut msg_received = false;

    // will loop until the listener gets a message.
    while !msg_received {
        // Poll for our registered events.
        poll.poll(&mut events, None).unwrap();

        // Handle each event accordingly...
        for event in &events {
            match event.token() {
                SENDER => {
                    // if the sender is writable and the message hasn't
                    // been sent, then we try to send it. If sending
                    // is not possible because the socket would block,
                    // then we just continue with handling polled events.
                    if event.readiness().is_writable() && !msg_sent {
                        if let Err(e) = sender.send("hello", 0) {
                           if e.kind() == io::ErrorKind::WouldBlock {
                               continue;
                           }
                           panic!("trouble sending message");
                        }
                        msg_sent = true;
                    }
                }
                LISTENER => {
                    // if the listener is readable, then we try to receive
                    // it. If reading is not possible because of blocking, then
                    // we continue handling events.
                    let msg = match listener.recv_msg(0) {
                        Ok(m) => m,
                        Err(e) => {
                            if e.kind() == io::ErrorKind::WouldBlock {
                                continue;
                            }
                            panic!("trouble receiving message");
                        }
                    };
                    msg_received = true;
                }
                _ => unreachable!(),
            }
        }
    }
}
```
