extern crate futures;
extern crate tokio_core;
extern crate zmq_tokio;

use futures::Future;
use tokio_core::reactor::Core;
use zmq_tokio::zmq;
use zmq_tokio::{Context};

fn main() {
    let ctx = Context::new();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = ctx.socket(zmq::REQ, &handle).unwrap();
    let _ = client.connect("tcp://127.0.0.1:79888").unwrap();

    let client_request = client
        .send_multipart(vec!["hey you", "out there in the cold"])
        .and_then(|_| {
            client.recv_multipart()
                .and_then(|msgs| {
                    for m in msgs {
                        println!("{}", m.as_str().unwrap());
                    }
                    Ok(())
                })
        })
        .and_then(|_| {
            Ok(())
        });

    let _ = core.run(client_request).unwrap();

    ::std::process::exit(0);
}
