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

diesel::table! {
    tags (id) {
        id -> Integer,
        name -> Text,
        category_id -> Integer,
        description -> Text,
    }
}

diesel::joinable!(tags -> tag_categories (category_id));

diesel::allow_tables_to_appear_in_same_query!(base_paths, tag_categories, tags,);
