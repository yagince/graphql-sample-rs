table! {
    tags (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Varchar,
    }
}

table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
    }
}

joinable!(tags -> users (user_id));

allow_tables_to_appear_in_same_query!(
    tags,
    users,
);
