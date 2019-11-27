table! {
    companies (id) {
        id -> Int4,
        name -> Varchar,
    }
}

table! {
    employments (id) {
        id -> Int4,
        user_id -> Int4,
        company_id -> Int4,
    }
}

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

joinable!(employments -> companies (company_id));
joinable!(employments -> users (user_id));
joinable!(tags -> users (user_id));

allow_tables_to_appear_in_same_query!(
    companies,
    employments,
    tags,
    users,
);
