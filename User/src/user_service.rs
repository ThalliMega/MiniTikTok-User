use std::collections::HashMap;
use std::fmt::Display;

use bb8_bolt::bb8::PooledConnection;
use bb8_bolt::{bolt_client, bolt_proto, Manager};
use log::{error, warn};
use tonic::{Request, Response, Status};

use crate::proto::{
    id_with_int64_values::IdWithInt64Value, id_with_string_values::IdWithStringValue, *,
};

use crate::AsyncWrapper;

const QUERIRS: [&str; 5] = [
    "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:FAVORITE]->(w:Work) RETURN u.id, count(w);", // favorite_counts
    "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:FOLLOW]->(other:User) RETURN u.id, count(other);", // follow_counts
    "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)<-[:FOLLOW]-(other:User) RETURN u.id, count(other);", // follower_counts
    "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:CREATE]->(w:Work) RETURN u.id, count(w);", // work_counts
    "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:CREATE]->(:Work)<-[:FAVORITE]-(other:User) RETURN u.id, count(other);" // total_favoriteds
];

pub struct UserService {
    pub bolt_pool: bb8_bolt::bb8::Pool<bb8_bolt::Manager>,
}

impl user_service_server::UserService for UserService {
    fn get_full_infos<'life0, 'async_trait>(
        &'life0 self,
        request: tonic::Request<FollowCheckRequests>,
    ) -> AsyncWrapper<'async_trait, FullUserInfos>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let req = request.into_inner();
        let ids = req.target_ids;
        let self_id = req.self_id;
        let request = UserIds { user_ids: ids };
        Box::pin(async move {
            let mut conn = map_bad_db_and_log(self.bolt_pool.get().await)?;
            // TODO: there are too many clones
            let infos = real_get_infos(&mut conn, request.clone()).await?;
            let favorite_counts =
                get_by_query_inner(&mut conn, QUERIRS[0], request.clone()).await?;
            let follow_counts = get_by_query_inner(&mut conn, QUERIRS[1], request.clone()).await?;
            let follower_counts =
                get_by_query_inner(&mut conn, QUERIRS[2], request.clone()).await?;
            let work_counts = get_by_query_inner(&mut conn, QUERIRS[3], request.clone()).await?;
            let total_favoriteds =
                get_by_query_inner(&mut conn, QUERIRS[4], request.clone()).await?;
            let follows = real_check_follows(&mut conn, self_id, request.user_ids).await?;

            let mut res = HashMap::with_capacity(infos.infos.len());

            for info in infos.infos {
                res.insert(
                    info.id,
                    FullUserInfo {
                        id: info.id,
                        name: info.username,
                        avatar: info.avatar,
                        background_image: info.background_img,
                        signature: info.signature,
                        ..Default::default()
                    },
                );
            }
            for count in favorite_counts.responses {
                // TODO: unwrap
                res.get_mut(&count.user_id).unwrap().favorite_count = count.value;
            }
            for count in follow_counts.responses {
                // TODO: unwrap
                res.get_mut(&count.user_id).unwrap().follow_count = count.value;
            }
            for count in follower_counts.responses {
                // TODO: unwrap
                res.get_mut(&count.user_id).unwrap().follower_count = count.value;
            }
            for count in work_counts.responses {
                // TODO: unwrap
                res.get_mut(&count.user_id).unwrap().work_count = count.value;
            }
            for count in total_favoriteds.responses {
                // TODO: unwrap
                res.get_mut(&count.user_id).unwrap().total_favorited = count.value;
            }
            for tar_id in follows.target_ids {
                // TODO: unwrap
                res.get_mut(&tar_id).unwrap().is_follow = true;
            }

            Ok(Response::new(FullUserInfos {
                infos: res.into_values().collect(),
            }))
        })
    }
    fn get_infos<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<UserInfos>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let mut conn = map_bad_db_and_log(self.bolt_pool.get().await)?;

            real_get_infos(&mut conn, request.into_inner())
                .await
                .map(Response::new)
        })
    }
    fn get_usernames<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<IdWithStringValues>
    where
        's: 'a,
        Self: 'a,
    {
        get_properties(self, "username", request)
    }

    fn get_avatars<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<IdWithStringValues>
    where
        's: 'a,
        Self: 'a,
    {
        get_properties(self, "avatar", request)
    }

    fn get_signatures<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<IdWithStringValues>
    where
        's: 'a,
        Self: 'a,
    {
        get_properties(self, "signature", request)
    }

    fn get_background_images<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithStringValues>
    where
        's: 'a,
        Self: 'a,
    {
        get_properties(self, "background_image", request)
    }

    fn get_favorite_counts<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(self, QUERIRS[0], request)
    }

    fn get_follow_counts<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(self, QUERIRS[1], request)
    }

    fn get_follower_counts<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(self, QUERIRS[2], request)
    }

    fn get_work_counts<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(self, QUERIRS[3], request)
    }

    fn get_total_favoriteds<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(self, QUERIRS[4], request)
    }

    fn check_follows<'s, 'a>(
        &self,
        request: Request<FollowCheckRequests>,
    ) -> AsyncWrapper<FollowCheckResponses>
    where
        's: 'a,
        Self: 'a,
    {
        let req = request.into_inner();
        Box::pin(async move {
            let mut conn = map_bad_db_and_log(self.bolt_pool.get().await)?;

            let records = real_check_follows(&mut conn, req.self_id, req.target_ids).await?;

            Ok(Response::new(records))
        })
    }
}

fn map_bad_db_and_log<O, E: Display>(res: Result<O, E>) -> Result<O, Status> {
    res.map_err(|e| {
        error!("{e}");
        Status::internal("Bad Database")
    })
}

