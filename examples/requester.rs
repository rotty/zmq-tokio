extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::{Future, Sink, Stream, stream};
use tokio_core::reactor::Core;
use zmq_tokio::zmq;
use zmq_tokio::{Context};

fn main() {
    let ctx = Context::new();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = ctx.socket(zmq::REQ, &handle).unwrap();
    let _ = client.connect("tcp://127.0.0.1:78999").unwrap();

    let msgs = vec!["hey you", "out there in the cold"];
    let client_requests = stream::iter_ok::<_, zmq_tokio::Error>(msgs)
        .and_then(|msg| {
            client.outgoing().send(msg.into())
        })
        .and_then(|_| {
            client.recv()
        })
        .and_then(|msg| {
           println!("client got: {}", msg.as_str().unwrap());
            Ok(())
        })
        .for_each(|_| {
            Ok(())
        });

    let _ = core.run(client_requests).unwrap();

    ::std::process::exit(0);
}
