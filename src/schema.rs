table! {
    Posts (PostID) {
        PostID -> Integer,
        AuthorID -> Integer,
        Title -> Text,
        Body -> Text,
    }
}

table! {
    Users (UserID) {
        UserID -> Integer,
        Username -> Text,
        Bio -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    Posts,
    Users,
);
