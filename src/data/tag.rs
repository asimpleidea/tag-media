use diesel::{AsChangeset, Queryable};
use serde::Serialize;

use crate::database::schema::tags;

/// This represents a tag.
#[derive(Debug, Queryable, Serialize, AsChangeset)]
#[diesel(table_name = tags)]
pub struct Tag {
    /// The ID of the tag in the database.
    pub id: i32,
    /// The name of the tag.
    pub name: String,
    /// The category of this tag.
    pub category_id: i32,
    /// The description.
    pub description: String,
}
