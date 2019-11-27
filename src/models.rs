use super::schema::users;

#[derive(Queryable, Clone, PartialEq, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
}

#[derive(Queryable, Clone, PartialEq, Debug)]
pub struct Tag {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
}
