use diesel::Queryable;
use serde::Serialize;

/// This represents a tag category.
#[derive(Debug, Queryable, Serialize)]
pub struct Category {
    /// The id of this category.
    pub id: i32,
    /// The name of this category.
    pub name: String,
    /// The color to display (in hex).
    pub color: String,
    /// A description for this category.
    pub description: String,
}
