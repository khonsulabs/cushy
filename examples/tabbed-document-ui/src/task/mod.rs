use std::fmt::{Debug, Formatter};
use std::future::Future;
use futures::{future, stream, Stream};
use futures::stream::BoxStream;
use futures::StreamExt;
use crate::runtime::boxed_stream;

pub mod rt;

pub struct Task<T>(Option<BoxStream<'static, T>>);

impl<T> Task<T> {

    pub fn none() -> Self {
        Self(None)
    }

    pub fn done(value: T) -> Self
    where
        T: Send + 'static,
    {
        Self::future(future::ready(value))
    }

    pub fn map<O>(
        self,
        mut f: impl FnMut(T) -> O + Send + 'static
    ) -> Task<O>
    where
        T: Send + 'static,
        O: Send + 'static,
    {
        self.then(move |output| Task::done(f(output)))
    }

    pub fn then<O>(
        self,
        mut f: impl FnMut(T) -> Task<O> + Send + 'static,
    ) -> Task<O>
    where
        T: Send + 'static,
        O: Send + 'static,
    {
        Task(match self.0 {
            None => None,
            Some(stream) => {
                Some(boxed_stream(stream.flat_map(move |output| {
                    let result = f(output)
                        .0
                        .unwrap_or_else(|| boxed_stream(stream::empty()));
                    result
                })))
            }
        })
    }

    pub fn future(future: impl Future<Output = T> + Send + 'static) -> Self
    where
        T: 'static,
    {
        Self::stream(stream::once(future))
    }

    pub fn stream(stream: impl Stream<Item = T> + Send + 'static) -> Self
    where
        T: 'static,
    {
        Self(Some(boxed_stream(stream)))
    }
}

pub fn into_stream<T>(task: Task<T>) -> Option<BoxStream<'static, T>> {
    task.0
}

impl<T> Debug for Task<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("Task<...>")
    }
}