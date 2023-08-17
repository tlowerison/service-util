use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait OptionFutureTransposeExt {
    type Inner;
    type Fut: Future<Output = Option<Self::Inner>>;
    fn transpose(self) -> Self::Fut;
}

impl<T, Fut: Future<Output = T>> OptionFutureTransposeExt for Option<Fut> {
    type Inner = T;
    type Fut = OptionFutureTranspose<Fut>;
    fn transpose(self) -> Self::Fut {
        match self {
            Some(fut) => OptionFutureTranspose::Some { fut },
            None => OptionFutureTranspose::None,
        }
    }
}

pin_project! {
    #[project = OptionFutureTransposeProj]
    pub enum OptionFutureTranspose<Fut> {
        Some { #[pin] fut: Fut },
        None,
    }
}

impl<T, Fut: Future<Output = T>> Future for OptionFutureTranspose<Fut> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            OptionFutureTransposeProj::Some { fut } => match fut.poll(cx) {
                Poll::Ready(t) => Poll::Ready(Some(t)),
                Poll::Pending => Poll::Pending,
            },
            OptionFutureTransposeProj::None => Poll::Ready(None),
        }
    }
}
