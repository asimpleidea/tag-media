use diesel::{sqlite::SqliteConnection, Connection};
use std::path::Path;
use thiserror::Error;

const MAIN_DATABASE_FILE_NAME: &str = "main.db";

/// Errors that can be returned by this module.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A connection to the database could not be established.
    #[error("cannot connect to database {0}")]
    ConnectionError(#[from] diesel::ConnectionError),
    /// The provided path is invalid, e.g. it is empty.
    #[error("invalid path provided")]
    InvalidPath,
    /// The provided path is not a directory.
    #[error("provided path is not a directory")]
    NotADirectory,
    /// The provided database name is not valid.
    #[error("provided database name is not valid")]
    InvalidName,
}

/// This represents the location of the database.
pub enum DatabaseLocation<'a> {
    /// Path means that the database is in a `.db` file inside the computer.
    /// The first parameter is the path to the directory that contains the
    /// database file, and the second is the name of the file. In case the
    /// latter is `None` then the default one - `main.db` - will be used.
    Path(&'a str, Option<&'a str>),
    /// URL defines the url of the database.
    URL(&'a str),
}

/// This represents a database connection.
pub struct DatabaseConnection {
    database_location: String,
}

impl DatabaseConnection {
    /// New returns a new database connection that can be passed to all the
    /// other structures in this crate for performing operations on the
    /// database.
    ///
    /// The database connection can be established either via a file or via
    /// URL.
    ///
    /// It returns an error in case the path is invalid, is not a directory.
    pub fn new(location: DatabaseLocation) -> Result<Self, Error> {
        let database_location = match location {
            DatabaseLocation::Path(dir, name) => {
                if dir.is_empty() {
                    return Err(Error::InvalidPath);
                }

                let database_dir = Path::new(dir);
                if !database_dir.is_dir() {
                    return Err(Error::NotADirectory);
                }

                let database_name = match name {
                    None => MAIN_DATABASE_FILE_NAME,
                    Some(val) => {
                        if val.is_empty() {
                            return Err(Error::InvalidName);
                        } else {
                            val
                        }
                    }
                };

                database_dir
                    .join(database_name)
                    .to_str()
                    .unwrap()
                    .to_owned()
            }
            // TODO: this needs validation too
            DatabaseLocation::URL(url) => url.into(),
        };

        Ok(Self { database_location })
    }

    pub(crate) fn establish_connection(&self) -> Result<SqliteConnection, Error> {
        match SqliteConnection::establish(&self.database_location) {
            Ok(conn) => Ok(conn),
            Err(err) => Err(err.into()),
        }
    }
}
