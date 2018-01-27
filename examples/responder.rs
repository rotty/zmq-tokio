extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;


use futures::Stream;
use tokio_core::reactor::Core;
use zmq_tokio::zmq;
use zmq_tokio::{Context};

fn main() {
    let ctx = Context::new();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let server = ctx.socket(zmq::REP, &handle).unwrap();
    let _ = server.bind("tcp://127.0.0.1:78999").unwrap();

    let server_stream = (&server)
        .incoming()
        .and_then(|msg| {
            println!("server got: {:?}", msg.as_str());
            Ok(msg)
        })
        .and_then(|msg| {
            (&server).send(msg)
        })
        .and_then(|_| {
            println!("server replied");
            Ok(())
        })
        .for_each(|_| {
            Ok(())
        });

    let _ = core.run(server_stream).unwrap();


    ::std::process::exit(0);
}
