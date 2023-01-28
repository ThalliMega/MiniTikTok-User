use bolt_client::bolt_proto::version::{V4_2, V4_3};
use log::error;
use tokio::io::BufStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
use tonic::{Request, Status};

use crate::{
    proto::{
        user_service_server, UserInfoRequest, UserInfoRequests, UserInfoResponse, UserInfoResponses,
    },
    AsyncWrapper,
};

pub struct UserService {
    pub bolt_metadata: bolt_client::Metadata,
    pub bolt_url: String,
    pub bolt_domain: Option<String>,
}

impl user_service_server::UserService for UserService {
    fn get_info<'s, 'a>(&self, request: Request<UserInfoRequest>) -> AsyncWrapper<UserInfoResponse>
    where
        's: 'a,
        Self: 'a,
    {
        let bad_database = Err(Status::internal("Bad Database"));

        let req = request.into_inner();
        let target_id = req.user_id;
        let token = req.token;

        Box::pin(async {
            let client = match create_bolt_conn(
                &self.bolt_url,
                self.bolt_domain.as_ref(),
                self.bolt_metadata.clone(),
            )
            .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("{e}");
                    return bad_database;
                }
            };

            todo!()
        })
    }

    fn get_infos<'s, 'a>(
        &self,
        request: Request<UserInfoRequests>,
    ) -> AsyncWrapper<UserInfoResponses>
    where
        's: 'a,
        Self: 'a,
    {
        todo!()
    }
}

async fn create_bolt_conn(
    url: &str,
    domain: Option<impl AsRef<str>>,
    metadata: bolt_client::Metadata,
) -> Result<bolt_client::Client<Compat<BufStream<bolt_client::Stream>>>, String> {
    let stream = bolt_client::Stream::connect(url, domain)
        .await
        .map_err(|e| e.to_string())?;
    let stream = BufStream::new(stream).compat();

    let mut client = bolt_client::Client::new(stream, &[V4_3, V4_2, 0, 0])
        .await
        .map_err(|e| e.to_string())?;

    client.hello(metadata).await.map_err(|e| e.to_string())?;

    Ok(client)
}
