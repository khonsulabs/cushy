use std::future::Future;
use std::marker::PhantomData;
use futures::channel::mpsc;
use futures::Sink;
use futures::stream::BoxStream;

#[derive(Debug)]
pub struct Executor;

impl Executor {
    pub fn new() -> Result<Self, futures::io::Error> {
        Ok(Self)
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        let _ = async_std::task::spawn(future);
    }
}

pub struct RunTime<S, M> {
    executor: Executor,
    sender: S,
    _message: PhantomData<M>,
}

impl<S, M> RunTime<S, M>
where
    S: Sink<M, Error = mpsc::SendError>
    + Unpin
    + Send
    + Clone
    + 'static,
    M: Send + 'static,
{
    pub fn new(executor: Executor, sender: S) -> Self {
        Self {
            executor,
            sender,
            _message: PhantomData::default(),
        }
    }

    pub fn run(&mut self, stream: BoxStream<'static, M>) {
        use futures::{FutureExt, StreamExt};

        let message = self.sender.clone();
        let future =
            stream.map(Ok).forward(message).map(|result| match result {
                Ok(()) => (),
                Err(error) => {
                    println!("Stream unable to complete, cause: {error}");
                }
            });

        self.executor.spawn(future);
    }
}

pub fn boxed_stream<T, S>(stream: S) -> BoxStream<'static, T>
where
    S: futures::Stream<Item = T> + Send + 'static,
{
    futures::stream::StreamExt::boxed(stream)
}
