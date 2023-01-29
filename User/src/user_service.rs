use std::fmt::Display;

use bb8_bolt::bolt_proto;
use log::error;
use tonic::{transport::Channel, Request, Response, Status};

use crate::{
    proto::{
        auth_response::AuthStatusCode, auth_service_client::AuthServiceClient,
        user_info_response::UserInfoStatusCode, user_service_server, AuthRequest, AuthResponse,
        UserInfoRequest, UserInfoRequests, UserInfoResponse, UserInfoResponses,
    },
    AsyncWrapper,
};

pub struct UserService {
    pub bolt_pool: bb8_bolt::bb8::Pool<bb8_bolt::Manager>,
    pub auth_client: AuthServiceClient<Channel>,
}

impl user_service_server::UserService for UserService {
    fn get_info<'s, 'a>(&self, request: Request<UserInfoRequest>) -> AsyncWrapper<UserInfoResponse>
    where
        's: 'a,
        Self: 'a,
    {
        let req = request.into_inner();
        let target_id = req.user_id;
        let token = req.token;

        let mut auth_client = self.auth_client.clone();

        Box::pin(async move {
            let user_id = match auth_client
                .auth(Request::new(AuthRequest { token }))
                .await?
                .into_inner()
            {
                AuthResponse {
                    user_id,
                    status_code,
                } if status_code == AuthStatusCode::Success.into() => user_id,
                _ => {
                    return Ok(Response::new(UserInfoResponse {
                        status_code: UserInfoStatusCode::AuthFail.into(),
                        ..Default::default()
                    }))
                }
            };

            let mut bolt_client = map_bad_db_and_log(self.bolt_pool.get().await)?;

            map_bad_db_and_log(bolt_client.run(
                "match (target:User {id: $target_id}) with target optional match (follower:User)-[:FOLLOW]->(target) with count(follower) as follower_count, collect(follower) as followers, target optional match (me:User {id: $user_id}) where me in followers with follower_count, count(me)>0 as is_follow, target optional match (target)-[:FOLLOW]->(follow:User) with count(follow) as follow_count, follower_count, is_follow, target return target.id, follow_count, follower_count, is_follow;",
                Some([("target_id", bolt_proto::Value::Integer(target_id.into())), ("user_id", bolt_proto::Value::Integer(user_id.into()))].into_iter().collect()),
                None,
            ).await)?;

            let results = map_bad_db_and_log(
                bolt_client
                    .pull(Some([("n", 1)].into_iter().collect()))
                    .await,
            )?
            .0;
            let result = match results.get(0) {
                Some(val) => val,
                None => {
                    return Ok(Response::new(UserInfoResponse {
                        status_code: UserInfoStatusCode::TargetNotFound.into(),
                        ..Default::default()
                    }))
                }
            };

            let fields = result.fields();

            Ok(Response::new(UserInfoResponse {
                status_code: UserInfoStatusCode::Success.into(),
                username: map_bad_db_and_log(fields[0].clone().try_into())?,
                follow_count: map_bad_db_and_log(
                    map_bad_db_and_log::<i64, _>(fields[1].clone().try_into())?.try_into(),
                )?,
                follower_count: map_bad_db_and_log(
                    map_bad_db_and_log::<i64, _>(fields[2].clone().try_into())?.try_into(),
                )?,
                is_follow: map_bad_db_and_log(fields[3].clone().try_into())?,
                user_id: target_id,
            }))
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

fn map_bad_db_and_log<O, E: Display>(res: Result<O, E>) -> Result<O, Status> {
    res.map_err(|e| {
        error!("{e}");
        Status::internal("Bad Database")
    })
}
