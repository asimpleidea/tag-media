use std::path;

use crate::{
    data::{self, base_path::BasePath},
    database::{self, connection::DatabaseConnection, schema::base_paths},
};
use diesel::{ExpressionMethods, Insertable, QueryDsl, RunQueryDsl};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

const MAX_DESCRIPTION_LENGTH: usize = 300;

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
    /// The provided description is too long.
    #[error("description cannot be longer than 300 characters")]
    DescriptionTooLong,
    /// The path is invalid, e.g. is empty.
    #[error("invalid path")]
    InvalidPath,
    /// The provided path does not exist.
    #[error("not exists")]
    NotExists,
    /// The provided path is not a directory.
    #[error("not a directory")]
    NotADirectory,
    /// The provided path is not an absolute path.
    #[error("not an absolute path")]
    NotAbsolute,
    /// The provided path is already registered on the database.
    #[error("already exists")]
    AlreadyExists,
    /// The provided path is a sub path of an existing path.
    #[error("is sub path")]
    IsSubPath,
    /// The base path cannot be deleted because it is still referenced
    /// somewhere.
    #[error("cannot be deleted because in use")]
    InUse,
}

#[derive(Insertable)]
#[diesel(table_name = base_paths)]
pub struct NewBasePath<'a> {
    base_path: &'a str,
    description: &'a str,
}

impl BasePaths {
    pub fn create(
        &self,
        base_path: impl AsRef<str>,
        description: impl AsRef<str>,
    ) -> Result<BasePath, Error> {
        let bp = base_path.as_ref().trim().trim_end_matches('/');
        if bp.len() == 0 {
            return Err(Error::InvalidPath);
        }

        let desc = description.as_ref().trim();
        if desc.graphemes(true).count() > MAX_DESCRIPTION_LENGTH {
            return Err(Error::DescriptionTooLong);
        }

        let p = path::Path::new(bp);
        if !p.exists() {
            return Err(Error::NotExists);
        }

        if !p.is_dir() {
            return Err(Error::NotADirectory);
        }
        if !p.is_absolute() {
            return Err(Error::NotAbsolute);
        }

        let list = self.list(None::<Vec<_>>)?;
        for basepath in list {
            if basepath.base_path == bp {
                return Err(Error::AlreadyExists);
            }

            if bp.starts_with(&basepath.base_path) {
                // TODO: on future this will change all existing paths to this new
                // sub path: e.g. if `/this/that/` exists and you are adding
                // `/this/that/another`, then all media that starts with
                // `another` and belongs to `this/that/` will be changed.
                // TODO: this means that this will become a transaction.
                return Err(Error::IsSubPath);
            }
        }

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::base_paths::dsl::base_paths;
        diesel::insert_into(base_paths)
            .values(NewBasePath {
                base_path: bp,
                description: desc,
            })
            .get_result(conn)
            .map_err(|err| Error::DatabaseError(err))
    }

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

    /// Updates the description of a base path.
    ///
    /// It returns an error in case the ID is not valid, it was not found, or
    /// if there was an error on the database.
    pub fn update_description(
        &self,
        id: i32,
        new_description: impl AsRef<str>,
    ) -> Result<(), Error> {
        if let Err(err) = self.get(id) {
            return Err(err);
        }

        let new_desc = new_description.as_ref().trim();
        if new_desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err(Error::DescriptionTooLong);
        }

        use database::schema::base_paths::dsl::{base_paths, description, id as bp_id};
        match diesel::update(base_paths.filter(bp_id.eq(id)))
            .set(description.eq(new_desc))
            .execute(&mut self.connection.establish_connection()?)
        {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(_) => Ok(()),
        }
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

    /// Deletes a base path from the database.
    ///
    /// It returns an error in case the ID is not valid, it was not found, or
    /// if there was an error on the database.
    pub fn delete(&self, id: i32) -> Result<(), Error> {
        if let Err(err) = self.get(id) {
            return Err(err);
        }

        {
            use database::schema::media::dsl::{base_path_id as bp_id, media as m_table};
            let conn = &mut self.connection.establish_connection()?;

            match m_table.filter(bp_id.eq(id)).count().execute(conn) {
                Ok(vals) if vals == 0 => (),
                Ok(_) => return Err(Error::InUse),
                Err(err) => return Err(Error::DatabaseError(err)),
            }
        }

        use database::schema::base_paths::dsl::{base_paths, id as bp_id};
        match diesel::delete(base_paths.filter(bp_id.eq(id)))
            .execute(&mut self.connection.establish_connection()?)
        {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::DatabaseError(err)),
        }
    }
}
