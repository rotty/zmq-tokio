//! Echoes a message between PAIR sockets.
extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::Future;
use tokio_core::reactor::Core;

use zmq_tokio::{Context, PAIR};

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

    // Exit our program, playing nice.
    ::std::process::exit(0);
}
