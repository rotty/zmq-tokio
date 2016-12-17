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

#[macro_use]
extern crate tokio_core;
extern crate zmq;
extern crate zmq_tokio;

use std::io;

use futures::{Future, Poll, Sink, Stream};
use zmq_tokio::{Context,Socket};
use tokio_core::reactor::Core;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

#[test]
fn inproc_req_rep() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let ctx = Context::new();
    let mut rep = t!(ctx.socket(zmq::REP, &handle));
    let mut req = t!(ctx.socket(zmq::REQ, &handle));

    t!(rep.bind("inproc://requests"));
    t!(req.connect("inproc://requests"));

    let recv_request = RecvMessage { socket: &rep };
    let send_reply = SendMessage { socket: &rep };
    let send_request = SendMessage { socket: &req };
    let recv_reply = RecvMessage { socket: &req };

    let client = send_request.and_then(|_| recv_reply);
    let server = recv_request.and_then(|_| send_reply);

    l.run(client.join(server)).unwrap();
}

#[test]
fn ipc_stream_req_rep() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let ctx = Context::new();
    let mut rep = t!(ctx.socket(zmq::REP, &handle));
    let mut req = t!(ctx.socket(zmq::REQ, &handle));
    t!(rep.bind("ipc:///run/user/1000/zmq.test"));
    t!(req.connect("ipc:///run/user/1000/zmq.test"));

    let server = {
        let (requests, responses) = rep.split();
        requests.fold(responses, move |responses, mut request| {
            println!("request: {:?}", request);
            // FIXME: multipart send support missing, this is a crude hack
            let mut part0 = None;
            for part in request.drain(0..1) {
                part0 = Some(part);
                break;
            }
            responses.send(part0.unwrap()) // assumes zero-part messages are impossible
        })
    };
    let client = {
        let (responses, requests): (zmq_tokio::SocketStream, zmq_tokio::SocketSink) = req.split();
        let request: futures::sink::Send<zmq_tokio::SocketSink> = requests.send("Hello".into());
        request.map(move |_| {
            println!("request sent!");
            responses.into_future().map(|(reply, _)| -> futures::future::Ok<(), io::Error> {
                println!("reply: {:?}", reply);
                futures::finished(())
            })
        })
    };
    l.run(client.join(server)).unwrap();
}

struct SendMessage<'a> {
    socket: &'a Socket,
}

impl<'a> Future for SendMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        let sock = self.socket;
        try_nb!(sock.send("1234"));
        Ok(().into())
    }
}

struct RecvMessage<'a> {
    socket: &'a Socket,
}

impl<'a> Future for RecvMessage<'a> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        let msg = try_nb!(self.socket.recv());
        assert_eq!(msg.len(), 4);
        assert_eq!(&msg[..4], b"1234");
        Ok(().into())
    }
}
