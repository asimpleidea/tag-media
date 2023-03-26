use crate::{
    data::{self, base_path::BasePath},
    database::{self, connection::DatabaseConnection},
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use thiserror::Error;

/// BasePaths contains code and data that performs operations on base paths
/// on the database.
pub struct BasePaths {
    connection: DatabaseConnection,
}

/// This returns a new instance of the `BasePaths` struct that can be used to
/// perform operations on base paths on the database.
pub fn base_paths(connection: DatabaseConnection) -> BasePaths {
    BasePaths { connection }
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
    /// The provided base path ID was not found, e.g. was <= 0.
    #[error("invalid id")]
    InvalidID,
    /// The base path could not be found.
    #[error("not found")]
    NotFound,
}

impl BasePaths {
    /// Gets a single base path by using its ID.
    ///
    /// It returns an error in case the ID is not valid, it was not found, or
    /// if there was an error on the database.
    pub fn get(&self, id: i32) -> Result<BasePath, Error> {
        if id <= 0 {
            return Err(Error::InvalidID);
        }
        use database::schema::base_paths::dsl::{base_paths as bp_table, id as bp_id};

        let conn = &mut self.connection.establish_connection()?;
        bp_table
            .filter(bp_id.eq(id))
            .first(conn)
            .map_err(|err| match err {
                err if err == diesel::result::Error::NotFound => Error::NotFound,
                err => Error::DatabaseError(err),
            })
    }

    /// List all base paths that are currently being saved on the database.
    ///
    /// Optionally, you can list only some specific IDs with `ids`.
    /// In case `ids` is `None` or is `Some` but empty, then the list of *all*
    /// base paths will be returned.
    ///
    /// It returns an error in case there are problems getting the list from
    /// the database.
    pub fn list(
        &self,
        ids: Option<impl IntoIterator<Item = i32>>,
    ) -> Result<Vec<data::base_path::BasePath>, Error> {
        // TODO: check whether the `ids` can be improved or another type
        // can be used.

        use database::schema::base_paths::dsl::{base_paths as bp_table, id};

        let bp_ids = match ids {
            None => vec![],
            Some(vals) => vals.into_iter().collect(),
        };

        let query = if bp_ids.len() == 0 {
            bp_table.into_boxed()
        } else {
            bp_table.filter(id.eq_any(bp_ids)).into_boxed()
        };

        let conn = &mut self.connection.establish_connection()?;
        query
            .order(id.asc())
            .load::<BasePath>(conn)
            .map_err(|err| Error::DatabaseError(err))
    }
}
