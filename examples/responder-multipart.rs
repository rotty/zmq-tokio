extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::Stream;
use tokio_core::reactor::Core;
use zmq_tokio::zmq;
use zmq_tokio::{Context};

fn main() {
    let context = Context::new();

    let ctx = context.clone();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let server = ctx.socket(zmq::REP, &handle).unwrap();
    let _ = server.bind("tcp://127.0.0.1:79888").unwrap();

    let server_stream = server
        .incoming_multipart()
        .and_then(|msgs| {
            for m in msgs {
                println!("{}", m.as_str().unwrap());
            }
            server.send_multipart(vec!["getting lonely", "getting old"])
        })
    .for_each(|_| {
        Ok(())
    });

    let _ = core.run(server_stream).unwrap();

    ::std::process::exit(0);
}
