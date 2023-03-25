use diesel::Queryable;
use serde::Serialize;

/// A base path representation.
#[derive(Debug, Queryable, Serialize)]
pub struct BasePath {
    /// ID of the base path.
    pub id: i32,
    /// Actual base path.
    pub base_path: String,
    // Description for this base path.
    pub description: String,
}
