use std::io;

use zmq::Message;

use std::marker::PhantomData;

pub trait MessageCodec {
    type In;
    type Out;

    fn decode(&mut self, buf: &[u8]) -> io::Result<Self::In>;
    fn encode(&mut self, msg: Self::Out) -> io::Result<Message>;
}

pub struct SimpleCodec<T, U>(PhantomData<T>, PhantomData<U>);

impl<T, U> SimpleCodec<T, U> {
    pub fn new() -> Self {
        SimpleCodec(PhantomData, PhantomData)
    }
}

impl<T, U> MessageCodec for SimpleCodec<T, U>
    where T: From<Message>,
          U: Into<Message>
{
    type In = T;
    type Out = U;

    fn decode(&mut self, buf: &[u8]) -> io::Result<Self::In> {
        Ok(Self::In::from(Message::from_slice(buf)))
    }
    fn encode(&mut self, msg: Self::Out) -> io::Result<Message> {
        Ok(msg.into())
    }
}

pub struct NullCodec;

impl MessageCodec for NullCodec
{
    type In = Message;
    type Out = Message;

    fn decode(&mut self, buf: &[u8]) -> io::Result<Self::In> {
        Ok(Message::from_slice(buf))
    }
    fn encode(&mut self, msg: Message) -> io::Result<Message> {
        Ok(msg.into())
    }
}
