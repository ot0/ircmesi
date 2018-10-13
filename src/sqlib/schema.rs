table! {
    member (id) {
        id -> Integer,
        name -> Text,
        pid -> Integer,
        create_time -> Timestamp,
    }
}

table! {
    party (id) {
        id -> Integer,
        title -> Text,
        open_time -> Nullable<Timestamp>,
        create_time -> Timestamp,
        valid -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    member,
    party,
);
