extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::{Future, Sink, Stream, stream};
use tokio_core::reactor::Core;

use zmq_tokio::{Context, Message, PUB, SUB};

const TEST_ADDR: &str = "inproc://test";


fn main() {
    let mut reactor = Core::new().unwrap();
    let context = Context::new();

    let recvr = context.socket(SUB, &reactor.handle()).unwrap();
    let _ = recvr.bind(TEST_ADDR).unwrap();
    let _ = recvr.set_subscribe(b"").unwrap();

    let sendr = context.socket(PUB, &reactor.handle()).unwrap();
    let _ = sendr.connect(TEST_ADDR).unwrap();


    let (_, recvr_split_stream) = recvr.framed().split();
    let (sendr_split_sink, _) = sendr.framed().split();

    let msg = Message::from_slice(b"hello there");

    // Step 1: start a stream with only one item.
    let start_stream = stream::iter_ok::<_, ()>(vec![(sendr_split_sink, recvr_split_stream, msg)]);

    // Step 2: send the message
    let send_msg = start_stream.and_then(|(sink, stream, msg)| {
            // send a message to the receiver.
            // return a future with the receiver
            let _ = sink.send(msg);
            Ok(stream)
        });

    // Step 3: read the message
    let fetch_msg = send_msg.for_each(|stream| {
            // process the first response that the
            // receiver gets.
            // Assert that it equals the message sent
            // by the sender.
            // returns `Ok(())` when the stream ends.
            let _ = stream.into_future().and_then(|(response, _)| {
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
