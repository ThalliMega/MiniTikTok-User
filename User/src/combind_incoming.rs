use std::{
    error::Error,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_core::Stream;
use tonic::transport::server::TcpIncoming;

pub struct CombinedIncoming {
    a: TcpIncoming,
    b: TcpIncoming,
}

impl Stream for CombinedIncoming {
    type Item = <TcpIncoming as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(value) = Pin::new(&mut self.a).poll_next(cx) {
            return Poll::Ready(value);
        }

        if let Poll::Ready(value) = Pin::new(&mut self.b).poll_next(cx) {
            return Poll::Ready(value);
        }

        Poll::Pending
    }
}

impl CombinedIncoming {
    pub fn new(a: SocketAddr, b: SocketAddr) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            a: TcpIncoming::new(a, false, Some(Duration::from_secs(60 * 60 * 24 * 3)))?,
            b: TcpIncoming::new(b, false, Some(Duration::from_secs(60 * 60 * 24 * 3)))?,
        })
    }
}
