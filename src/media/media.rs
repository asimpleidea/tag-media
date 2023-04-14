use super::base_paths::Error as BasePathsError;
use crate::data::media_file::MediaFile;
use crate::database::{
    connection::{DatabaseConnection, Error as ConnectionError},
    schema::media::{self, dsl::media as media_table},
};
use crate::media::base_paths;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The operation could not be performed because the database returned an
    /// error.
    #[error("database error {0}")]
    DatabaseError(#[from] diesel::result::Error),
    /// It was not possible to establish a connection to the database.
    #[error("connection error: {0}")]
    ConnectionError(#[from] ConnectionError),
    /// Error in the base path.
    #[error("base path error: {0}")]
    BasePathsError(BasePathsError),
    /// The ID is invalid, e.g. was <= 0.
    #[error("invalid ID")]
    InvalidID,
    /// The media file was not found on database.
    #[error("not found")]
    NotFound,
    /// The relative path is invalid, e.g. it is empty.
    #[error("invalid relative path")]
    InvalidRelativePath,
    /// The base path ID is invalid, e.g. it is <= 0.
    #[error("invalid base path ID")]
    InvalidBasePathID,
}

pub struct Media {
    connection: DatabaseConnection,
}

pub fn media(connection: DatabaseConnection) -> Media {
    Media { connection }
}

impl Media {
    /// Gets a media file by using its ID.
    pub fn get(&self, id: i64) -> Result<MediaFile, Error> {
        if id <= 0 {
            return Err(Error::InvalidID);
        }

        let conn = &mut self.connection.establish_connection()?;
        use media::dsl::id as media_id;

        media_table
            .filter(media_id.eq(id))
            .first(conn)
            .map_err(|err| match err {
                diesel::NotFound => Error::NotFound,
                _ => Error::DatabaseError(err),
            })
    }

    /// Gets a media file by using the relative path and the base path id.
    pub fn get_by_relative_path(
        &self,
        base_path_id: i32,
        relative_path: impl AsRef<str>,
    ) -> Result<MediaFile, Error> {
        let rp = relative_path.as_ref().trim_matches('/');
        if rp.is_empty() {
            return Err(Error::InvalidRelativePath);
        }

        if base_path_id <= 0 {
            return Err(Error::InvalidBasePathID);
        }

        let conn = &mut self.connection.establish_connection()?;
        use media::dsl::{base_path_id as bp_id, relative_path as media_relative_path};

        media_table
            .filter(media_relative_path.eq(rp))
            .filter(bp_id.eq(base_path_id))
            .first(conn)
            .map_err(|err| match err {
                diesel::NotFound => Error::NotFound,
                _ => Error::DatabaseError(err),
            })
    }

    /// List all media file from a base path ID, if registered.
    ///
    /// Returns a list of media files or an error in case `base_path_id` is
    /// not valid or if there was an error in the database.
    pub fn list(&self, base_path_id: i32) -> Result<Vec<MediaFile>, Error> {
        if let Err(err) = base_paths::base_paths(self.connection.clone()).get(base_path_id) {
            return Err(Error::BasePathsError(err));
        }

        use media::dsl::id as media_id;
        let conn = &mut self.connection.establish_connection()?;

        match media_table.order(media_id.asc()).load::<MediaFile>(conn) {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(files) => Ok(files),
        }
    }
}
