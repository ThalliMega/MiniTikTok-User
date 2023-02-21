use std::fmt::Display;

use bb8_bolt::bb8::PooledConnection;
use bb8_bolt::{bolt_client, bolt_proto};
use log::{error, warn};
use tonic::{Request, Response, Status};

use crate::proto::{
    id_with_int64_values::IdWithInt64Value, id_with_string_values::IdWithStringValue, *,
};

use crate::AsyncWrapper;

pub struct UserService {
    pub bolt_pool: bb8_bolt::bb8::Pool<bb8_bolt::Manager>,
}

impl user_service_server::UserService for UserService {
    fn get_infos<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<UserInfos>
    where
        's: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let mut conn = map_bad_db_and_log(self.bolt_pool.get().await)?;

            let records = get_records(
                &mut conn,
                "MATCH (u:User) WHERE u.id in $ids RETURN u.id, u.username, u.avatar, u.background_image, u.signature;",
                request,
            ).await?;

            Ok(Response::new(UserInfos {
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
            }))
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
        get_by_query(
            self,
            "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:FAVORITE]->(w:Work) RETURN u.id, count(w);",
            request,
        )
    }

    fn get_follow_counts<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(
            self,
            "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:FOLLOW]->(other:User) RETURN u.id, count(other);",
            request,
        )
    }

    fn get_follower_counts<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(
            self,
            "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)<-[:FOLLOW]-(other:User) RETURN u.id, count(other);",
            request,
        )
    }

    fn get_work_counts<'s, 'a>(&self, request: Request<UserIds>) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(
            self,
            "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:CREATE]->(w:Work) RETURN u.id, count(w);",
            request,
        )
    }

    fn get_total_favoriteds<'s, 'a>(
        &self,
        request: Request<UserIds>,
    ) -> AsyncWrapper<IdWithInt64Values>
    where
        's: 'a,
        Self: 'a,
    {
        get_by_query(
            self,
            "MATCH (u:User) WHERE u.id IN ids OPTIONAL MATCH (u)-[:CREATE]->(:Work)<-[:FAVORITE]-(other:User) RETURN u.id, count(other);",
            request,
        )
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
            transform_result(
                conn.run(
                    "MATCH (:User {id: $id})-[:FOLLOW]->(o:User) WHERE o.id IN $ids RETURN o.id;",
                    Some(
                        [
                            ("id", bolt_proto::Value::Integer(req.self_id)),
                            ("ids", req.target_ids.into()),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                    None,
                )
                .await,
                &mut conn,
            )
            .await?;

            let records = transform_records(
                conn.pull(Some([("n", -1)].into_iter().collect())).await,
                &mut conn,
            )
            .await?;

            Ok(Response::new(FollowCheckResponses {
                target_ids: records
                    .into_iter()
                    // TODO: panic
                    .map(|r| r.fields()[0].clone().try_into().unwrap())
                    .collect(),
            }))
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
    ids: Request<UserIds>,
) -> Result<Vec<bolt_proto::message::Record>, Status> {
    let ids = ids.into_inner().user_ids;
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

        let records = get_records(&mut conn, query, ids).await?;

        Ok(Response::new(IdWithInt64Values {
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
        }))
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
            ids,
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
            return Err(Status::internal("Bad Database"));
        }
        Err(e) => {
            error!("{e}");
            return Err(Status::internal("Bad Database"));
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
            return Err(Status::internal("Bad Database"));
        }
        Err(e) => {
            error!("{e}");
            return Err(Status::internal("Bad Database"));
        }
    }
}
