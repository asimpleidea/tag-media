use diesel::{ExpressionMethods, Insertable, QueryDsl, RunQueryDsl};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    data::tag::Tag,
    database::{
        self,
        connection::DatabaseConnection,
        connection::Error as ConnectionError,
        schema::tags::{self, dsl::tags as tags_table},
    },
    tags::category::{self, tag_categories},
};

const MAX_NAME_LENGTH: usize = 50;
const MAX_DESCRIPTION_LENGTH: usize = 300;

pub struct Tags {
    connection: DatabaseConnection,
}

pub fn tags(connection: DatabaseConnection) -> Tags {
    Tags { connection }
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
    ConnectionError(#[from] ConnectionError),
    /// An invalid category ID was passed, e.g. <= 0
    #[error("invalid category id")]
    InvalidCategoryID,
    /// The category was not found.
    #[error("category not found")]
    CategoryNotFound,
    /// Something happened while getting or working with the category.
    #[error("category error: {0}")]
    CategoryError(#[from] category::Error),
    /// The provided ID is invalid.
    #[error("invalid id")]
    InvalidID,
    /// The provided name is invalid.
    #[error("invalid name")]
    InvalidName,
    /// The provided name is too long.
    #[error("name too long")]
    NameTooLong,
    /// The provided description is too long.
    #[error("description too long")]
    DescriptionTooLong,
    /// The tag was not found.
    #[error("not found")]
    NotFound,
    /// The tag already exists
    #[error("already exists")]
    AlreadyExists,
}

/// Used to define what to update in a tag.
pub struct UpdateTag<'a> {
    /// The new name. If `None` the existing name will be used.
    ///
    /// If `Some` it cannot be empty and cannot be longer that 50 characters.
    pub name: Option<&'a str>,
    /// The new category ID. if `None` the existing one will be used.
    ///
    /// If `Some` it will throw an error if is invalid or not found.
    pub category_id: Option<i32>,
    /// The new description. If `None` the existing one will be used.
    ///
    /// If `Some` it cannot be longer than 300 characters.
    pub description: Option<&'a str>,
}

impl Default for UpdateTag<'_> {
    fn default() -> Self {
        Self {
            name: None,
            category_id: None,
            description: None,
        }
    }
}

impl Tag {
    fn validate(self, connection: &DatabaseConnection) -> Result<Self, Error> {
        if self.category_id <= 0 {
            return Err(Error::InvalidCategoryID);
        }

        match tag_categories(connection.clone()).get(self.category_id) {
            Ok(_) => (),
            Err(err) => return Err(Error::CategoryError(err)),
        };

        match self.name.len() {
            0 => return Err(Error::InvalidName),
            n if n > MAX_NAME_LENGTH => return Err(Error::NameTooLong),
            _ => (),
        };

        if self.description.len() > MAX_DESCRIPTION_LENGTH {
            return Err(Error::DescriptionTooLong);
        }

        Ok(self)
    }

    fn clean(mut self) -> Self {
        self.name = self.name.trim().into();
        self.description = self.description.trim().into();

        self
    }

    fn with_new_data(mut self, new_data: UpdateTag) -> Self {
        self.name = new_data.name.unwrap_or(&self.name).into();
        self.category_id = new_data.category_id.unwrap_or(self.category_id);
        self.description = new_data.description.unwrap_or(&self.description).into();

        self
    }
}

impl From<CreateTag> for Tag {
    fn from(value: CreateTag) -> Self {
        Self {
            id: 0,
            name: value.name,
            category_id: value.category_id,
            description: value.description,
        }
    }
}

impl CreateTag {
    fn into_tag(self) -> Tag {
        Tag {
            id: 0,
            name: self.name,
            category_id: self.category_id,
            description: self.description,
        }
    }
}

impl From<Tag> for CreateTag {
    fn from(value: Tag) -> Self {
        Self {
            name: value.name,
            category_id: value.category_id,
            description: value.description,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = tags)]
/// Represents a new tag to insert.
pub struct CreateTag {
    /// The name of the tag.
    pub name: String,
    /// The category that it will belong to.
    pub category_id: i32,
    /// The description.
    pub description: String,
}

impl Tags {
    /// Inserts a new tag on the database.
    ///
    /// Returns an error if the data is not valid, it already exists or if
    /// there were errors with the database.
    pub fn create(&self, data: CreateTag) -> Result<Tag, Error> {
        let data: CreateTag = data.into_tag().clean().validate(&self.connection)?.into();

        match self.already_exists(&data.name, data.category_id) {
            Err(err) => return Err(err),
            Ok(exists) if exists.is_some() => return Err(Error::AlreadyExists),
            Ok(_) => (),
        }

        let conn = &mut self.connection.establish_connection()?;
        match diesel::insert_into(tags_table)
            .values(data)
            .get_result(conn)
        {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(inserted) => Ok(inserted),
        }
    }

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

    /// Updates a tag with the provided ID and the provided
    /// [`new data`](UpdateTag).
    ///
    /// Take a look at [`UpdateTag`] to learn about the errors in the data.
    /// It returns an error in case the tag does not exist, the new data is
    /// invalid or there were problems in the database.
    pub fn update(&self, id: i32, new_data: UpdateTag) -> Result<(), Error> {
        let data = match self.get(id) {
            Err(err) => return Err(err),
            Ok(val) => val,
        }
        .with_new_data(new_data)
        .clean()
        .validate(&self.connection)?;

        match self.already_exists(&data.name, data.category_id) {
            Err(err) => return Err(err),
            Ok(exists) if exists.is_some() => return Err(Error::AlreadyExists),
            Ok(_) => (),
        }

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::tags::dsl::id as tag_id;
        match diesel::update(tags_table.filter(tag_id.eq(id)))
            .set(data)
            .execute(conn)
        {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(_) => Ok(()),
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

    /// Searches a tag that starts with the provided name.
    ///
    /// The name to search must have at least 3 characters.
    /// This is a convenient function for [`list`] and thus returns the same errors.
    pub fn search_by_name(&self, name: impl AsRef<str>) -> Result<Vec<Tag>, Error> {
        let name_to_search = name.as_ref().trim();
        if name_to_search.graphemes(true).count() < 3 {
            return Err(Error::InvalidName);
        }

        let list = self.list(None)?;

        Ok(list
            .into_iter()
            .filter(|tag| {
                tag.name
                    .to_ascii_lowercase()
                    .starts_with(&name_to_search.to_ascii_lowercase())
            })
            .collect())
    }

    /// Deletes the tag with the provided id
    ///
    /// Returns the same errors as the `get` function.
    ///
    /// TODO: check if there is any media with this tag. If there are, then
    /// prevent delete.
    pub fn delete(&self, id: i32) -> Result<(), Error> {
        if let Err(err) = self.get(id) {
            return Err(err);
        }

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::tags::dsl::id as tag_id;
        match diesel::delete(tags_table.filter(tag_id.eq(id))).execute(conn) {
            Err(err) => Err(Error::DatabaseError(err)),
            Ok(_) => Ok(()),
        }
    }

    fn already_exists(&self, name: &str, category_id: i32) -> Result<Option<()>, Error> {
        use database::schema::tags::dsl::{category_id as cat_id, name as tag_name};
        let conn = &mut self.connection.establish_connection()?;

        match tags_table
            .filter(tag_name.eq(name))
            .filter(cat_id.eq(category_id))
            .first::<Tag>(conn)
        {
            Err(err) if err == diesel::result::Error::NotFound => return Ok(None),
            Err(err) => return Err(Error::DatabaseError(err)),
            Ok(_) => return Ok(Some(())),
        }
    }
}
