use std::fmt::Display;

use bb8_bolt::bolt_proto;
use log::error;
use tonic::{Request, Response, Status};

use crate::{
    proto::{
        user_info_response::UserInfoStatusCode, user_service_server, UserInfoRequest,
        UserInfoRequests, UserInfoResponse, UserInfoResponses,
    },
    AsyncWrapper,
};

pub struct UserService {
    pub bolt_pool: bb8_bolt::bb8::Pool<bb8_bolt::Manager>,
}

impl user_service_server::UserService for UserService {
    fn get_info<'s, 'a>(&self, request: Request<UserInfoRequest>) -> AsyncWrapper<UserInfoResponse>
    where
        's: 'a,
        Self: 'a,
    {
        let req = request.into_inner();
        let target_id = req.target_id;
        let self_id = req.self_id;

        Box::pin(async move {
            let mut bolt_client = map_bad_db_and_log(self.bolt_pool.get().await)?;

            map_bad_db_and_log(bolt_client.run(
                "match (target:User {id: $target_id}) with target optional match (follower:User)-[:FOLLOW]->(target) with count(follower) as follower_count, collect(follower) as followers, target optional match (me:User {id: $user_id}) where me in followers with follower_count, count(me)>0 as is_follow, target optional match (target)-[:FOLLOW]->(follow:User) return target.username, count(follow) as follow_count, follower_count, is_follow;",
                Some([("target_id", bolt_proto::Value::Integer(target_id.into())), ("user_id", bolt_proto::Value::Integer(self_id.into()))].into_iter().collect()),
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
        let req = request.into_inner();
        let target_ids = req
            .target_ids
            .into_iter()
            .map(|v| bolt_proto::Value::Integer(v.into()))
            .collect();
        let self_id = req.self_id;

        Box::pin(async move {
            let mut bolt_client = map_bad_db_and_log(self.bolt_pool.get().await)?;

            map_bad_db_and_log(bolt_client.run(
                "match (target:User) where target.id in $target_ids with target optional match (follower:User)-[:FOLLOW]->(target) with count(follower) as follower_count, collect(follower) as followers, target optional match (me:User {id: $user_id}) where me in followers with follower_count, count(me)>0 as is_follow, target optional match (target)-[:FOLLOW]->(follow:User) return target.id, target.username, count(follow) as follow_count, follower_count, is_follow;",
                Some([("target_ids", bolt_proto::Value::List(target_ids)), ("user_id", bolt_proto::Value::Integer(self_id.into()))].into_iter().collect()),
                None,
            ).await)?;

            let results = map_bad_db_and_log(
                bolt_client
                    .pull(Some([("n", 1)].into_iter().collect()))
                    .await,
            )?
            .0;

            let mut res_vec = Vec::with_capacity(results.len());
            for record in results {
                let fields = record.fields();
                res_vec.push(UserInfoResponse {
                    status_code: UserInfoStatusCode::Success.into(),
                    username: map_bad_db_and_log(fields[1].clone().try_into())?,
                    follow_count: map_bad_db_and_log(
                        map_bad_db_and_log::<i64, _>(fields[2].clone().try_into())?.try_into(),
                    )?,
                    follower_count: map_bad_db_and_log(
                        map_bad_db_and_log::<i64, _>(fields[3].clone().try_into())?.try_into(),
                    )?,
                    is_follow: map_bad_db_and_log(fields[4].clone().try_into())?,
                    user_id: map_bad_db_and_log(
                        map_bad_db_and_log::<i64, _>(fields[0].clone().try_into())?.try_into(),
                    )?,
                });
            }

            Ok(Response::new(UserInfoResponses { responses: res_vec }))
        })
    }
}

fn map_bad_db_and_log<O, E: Display>(res: Result<O, E>) -> Result<O, Status> {
    res.map_err(|e| {
        error!("{e}");
        Status::internal("Bad Database")
    })
}
