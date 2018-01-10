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

extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;

extern crate tokio_core;
extern crate zmq;
extern crate zmq_tokio;

use std::io;

use futures::{stream, Future, Sink, Stream};
use zmq_tokio::{Context, Socket};
use tokio_core::reactor::Core;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

const SOCKET_ADDRESS: &'static str = "tcp://127.0.0.1:3294";

fn stream_server(
    rep: Socket,
    count: u64,
) -> Box<futures::Future<Item = (), Error = io::Error> + std::marker::Send + 'static> {
    trace!("server started");
    let (responses, requests) = rep.framed().split();
    Box::new(
        requests
            .take(count)
            .fold(responses, |responses, request| {
                // FIXME: multipart send support missing, this is a crude hack
                trace!("responding: {:?}", request.as_str());
                responses.send(request)
            })
            .map(|_| {}),
    )
}

fn stream_client(
    req: Socket,
    count: u64,
) -> Box<futures::Future<Item = (), Error = io::Error> + std::marker::Send + 'static> {
    Box::new(
        stream::iter_result((0..count).map(Ok))
            .fold(req.framed().split(), move |(requests, responses), i| {
                let msg_str = format!("Hello {}", i);
                let msg = zmq::Message::from_slice(msg_str.as_bytes());
                trace!("requesting {:?}", msg.as_str());
                requests.send(msg).and_then(move |requests| {
                    trace!("request sent!");
                    let show_reply = responses.into_future().and_then(move |(reply, rest)| {
                        let msg = reply.unwrap();
                        let m_str = msg.as_str().unwrap();
                        // test that we get our message echoed back to us.
                        assert_eq!(m_str.to_string(), msg_str);
                        Ok(rest)
                    });
                    show_reply
                        .map(|responses| (requests, responses))
                        .map_err(|(e, _)| e)
                })
            })
            .map(|_| {}),
    )
}

#[test]
fn test_stream() {
    env_logger::init().unwrap();
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let ctx = Context::new();
    let rep = t!(ctx.socket(zmq::REP, &handle));
    t!(rep.bind(SOCKET_ADDRESS));

    let client = std::thread::spawn(move || {
        let mut l = Core::new().unwrap();
        let handle = l.handle();

        let ctx = Context::new();
        let req = t!(ctx.socket(zmq::REQ, &handle));
        t!(req.connect(SOCKET_ADDRESS));

        let client = stream_client(req, 1000);
        let _ = l.run(client).unwrap();
    });

    let server = stream_server(rep, 1000);
    let _ = l.run(server).unwrap();
    let _ = client.join().unwrap();
}
