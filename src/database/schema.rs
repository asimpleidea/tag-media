// @generated automatically by Diesel CLI.

diesel::table! {
    base_paths (id) {
        id -> Integer,
        base_path -> Text,
        description -> Text,
    }
}
