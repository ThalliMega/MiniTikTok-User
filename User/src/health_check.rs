use std::future::ready;

use tonic::Response;

use crate::{proto::health_checker_server, AsyncWrapper};

pub struct HealthChecker;

impl health_checker_server::HealthChecker for HealthChecker {
    fn check<'s, 'a>(&self, _: tonic::Request<()>) -> AsyncWrapper<()>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(ready(Ok(Response::new(()))))
    }
}
