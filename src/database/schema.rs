// @generated automatically by Diesel CLI.

diesel::table! {
    base_paths (id) {
        id -> Integer,
        base_path -> Text,
        description -> Text,
    }
}

diesel::table! {
    tag_categories (id) {
        id -> Integer,
        name -> Text,
        color -> Text,
        description -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(base_paths, tag_categories,);
