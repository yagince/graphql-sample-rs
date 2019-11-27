use std::convert::From;
use std::sync::Arc;

use actix_web::{web, Error, HttpResponse};
use futures01::future::Future;

use juniper::http::playground::playground_source;
use juniper::{http::GraphQLRequest, Executor, FieldResult};
use juniper_eager_loading::{prelude::*, EagerLoading, HasMany, HasManyThrough};
use juniper_from_schema::graphql_schema_from_file;

use diesel::prelude::*;

use itertools::Itertools;

use crate::{models, DbCon, DbPool};

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
        trail: &QueryTrail<'_, User, Walked>,
    ) -> FieldResult<Vec<User>> {
        use crate::schema::users;

        let model_users = users::table
            .load::<models::User>(&executor.context().db_con)
            .and_then(|users| Ok(users.into_iter().map_into().collect_vec()))?;

        let mut users = User::from_db_models(&model_users);
        User::eager_load_all_children_for_each(
            &mut users,
            &model_users,
            executor.context(),
            trail,
        )?;

        Ok(users)
    }
}

impl MutationFields for Mutation {
    fn field_create_user(
        &self,
        executor: &Executor<'_, Context>,
        trail: &QueryTrail<'_, User, Walked>,
        name: String,
        tags: Vec<String>,
        companies: Vec<String>,
    ) -> FieldResult<User> {
        use crate::schema::{companies, employments, tags, users};

        let new_user = models::NewUser { name: name };

        let model_user = executor.context().db_con.transaction(|| {
            diesel::insert_into(users::table)
                .values(&new_user)
                .get_result::<models::User>(&executor.context().db_con)
                .and_then(|user| {
                    let values = tags
                        .into_iter()
                        .map(|tag| (tags::user_id.eq(&user.id), tags::name.eq(tag)))
                        .collect_vec();

                    diesel::insert_into(tags::table)
                        .values(&values)
                        .execute(&executor.context().db_con)?;

                    companies
                        .into_iter()
                        .map(|company_name| {
                            let company = companies::table
                                .filter(companies::name.eq(&company_name))
                                .first::<models::Company>(&executor.context().db_con)
                                .optional()?;

                            let company = match company {
                                Some(x) => x,
                                _ => diesel::insert_into(companies::table)
                                    .values(companies::name.eq(&company_name))
                                    .get_result::<models::Company>(&executor.context().db_con)?,
                            };

                            diesel::insert_into(employments::table)
                                .values((
                                    employments::user_id.eq(&user.id),
                                    employments::company_id.eq(&company.id),
                                ))
                                .execute(&executor.context().db_con)?;

                            Ok(company)
                        })
                        .collect::<Result<Vec<_>, diesel::result::Error>>()?;
                    Ok(user)
                })
        })?;

        let user = User::new_from_model(&model_user);
        User::eager_load_all_children(user, &[model_user], &executor.context(), trail)
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, EagerLoading)]
#[eager_loading(context = Context, error = diesel::result::Error)]
pub struct User {
    user: models::User,

    #[has_many(root_model_field = tag)]
    tags: HasMany<Tag>,

    #[has_many_through(join_model = models::Employment)]
    companies: HasManyThrough<Company>,
}

impl UserFields for User {
    fn field_id(&self, _: &Executor<'_, Context>) -> FieldResult<juniper::ID> {
        Ok(juniper::ID::new(self.user.id.to_string()))
    }

    fn field_name(&self, _: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.user.name)
    }

    fn field_tags(
        &self,
        _: &Executor<'_, Context>,
        _: &QueryTrail<'_, Tag, Walked>,
    ) -> FieldResult<&Vec<Tag>> {
        self.tags.try_unwrap().map_err(Into::into)
    }

    fn field_companies(
        &self,
        _: &Executor<'_, Context>,
        _: &QueryTrail<'_, Company, Walked>,
    ) -> FieldResult<&Vec<Company>> {
        self.companies.try_unwrap().map_err(Into::into)
    }
}

#[derive(Debug, Clone, PartialEq, EagerLoading)]
#[eager_loading(context = Context, error = diesel::result::Error)]
pub struct Tag {
    tag: models::Tag,
}

impl TagFields for Tag {
    fn field_id(&self, _: &Executor<'_, Context>) -> FieldResult<juniper::ID> {
        Ok(juniper::ID::new(self.tag.id.to_string()))
    }

    fn field_user_id(&self, _: &Executor<'_, Context>) -> FieldResult<juniper::ID> {
        Ok(juniper::ID::new(self.tag.user_id.to_string()))
    }

    fn field_name(&self, _: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.tag.name)
    }
}

#[derive(Debug, Clone, PartialEq, EagerLoading)]
#[eager_loading(context = Context, error = diesel::result::Error)]
pub struct Company {
    company: models::Company,
}

impl CompanyFields for Company {
    fn field_id(&self, _: &Executor<'_, Context>) -> FieldResult<juniper::ID> {
        Ok(juniper::ID::new(self.company.id.to_string()))
    }

    fn field_name(&self, _: &Executor<'_, Context>) -> FieldResult<&String> {
        Ok(&self.company.name)
    }
}

impl juniper_eager_loading::LoadFrom<models::User> for models::Tag {
    type Error = diesel::result::Error;
    type Context = Context;

    fn load(
        users: &[models::User],
        _field_args: &(),
        context: &Self::Context,
    ) -> Result<Vec<models::Tag>, Self::Error> {
        use crate::schema::tags;
        tags::table
            .filter(tags::user_id.eq_any(users.iter().map(|x| x.id).collect_vec()))
            .load::<models::Tag>(&context.db_con)
    }
}

impl juniper_eager_loading::LoadFrom<models::Employment> for models::Company {
    type Error = diesel::result::Error;
    type Context = Context;

    fn load(
        employments: &[models::Employment],
        _field_args: &(),
        context: &Self::Context,
    ) -> Result<Vec<models::Company>, Self::Error> {
        use crate::schema::companies;
        companies::table
            .filter(companies::id.eq_any(employments.iter().map(|x| x.company_id).collect_vec()))
            .load::<models::Company>(&context.db_con)
    }
}

impl juniper_eager_loading::LoadFrom<models::User> for models::Employment {
    type Error = diesel::result::Error;
    type Context = Context;

    fn load(
        users: &[models::User],
        _field_args: &(),
        context: &Self::Context,
    ) -> Result<Vec<models::Employment>, Self::Error> {
        use crate::schema::employments;
        employments::table
            .filter(employments::user_id.eq_any(users.iter().map(|x| x.id).collect_vec()))
            .load::<models::Employment>(&context.db_con)
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
