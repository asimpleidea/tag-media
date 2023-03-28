use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use thiserror::Error;

use crate::{
    data::tag_category::Category,
    database::{self, connection::DatabaseConnection},
};

pub struct TagCategories {
    connection: DatabaseConnection,
}

pub fn tag_categories(connection: DatabaseConnection) -> TagCategories {
    TagCategories { connection }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The operation could not be performed because the database returned an
    /// error.
    #[error("database error {0}")]
    DatabaseError(#[from] diesel::result::Error),
    /// It was not possible to establish a connection to the database.
    #[error("connection error: {0}")]
    ConnectionError(#[from] database::connection::Error),
    /// The provided ID is not valid, e.g. it is <= 0.
    #[error("invalid id provided")]
    InvalidID,
    /// The category was not found on the database.
    #[error("not found")]
    NotFound,
}

impl TagCategories {
    /// Get a single category by ID.
    ///
    /// It returns an error if the ID is not valid, if the category with the
    /// provided ID was not found or if an error occurred while getting the
    /// category from the database.
    pub fn get(&self, id: i32) -> Result<Category, Error> {
        if id <= 0 {
            return Err(Error::InvalidID);
        }

        use database::schema::tag_categories::dsl::{id as tc_id, tag_categories as tc_table};
        let conn = &mut self.connection.establish_connection()?;
        tc_table
            .filter(tc_id.eq(id))
            .first(conn)
            .map_err(|err| match err {
                diesel::result::Error::NotFound => Error::NotFound,
                _ => Error::DatabaseError(err),
            })
    }

    /// List all tag categories that are currently being saved on the database.
    ///
    /// Optionally, you can list only some specific IDs with `ids`.
    /// In case `ids` is `None` or is `Some` but empty, then the list of *all*
    /// tag categories will be returned.
    ///
    /// It returns an error in case there are problems getting the list from
    /// the database.
    pub fn list(&self, ids: Option<impl IntoIterator<Item = i32>>) -> Result<Vec<Category>, Error> {
        // TODO: check whether the `ids` can be improved or another type
        // can be used.

        use database::schema::tag_categories::dsl::{id, tag_categories as tc_table};
        let tc_ids = match ids {
            None => vec![],
            Some(vals) => vals.into_iter().collect(),
        };

        let query = if tc_ids.len() == 0 {
            tc_table.into_boxed()
        } else {
            tc_table.filter(id.eq_any(tc_ids)).into_boxed()
        };

        let conn = &mut self.connection.establish_connection()?;
        query
            .order(id.asc())
            .load::<Category>(conn)
            .map_err(|err| Error::DatabaseError(err))
    }
}
