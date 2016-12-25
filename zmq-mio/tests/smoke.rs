extern crate futures;
#[macro_use] extern crate log;
extern crate env_logger;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

const SOCKET_ADDRESS: &'static str = "tcp://127.0.0.1:3294";

#[test]
fn test_futures() {
    env_logger::init().unwrap();

    let ctx = Context::new();
    let mut rep = t!(ctx.socket(zmq::REP, &handle));

    t!(rep.bind(SOCKET_ADDRESS));

    let client = std::thread::spawn(move || {
        let mut l = Core::new().unwrap();
        let handle = l.handle();
        
        let ctx = Context::new();
        let mut req = t!(ctx.socket(zmq::REQ, &handle));
        t!(req.connect(SOCKET_ADDRESS));

        let (sink, stream) = req.framed().split();
        let send_request = SendMessage { sink: sink };
        let recv_reply = RecvMessage { stream: stream };
        let client = send_request.and_then(|_| recv_reply);
        l.run(client).unwrap();
    });

    let (sink, stream) = rep.framed().split();
    let recv_request = RecvMessage { stream: stream };
    let send_reply = SendMessage { sink: sink };
    let server = recv_request.and_then(|_| send_reply);

    l.run(server).unwrap();
    client.join().unwrap();
}

struct SendMessage {
    sink: stream::SplitSink<SocketFramed>,
}

impl Future for SendMessage {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        trace!("SendMessage");
        self.sink.send(vec!["1234".into()]);
        trace!("SendMessage - done");
        Ok(().into())
    }
}

struct RecvMessage {
    stream: stream::SplitStream<SocketFramed>,
}

impl Future for RecvMessage {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        trace!("RecvMessage");
        let msg = self.socket.recv();
        trace!("RecvMessage - {:?}", msg);
        assert_eq!(msg.len(), 4);
        assert_eq!(&msg[..4], b"1234");
        Ok(().into())
    }
}

