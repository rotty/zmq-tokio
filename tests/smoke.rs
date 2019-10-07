// Modeled after tests/udp.rs from tokio-core.
//
// Note that this example is somewhat ridiculously using zmq to
// exchange messages between parts of the program which run in a
// single thread. It is not totally clear from the docs that this
// should work, but it seems to.
//
// Let's hope this is working due to zmq's inherent architecture, and
// not by chance. The intuition of why this should work reliably is
// that assuming there is buffer space for at least one message in an
// zmq socket, an initial send() can be done, and everything else
// chains upon that event.

extern crate futures;
#[macro_use] extern crate log;
extern crate env_logger;

#[macro_use]
extern crate tokio_core;
extern crate zmq;
extern crate zmq_tokio;

use std::io;

use futures::{stream, Future, Sink, Stream};
use futures::future::BoxFuture;
use zmq_tokio::{Context, Socket};
use zmq_tokio::codec::NullCodec;
use tokio_core::reactor::Core;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

const SOCKET_ADDRESS: &'static str = "tcp://127.0.0.1:3294";

fn stream_server(rep: Socket, count: u64) -> BoxFuture<(), io::Error> {
    trace!("server started");
    let (responses, requests) = rep.framed(NullCodec).split();
    requests.take(count).fold(responses, |responses, mut request| {
        // FIXME: multipart send support missing, this is a crude hack
        let mut part0 = None;
        for part in request.drain(0..1) {
            part0 = Some(part);
            break;
        }
        trace!("request: {:?}", part0);
        responses.send(part0.unwrap())
    }).map(|_| {}).boxed()
}

fn stream_client(req: Socket, count: u64) -> BoxFuture<(), io::Error> {
    stream::iter((0..count).map(Ok)).fold(req.framed(NullCodec).split(), move |(requests, responses), i| {
        let content = format!("Hello {}", i);
        let request = content.as_str().into();
        requests.send(request).and_then(move |requests| {
            trace!("request sent!");
            let test_reply = responses.into_future().and_then(move |(reply, rest)| {
                trace!("reply: {:?}", reply);
                assert_eq!(reply.map(|msgs| Vec::from(&msgs[0][..])), Some(Vec::from(content.as_bytes())));
                Ok(rest)
            });
            test_reply.map(|responses| (requests, responses)).map_err(|(e, _)| e)
        })
    }).map(|_| {}).boxed()
}

#[test]
fn test_stream() {
    env_logger::init().unwrap();
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let ctx = Context::new();
    let mut rep = t!(ctx.socket(zmq::REP, &handle));
    t!(rep.bind(SOCKET_ADDRESS));

    let client = std::thread::spawn(move || {
        let mut l = Core::new().unwrap();
        let handle = l.handle();

        let ctx = Context::new();
        let mut req = t!(ctx.socket(zmq::REQ, &handle));
        t!(req.connect(SOCKET_ADDRESS));

        let client = stream_client(req, 1000);
        l.run(client).unwrap();
    });

    let server = stream_server(rep, 1000);
    l.run(server).unwrap();
    client.join().unwrap();
}