async fn get_records(
    conn: &mut PooledConnection<'_, bb8_bolt::Manager>,
    query: &str,
    ids: UserIds,
) -> Result<Vec<bolt_proto::message::Record>, Status> {
    let ids = ids.user_ids;
    transform_result(
        conn.run(query, Some([("ids", ids)].into_iter().collect()), None)
            .await,
        conn,
    )
    .await?;

    transform_records(
        conn.pull(Some([("n", -1)].into_iter().collect())).await,
        conn,
    )
    .await
}

/// Return id first.
/// ids: ids
fn get_by_query<'s>(
    service: &'s UserService,
    query: &'s str,
    ids: Request<UserIds>,
) -> AsyncWrapper<'s, IdWithInt64Values> {
    Box::pin(async move {
        let mut conn = map_bad_db_and_log(service.bolt_pool.get().await)?;

        get_by_query_inner(&mut conn, query, ids.into_inner())
            .await
            .map(Response::new)
    })
}

/// Return id first.
/// ids: ids
async fn get_by_query_inner(
    conn: &mut PooledConnection<'_, Manager>,
    query: &str,
    ids: UserIds,
) -> Result<IdWithInt64Values, Status> {
    let records = get_records(conn, query, ids).await?;

    Ok(IdWithInt64Values {
        responses: records
            .into_iter()
            .map(|r| {
                let fs = r.fields();
                IdWithInt64Value {
                    // TODO: panic
                    user_id: fs[0].clone().try_into().unwrap(),
                    value: fs[1].clone().try_into().unwrap(),
                }
            })
            .collect(),
    })
}

fn get_properties<'s>(
    service: &'s UserService,
    property: &'s str,
    ids: Request<UserIds>,
) -> AsyncWrapper<'s, IdWithStringValues> {
    Box::pin(async move {
        let mut conn = map_bad_db_and_log(service.bolt_pool.get().await)?;

        let records = get_records(
            &mut conn,
            &format!("MATCH (u:User) WHERE u.id IN $ids RETURN id, {property};"),
            ids.into_inner(),
        )
        .await?;

        Ok(Response::new(IdWithStringValues {
            responses: records
                .into_iter()
                .map(|r| {
                    let fs = r.fields();
                    IdWithStringValue {
                        // TODO: panic
                        user_id: fs[0].clone().try_into().unwrap(),
                        value: fs[1].clone().try_into().unwrap(),
                    }
                })
                .collect(),
        }))
    })
}

async fn transform_result(
    r: Result<bolt_proto::Message, bolt_client::error::CommunicationError>,
    conn: &mut PooledConnection<'_, bb8_bolt::Manager>,
) -> Result<(), Status> {
    match r {
        Ok(bolt_proto::Message::Success(_)) => Ok(()),
        Ok(res) => {
            warn!("{res:?}");
            // TODO: ignore result
            _ = conn.reset().await;
            Err(Status::internal("Bad Database"))
        }
        Err(e) => {
            error!("{e}");
            Err(Status::internal("Bad Database"))
        }
    }
}

async fn transform_records(
    r: Result<
        (Vec<bolt_proto::message::Record>, bolt_proto::Message),
        bolt_client::error::CommunicationError,
    >,
    conn: &mut PooledConnection<'_, bb8_bolt::Manager>,
) -> Result<Vec<bolt_proto::message::Record>, Status> {
    match r {
        Ok((rec, bolt_proto::Message::Success(_))) => Ok(rec),
        Ok((_, res)) => {
            warn!("{res:?}");
            // TODO: ignore result
            _ = conn.reset().await;
            Err(Status::internal("Bad Database"))
        }
        Err(e) => {
            error!("{e}");
            Err(Status::internal("Bad Database"))
        }
    }
}

async fn real_get_infos(
    conn: &mut PooledConnection<'_, Manager>,
    ids: UserIds,
) -> Result<UserInfos, Status> {
    let records = get_records(
                conn,
                "MATCH (u:User) WHERE u.id in $ids RETURN u.id, u.username, u.avatar, u.background_image, u.signature;",
                ids,
            ).await?;

    Ok(UserInfos {
        infos: records
            .into_iter()
            .map(|r| {
                let fields = r.fields();
                // TODO: panic, clone
                UserInfo {
                    id: fields.get(0).unwrap().clone().try_into().unwrap(),
                    username: fields.get(1).unwrap().clone().try_into().unwrap(),
                    avatar: fields.get(2).unwrap().clone().try_into().unwrap(),
                    background_img: fields.get(3).unwrap().clone().try_into().unwrap(),
                    signature: fields.get(4).unwrap().clone().try_into().unwrap(),
                }
            })
            .collect(),
    })
}

async fn real_check_follows(
    conn: &mut PooledConnection<'_, Manager>,
    self_id: i64,
    ids: Vec<i64>,
) -> Result<FollowCheckResponses, Status> {
    transform_result(
        conn.run(
            "MATCH (:User {id: $id})-[:FOLLOW]->(o:User) WHERE o.id IN $ids RETURN o.id;",
            Some(
                [
                    ("id", bolt_proto::Value::Integer(self_id)),
                    ("ids", ids.into()),
                ]
                .into_iter()
                .collect(),
            ),
            None,
        )
        .await,
        conn,
    )
    .await?;

    let follows = transform_records(
        conn.pull(Some([("n", -1)].into_iter().collect())).await,
        conn,
    )
    .await?
    .into_iter()
    .map(|record| record.fields().get(0).unwrap().clone().try_into().unwrap()) // TODO: unwrap, Value clone
    .collect();

    Ok(FollowCheckResponses {
        target_ids: follows,
    })
}
