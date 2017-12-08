// Modeled after tests/echo.rs from mio-uds.

extern crate futures;
extern crate log;
extern crate env_logger;

extern crate mio;
extern crate zmq;
extern crate zmq_mio;

use std::io;
use std::io::ErrorKind::WouldBlock;

use zmq::Message;
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::unix::UnixReady;
use zmq_mio::{Context, Socket};

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
    })
}

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

const SOCKET_ADDRESS: &'static str = "tcp://127.0.0.1:3294";

struct EchoServer {
    sock: Socket,
    msg: Option<Message>,
    interest: Ready,
}

impl EchoServer {
    fn new(sock: Socket) -> EchoServer {
        EchoServer {
            sock: sock,
            msg: None,
            interest: Ready::readable() | UnixReady::hup(),
        }
    }

    fn writable(&mut self, poll: &Poll) -> io::Result<()> {
        match self.sock.send(self.msg.take().unwrap(), 0) {
            Ok(_) => {
                self.interest.insert(Ready::readable());
                self.interest.remove(Ready::writable());
            }
            Err(ref e) if e.kind() == WouldBlock => {
                self.interest.insert(Ready::writable());
            }
            Err(e) => panic!("not implemented; client err={:?}", e),
        }

        assert!(self.interest.is_readable() || self.interest.is_writable(),
                "actual={:?}", self.interest);
        poll.reregister(&self.sock, SERVER, self.interest,
                        PollOpt::edge() | PollOpt::oneshot())
    }

    fn readable(&mut self, poll: &Poll) -> io::Result<()> {
        match self.sock.recv(0) {
            Ok(msg) => {
                self.msg = Some(msg);

                self.interest.remove(Ready::readable());
                self.interest.insert(Ready::writable());
            }
            Err(ref e) if e.kind() == WouldBlock => {}
            Err(_e) => {
                self.interest.remove(Ready::readable());
            }
        }

        assert!(self.interest.is_readable() || self.interest.is_writable(),
                "actual={:?}", self.interest);
        poll.reregister(&self.sock, SERVER, self.interest,
                        PollOpt::edge() | PollOpt::oneshot())
    }
}

struct EchoClient {
    sock: Socket,
    msgs: Vec<&'static str>,
    tx: &'static [u8],
    rx: &'static [u8],
    token: Token,
    interest: Ready,
    active: bool,
}


// Sends a message and expects to receive the same exact message, one at a time
impl EchoClient {
    fn new(sock: Socket, tok: Token, mut msgs: Vec<&'static str>) -> EchoClient {
        let curr = msgs.remove(0);

        EchoClient {
            sock: sock,
            msgs: msgs,
            tx: curr.as_bytes(),
            rx: curr.as_bytes(),
            token: tok,
            interest: Ready::empty(),
            active: true,
        }
    }

    fn readable(&mut self, poll: &Poll) -> io::Result<()> {
        match self.sock.recv(0) {
            Ok(msg) => {
                let n = msg.len();
                assert_eq!(&self.rx[..n], &msg[..n]);
                self.rx = &self.rx[n..];

                self.interest.remove(Ready::readable());

                if self.rx.len() == 0 {
                    self.next_msg(poll).unwrap();
                }
            }
            Err(ref e) if e.kind() == WouldBlock => {}
            Err(e) => panic!("error {}", e),
        }

        if !self.interest.is_empty() {
            assert!(self.interest.is_readable() || self.interest.is_writable(),
                    "actual={:?}", self.interest);
            try!(poll.reregister(&self.sock, self.token, self.interest,
                                 PollOpt::edge() | PollOpt::oneshot()));
        }

        Ok(())
    }

    fn writable(&mut self, poll: &Poll) -> io::Result<()> {
        match self.sock.send(self.tx, 0) {
            Ok(_) => {
                self.tx = &self.tx[self.tx.len()..];
                self.interest.insert(Ready::readable());
                self.interest.remove(Ready::writable());
            }
            Err(ref e) if e.kind() == WouldBlock => {
                self.interest.insert(Ready::writable());
            }
            Err(e) => panic!("not implemented; client err={:?}", e)
        }

        assert!(self.interest.is_readable() || self.interest.is_writable(),
                "actual={:?}", self.interest);
        poll.reregister(&self.sock, self.token, self.interest,
                        PollOpt::edge() | PollOpt::oneshot())
    }

    fn next_msg(&mut self, poll: &Poll) -> io::Result<()> {
        if self.msgs.is_empty() {
            self.active = false;
            return Ok(());
        }

        let curr = self.msgs.remove(0);

        self.tx = curr.as_bytes();
        self.rx = curr.as_bytes();

        self.interest.insert(Ready::writable());
        assert!(self.interest.is_readable() || self.interest.is_writable(),
                "actual={:?}", self.interest);
        poll.reregister(&self.sock, self.token, self.interest,
                        PollOpt::edge() | PollOpt::oneshot())
    }
}

struct Echo {
    server: EchoServer,
    client: EchoClient,
}

impl Echo {
    fn new(srv: Socket, client: Socket, msgs: Vec<&'static str>) -> Echo {
        Echo {
            server: EchoServer::new(srv),
            client: EchoClient::new(client, CLIENT, msgs)
        }
    }

    fn ready(&mut self,
             poll: &Poll,
             token: Token,
             events: Ready) {
        println!("ready {:?} {:?}", token, events);
        if events.is_readable() {
            match token {
                SERVER => self.server.readable(poll).unwrap(),
                CLIENT => self.client.readable(poll).unwrap(),
                _ => panic!()
            }
        }

        if events.is_writable() {
            match token {
                SERVER => self.server.writable(poll).unwrap(),
                CLIENT => self.client.writable(poll).unwrap(),
                _ => panic!()
            }
        }
    }
}

#[test]
fn echo_server() {
    let ctx = Context::new();
    let addr = SOCKET_ADDRESS;

    let poll = t!(Poll::new());
    let mut events = Events::with_capacity(1024);

    let mut srv = t!(ctx.socket(zmq::REP));
    t!(srv.bind(&addr));
    t!(poll.register(&srv,
                     SERVER,
                     Ready::readable(),
                     PollOpt::edge() | PollOpt::oneshot()));

    let mut sock = t!(ctx.socket(zmq::REQ));
    t!(sock.connect(&addr));
    t!(poll.register(&sock,
                     CLIENT,
                     Ready::writable(),
                     PollOpt::edge() | PollOpt::oneshot()));

    let mut echo = Echo::new(srv, sock, vec!["foo", "bar"]);
    while echo.client.active {
        t!(poll.poll(&mut events, None));

        for i in 0..events.len() {
            let event = events.get(i).unwrap();
            echo.ready(&poll, event.token(), event.readiness());
        }
    }
}
