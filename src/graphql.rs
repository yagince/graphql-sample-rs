use std::convert::From;
use std::sync::Arc;

use actix_web::{web, Error, HttpResponse};
use futures01::future::Future;

use juniper::http::playground::playground_source;
use juniper::{http::GraphQLRequest, Executor, FieldResult};
use juniper_from_schema::graphql_schema_from_file;

use diesel::prelude::*;

use itertools::Itertools;

use crate::{DbCon, DbPool};

graphql_schema_from_file!("src/schema.graphql");

pub struct Context {
    db_con: DbCon,
}
impl juniper::Context for Context {}

pub struct Query;
pub struct Mutation;

impl QueryFields for Query {
    fn field_users(
        &self,
        executor: &Executor<'_, Context>,
        _trail: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<Vec<User>> {
        use crate::schema::users;

        users::table
            .load::<crate::models::User>(&executor.context().db_con)
            .and_then(|users| Ok(users.into_iter().map_into().collect()))
            .map_err(Into::into)
    }
}

impl MutationFields for Mutation {
    fn field_create_user(
        &self,
        executor: &Executor<'_, Context>,
        _trail: &QueryTrail<'_, User, Walked>,
        name: String,
    ) -> FieldResult<User> {
        use crate::schema::users;

        let new_user = crate::models::NewUser { name: name };

        diesel::insert_into(users::table)
            .values(&new_user)
            .get_result::<crate::models::User>(&executor.context().db_con)
            .map(Into::into)
            .map_err(Into::into)
    }
}

pub struct User {
    id: i32,
    name: String,
}

impl UserFields for User {
    fn field_id(&self, _: &Executor<'_, Context>) -> FieldResult<juniper::ID> {
        Ok(juniper::ID::new(self.id.to_string()))
    }

    fn field_name(&self, _: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.name)
    }
}

impl From<crate::models::User> for User {
    fn from(user: crate::models::User) -> Self {
        Self {
            id: user.id,
            name: user.name,
        }
    }
}

fn playground() -> HttpResponse {
    let html = playground_source("");
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

fn graphql(
    schema: web::Data<Arc<Schema>>,
    data: web::Json<GraphQLRequest>,
    db_pool: web::Data<DbPool>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let ctx = Context {
        db_con: db_pool.get().unwrap(),
    };

    web::block(move || {
        let res = data.execute(&schema, &ctx);
        Ok::<_, serde_json::error::Error>(serde_json::to_string(&res)?)
    })
    .map_err(Error::from)
    .and_then(|user| {
        Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(user))
    })
}

pub fn register(config: &mut web::ServiceConfig) {
    let schema = std::sync::Arc::new(Schema::new(Query, Mutation));

    config
        .data(schema)
        .route("/", web::post().to_async(graphql))
        .route("/", web::get().to(playground));
}
