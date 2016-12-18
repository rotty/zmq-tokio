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
use tokio_core::reactor::Core;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

const SOCKET_ADDRESS: &'static str = "tcp://127.0.0.1:3294";

// #[test]
// fn test_futures() {
//     env_logger::init().unwrap();

//     let mut l = Core::new().unwrap();
//     let handle = l.handle();

//     let ctx = Context::new();
//     let mut rep = t!(ctx.socket(zmq::REP, &handle));

//     t!(rep.bind(SOCKET_ADDRESS));

//     let client = std::thread::spawn(move || {
//         let mut l = Core::new().unwrap();
//         let handle = l.handle();
        
//         let ctx = Context::new();
//         let mut req = t!(ctx.socket(zmq::REQ, &handle));
//         t!(req.connect(SOCKET_ADDRESS));

//         let (stream, sink) = req.framed().split();
//         let send_request = SendMessage { sink: sink };
//         let recv_reply = RecvMessage { stream: stream };
//         let client = send_request.and_then(|_| recv_reply);
//         l.run(client).unwrap();
//     });
                               
//     let (stream, sink) = rep.framed().split();
//     let recv_request = RecvMessage { sink: sink };
//     let send_reply = SendMessage { stream: stream };
//     let server = recv_request.and_then(|_| send_reply);

//     l.run(server).unwrap();
//     client.join().unwrap();
// }

fn stream_server(rep: Socket, count: u64) -> BoxFuture<(), io::Error> {
    trace!("server started");
    let (responses, requests) = rep.framed().split();
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
    stream::iter((0..count).map(Ok)).fold(req.framed().split(), move |(requests, responses), i| {
        requests.send(format!("Hello {}", i).into()).and_then(move |requests| {
            trace!("request sent!");
            let show_reply = responses.into_future().and_then(move |(reply, rest)| {
                trace!("reply: {:?}", reply);
                Ok(rest)
            });
            show_reply.map(|responses| (requests, responses)).map_err(|(e, _)| e)
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

// struct SendMessage<'a> {
//     socket: &'a mut Socket,
// }

// impl<'a> Future for SendMessage<'a> {
//     type Item = ();
//     type Error = io::Error;

//     fn poll(&mut self) -> Poll<(), io::Error> {
//         trace!("SendMessage");
//         try_nb!(self.socket.send("1234"));
//         trace!("SendMessage - done");
//         Ok(().into())
//     }
// }

// struct RecvMessage<'a> {
//     socket: &'a Socket,
// }

// impl<'a> Future for RecvMessage<'a> {
//     type Item = ();
//     type Error = io::Error;

//     fn poll(&mut self) -> Poll<(), io::Error> {
//         trace!("RecvMessage");
//         let msg = try_nb!(self.socket.recv());
//         trace!("RecvMessage - {:?}", msg);
//         assert_eq!(msg.len(), 4);
//         assert_eq!(&msg[..4], b"1234");
//         Ok(().into())
//     }
// }
