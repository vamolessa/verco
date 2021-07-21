use crate::application::Context;

pub enum Poll<T> {
    Pending,
    Ok(T),
    Err(String),
}

pub trait Promise {
    type Output;
    fn poll(&mut self, ctx: &mut Context) -> Poll<Self::Output>;

    fn map<O>(
        self,
        f: fn(ctx: &mut Context, Self::Output) -> O,
    ) -> MapPromise<Self, O>
    where
        Self: Sized,
    {
        MapPromise { inner: self, f }
    }

    fn then<P>(self, other: P) -> ThenPromise<Self, P>
    where
        Self: Sized,
        P: Promise,
    {
        ThenPromise {
            first: self,
            second: other,
            first_output: None,
            second_output: None,
        }
    }
}

pub struct MapPromise<P, O>
where
    P: Promise,
{
    inner: P,
    f: fn(ctx: &mut Context, P::Output) -> O,
}
impl<P, O> Promise for MapPromise<P, O>
where
    P: Promise,
{
    type Output = O;
    fn poll(&mut self, ctx: &mut Context) -> Poll<Self::Output> {
        match self.inner.poll(ctx) {
            Poll::Pending => Poll::Pending,
            Poll::Ok(output) => Poll::Ok((self.f)(ctx, output)),
            Poll::Err(error) => Poll::Err(error),
        }
    }
}

pub struct ThenPromise<A, B>
where
    A: Promise,
    B: Promise,
{
    first: A,
    second: B,
    first_output: Option<A::Output>,
    second_output: Option<B::Output>,
}
impl<A, B> Promise for ThenPromise<A, B>
where
    A: Promise,
    B: Promise,
{
    type Output = (A::Output, B::Output);
    fn poll(&mut self, ctx: &mut Context) -> Poll<Self::Output> {
        if self.first_output.is_none() {
            match self.first.poll(ctx) {
                Poll::Pending => (),
                Poll::Ok(output) => self.first_output = Some(output),
                Poll::Err(error) => return Poll::Err(error),
            }
        }
        if self.second_output.is_none() {
            match self.second.poll(ctx) {
                Poll::Pending => (),
                Poll::Ok(output) => self.second_output = Some(output),
                Poll::Err(error) => return Poll::Err(error),
            }
        }

        if self.first_output.is_some() && self.second_output.is_some() {
            let first = self.first_output.take().unwrap();
            let second = self.second_output.take().unwrap();
            Poll::Ok((first, second))
        } else {
            Poll::Pending
        }
    }
}

