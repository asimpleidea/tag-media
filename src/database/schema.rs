// @generated automatically by Diesel CLI.

diesel::table! {
    base_paths (id) {
        id -> Integer,
        base_path -> Text,
        description -> Text,
    }
}

diesel::table! {
    media (id) {
        id -> BigInt,
        relative_path -> Text,
        base_path_id -> Integer,
        width -> Nullable<SmallInt>,
        height -> Nullable<SmallInt>,
        size -> Double,
        mark -> Nullable<SmallInt>,
        description -> Text,
        media_type -> Text,
    }
}

diesel::table! {
    media_tags (id) {
        id -> BigInt,
        media_id -> BigInt,
        tag_id -> Integer,
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

diesel::joinable!(media -> base_paths (base_path_id));
diesel::joinable!(media_tags -> media (media_id));
diesel::joinable!(media_tags -> tags (tag_id));
diesel::joinable!(tags -> tag_categories (category_id));

diesel::allow_tables_to_appear_in_same_query!(base_paths, media, media_tags, tag_categories, tags,);
