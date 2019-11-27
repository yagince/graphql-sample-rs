use super::schema::users;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
}
