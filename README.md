This is the barely budding seed of providing access to
[ZeroMQ](http://zeromq.org/) via the tokio async I/O abstraction.

Status
------

This project is in its very infancy. Do not expect to be able to build
something useful on top of this (yet). The API will certainly change
wildly before approaching some kind of stability.

Currently this repo provides a rough proof-of-concept implementation
of a client-server (`ZMQ_REQ`/`ZMQ_REP`) interaction in
`examples/req-rep-single-threaded.rs`. The underlying library API is
sketched just as far as needed to meet the needs of this example.

