use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use thiserror::Error;

use crate::{
    data::tag::Tag,
    database::{
        self, connection::DatabaseConnection, connection::Error as ConnectionError,
        schema::tags::dsl::tags as tags_table,
    },
    tags::category::{self, tag_categories},
};

pub struct Tags {
    connection: DatabaseConnection,
}

pub fn tags(connection: DatabaseConnection) -> Tags {
    Tags { connection }
}

#[derive(Debug, Error)]
pub enum Error {
    /// The operation could not be performed because the database returned an
    /// error.
    #[error("database error {0}")]
    DatabaseError(#[from] diesel::result::Error),
    /// It was not possible to establish a connection to the database.
    #[error("connection error: {0}")]
    ConnectionError(#[from] ConnectionError),
    /// An invalid category ID was passed, e.g. <= 0
    #[error("invalid category id")]
    InvalidCategoryID,
    /// The category was not found
    #[error("category not found")]
    CategoryNotFound,
    /// Something happened while getting or working with the category.
    #[error("category error: {0}")]
    CategoryError(#[from] category::Error),
    /// The provided ID is invalid
    #[error("invalid id")]
    InvalidID,
    /// The tag was not found.
    #[error("not found")]
    NotFound,
}

impl Tags {
    /// Gets the tag with the provided id.
    ///
    /// Returns an error if the id is not valid, if no tags with the provided
    /// ID are found or if there is an error with the database.
    pub fn get(&self, id: i32) -> Result<Tag, Error> {
        if id <= 0 {
            return Err(Error::InvalidID);
        }

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::tags::dsl::id as tag_id;
        match tags_table.filter(tag_id.eq(id)).first(conn) {
            Err(err) if err == diesel::result::Error::NotFound => Err(Error::NotFound),
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(tag) => Ok(tag),
        }
    }
    /// List tags, optionally belonging to a category.
    ///
    /// Returns an error in case the category is invalid or not found, or if
    /// there were problems with the database.
    pub fn list(&self, category: Option<i32>) -> Result<Vec<Tag>, Error> {
        use database::schema::tags::dsl::{category_id, name};

        let query = match category {
            None => tags_table.into_boxed(),
            Some(cat_id) => {
                if cat_id <= 0 {
                    return Err(Error::InvalidCategoryID);
                }

                if let Err(err) =
                    tag_categories(self.connection.clone())
                        .get(cat_id)
                        .map_err(|err| match err {
                            category::Error::NotFound => Error::CategoryNotFound,
                            _ => Error::CategoryError(err),
                        })
                {
                    return Err(err);
                }

                tags_table.filter(category_id.eq(cat_id)).into_boxed()
            }
        };

        let conn = &mut self.connection.establish_connection()?;
        query
            .order(name.asc())
            .load::<Tag>(conn)
            .map_err(|err| Error::DatabaseError(err))
    }
}
