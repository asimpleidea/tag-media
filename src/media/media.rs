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
    #[error("base path error: {0}")]
    BasePathsError(BasePathsError),
}

pub struct Media {
    connection: DatabaseConnection,
}

pub fn media(connection: DatabaseConnection) -> Media {
    Media { connection }
}

impl Media {
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
