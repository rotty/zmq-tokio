Examples
========

## Request-Response Pattern (`REQ-REP`)

### Stream

* [responder.rs](responder.rs) - `REP` server with simple-messaging.

* [requester.rs](requester.rs) - `REQ` client with simple-messaging.

## Exclusive-Pair Pattern (`PAIR`)

### Future

* [echo-pair.rs](echo-pair.rs) - Sending and receiving simple messages with futures.

  A PAIR of sockets is created. The `sender` socket sends
  a message, and the `receiver` gets it.

* [echo-push-pull-multipart](echo-push-pull-multipart.rs) - Sending and receiving multi-part messages with futures.

  This time we use `PUSH` and `PULL` sockets to move multi-part messages.

  Remember that ZMQ will either send all parts or none at all.
  Save goes for receiving.

## Pipeline Pattern (`PUSH-PULL`)

### Future

* [echo-push-pull-multipart](echo-push-pull-multipart.rs) - Sending and receiving multi-part messages with futures.

## Publish/Subscribe (`PUB-SUB`)

### Transport

* [echo-pub-sub](echo-pub-sub.rs) - Manual use of tokio tranports with `Sink` and `Stream`

  This time, we use `PUB`-`SUB` sockets to send and receive a message.
