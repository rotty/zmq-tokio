use std::io;

use zmq::Message;

use std::marker::PhantomData;

/// Convert from and to 0MQ messages.
///
/// This trait consumes and produces messages by value, so
/// implementations can decide whether to retain ownership or drop
/// them.
pub trait MessageCodec {
    type In;
    type Out;

    fn decode(&mut self, msg: Message) -> io::Result<Self::In>;
    fn encode(&mut self, item: Self::Out) -> io::Result<Message>;
}

pub trait MultiPartCodec {
    type In;
    type Out;

    fn decode<I, T>(&mut self, buf: I) -> io::Result<Self::In>
        where I: IntoIterator<Item=T>,
              T: AsRef<[u8]> + Sized;

    fn encode<I, T>(&mut self, msg: Self::Out) -> io::Result<I>
        where I: IntoIterator<Item=T>,
              T: Into<Message>;
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

    fn decode(&mut self, msg: Message) -> io::Result<Self::In> {
        Ok(Self::In::from(msg))
    }
    fn encode(&mut self, item: Self::Out) -> io::Result<Message> {
        Ok(item.into())
    }
}

pub struct NullCodec;

impl MessageCodec for NullCodec
{
    type In = Message;
    type Out = Message;

    fn decode(&mut self, msg: Message) -> io::Result<Self::In> {
        Ok(msg)
    }
    fn encode(&mut self, msg: Message) -> io::Result<Message> {
        Ok(msg)
    }
}

// impl MultiPartCodec for NullCodec {
//     type In = Vec<Message>;
//     type Out = Vec<Message>;

//     fn decode<I, T>(&mut self, parts: I) -> io::Result<Vec<Message>>
//         where I: IntoIterator<Item=T>,
//               T: AsRef<[u8]> + Sized {
//         Ok(parts.into_iter()
//            .map(|part| Message::from_slice(part.as_ref()))
//            .collect())
//     }
//     fn encode<I, T>(&mut self, parts: Vec<Message>) -> io::Result<I>
//         where I: IntoIterator,
//               T: Into<Message> {
//         Ok(parts)
//     }
// }

#[cfg(test)]
mod test {
}
