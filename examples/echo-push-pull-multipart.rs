extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::Future;
use tokio_core::reactor::Core;

use zmq_tokio::{Context, PULL, PUSH};

const TEST_ADDR: &str = "inproc://test";

fn main() {
    let mut reactor = Core::new().unwrap();
    let context = Context::new();

    let recvr = context.socket(PULL, &reactor.handle()).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();

    let sendr = context.socket(PUSH, &reactor.handle()).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();

    let msgs: Vec<Vec<u8>> = vec![b"hello".to_vec(), b"goodbye".to_vec()];
    // Step 1: send a vector of byte-vectors, `Vec<Vec<u8>>`
    let send_future = sendr.send_multipart(msgs);

    // Step 2: receive the complete multi-part message
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

    // Exit our program, playing nice.
    ::std::process::exit(0);
}
