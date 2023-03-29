use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use raster::Color;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    data::tag_category::Category,
    database::{self, connection::DatabaseConnection},
};

const MAX_DESCRIPTION_LENGTH: usize = 300;
const MAX_NAME_LENGTH: usize = 50;

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
    /// Invalid color provided.
    #[error("invalid color")]
    InvalidColor,
    /// Invalid name provided, e.g. it is empty.
    #[error("invalid name")]
    InvalidName,
    /// Name is longer than 50 characters long.
    #[error("name too long")]
    NameTooLong,
    /// Description is longer than 300 characters long.
    #[error("description too long")]
    DescriptionTooLong,
    /// Name to search is to short.
    #[error("name to search too short")]
    NameToSearchTooShort,
}

/// Use to update the category.
pub struct UpdateTagCategory<'a> {
    /// New name to use. If `None` the existing name will be used. If `Some`
    /// the new name will be used, but an error will be returned if the value
    /// is empty.
    pub name: Option<&'a str>,
    /// The new color. If `None` the existing color will be used. Will return
    /// an error if `Some` and empty.
    pub color: Option<&'a str>,
    /// The new description. If `None` the exsiting description will be used.
    pub description: Option<&'a str>,
}

impl Default for UpdateTagCategory<'_> {
    fn default() -> Self {
        Self {
            name: None,
            color: None,
            description: None,
        }
    }
}

impl Category {
    fn validate(self) -> Result<Self, Error> {
        if self.name.len() == 0 {
            return Err(Error::InvalidName);
        }

        if self.name.graphemes(true).count() > MAX_NAME_LENGTH {
            return Err(Error::NameTooLong);
        }

        if self.description.graphemes(true).count() > MAX_DESCRIPTION_LENGTH {
            return Err(Error::DescriptionTooLong);
        }

        if let Err(_) = Color::hex(&self.color) {
            return Err(Error::InvalidColor);
        }

        Ok(self)
    }

    fn clean(mut self) -> Self {
        self.name = self.name.trim().into();
        self.color = self.color.to_ascii_lowercase().into();
        self.description = self.description.trim().into();

        self
    }

    fn with_new_data(mut self, new_data: UpdateTagCategory) -> Self {
        self.name = new_data.name.unwrap_or(&self.name).into();
        self.color = new_data.color.unwrap_or(&self.color).into();
        self.description = new_data.description.unwrap_or(&self.description).into();

        self
    }
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

    /// Update an existing category with new data provided as second argument.
    ///
    /// Look at documentation for [`UpdateTagCategory`] to learn about the
    /// data you can provide.
    ///
    /// It returns an error if `id` is not valid, if `new_data` contains
    /// invalid data, or if there were problems with the database.
    pub fn update(&self, id: i32, new_data: UpdateTagCategory) -> Result<(), Error> {
        let data = match self.get(id) {
            Err(err) => return Err(err),
            Ok(val) => val,
        }
        .with_new_data(new_data)
        .clean()
        .validate()?;

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::tag_categories::dsl::{id as tc_id, tag_categories as tc_table};

        match diesel::update(tc_table.filter(tc_id.eq(id)))
            .set(data)
            .execute(conn)
        {
            Err(err) => Err(err.into()),
            Ok(_) => Ok(()),
        }
    }

    /// Searches a category that starts with the provided name.
    ///
    /// The name to search must have at least 3 characters.
    /// This is a convenient function for [`list`] and thus returns the same errors.
    pub fn search_by_name(&self, name: impl AsRef<str>) -> Result<Vec<Category>, Error> {
        if name.as_ref().graphemes(true).count() < 3 {
            return Err(Error::NameToSearchTooShort);
        }

        let res = self.list(None::<Vec<_>>)?;

        Ok(res
            .into_iter()
            .filter(|category| {
                category
                    .name
                    .to_lowercase()
                    .starts_with(&name.as_ref().to_lowercase())
            })
            .collect())
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

    /// Deletes a tag category.
    ///
    /// It returns an error if the ID is not valid, the category was not found
    /// or if there was an error on the database.
    ///
    /// TODO: check if there are tags belonging to this: if there are then error.
    pub fn delete(&self, id: i32) -> Result<(), Error> {
        if let Err(err) = self.get(id) {
            return Err(err);
        }

        use database::schema::tag_categories::dsl::{id as tc_id, tag_categories};
        let conn = &mut self.connection.establish_connection()?;
        match diesel::delete(tag_categories.filter(tc_id.eq(id))).execute(conn) {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(_) => Ok(()),
        }
    }
}
