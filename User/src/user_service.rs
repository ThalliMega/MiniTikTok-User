use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Status, Streaming};

use crate::{
    proto::{user_service_server, UserInfoRequest, UserInfoResponse},
    AsyncWrapper,
};

pub struct UserService {
    pub postgres_config: tokio_postgres::config::Config,
}

impl user_service_server::UserService for UserService {
    fn get_info<'s, 'a>(&self, request: Request<UserInfoRequest>) -> AsyncWrapper<UserInfoResponse>
    where
        's: 'a,
        Self: 'a,
    {
        todo!()
    }

    type GetInfosStream = ReceiverStream<Result<UserInfoResponse, Status>>;

    fn get_infos<'s, 'a>(
        &self,
        request: Request<Streaming<UserInfoRequest>>,
    ) -> AsyncWrapper<Self::GetInfosStream>
    where
        's: 'a,
        Self: 'a,
    {
        todo!()
    }
}
