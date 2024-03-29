use super::base_paths::Error as BasePathsError;
use crate::{
    data::{
        media_file::{MediaFile, MediaType},
        tag::Tag,
    },
    database::{
        self,
        connection::{DatabaseConnection, Error as ConnectionError},
        schema::{
            media::{self, dsl::media as media_table},
            media_tags::{self},
        },
    },
    media::base_paths,
    tags::{self},
};
use diesel::{
    dsl::count_distinct, ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl,
};
use std::convert::From;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

const MAX_DESCRIPTION_LENGTH: usize = 300;

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
    /// The provided width is invalid, e.g. is <= 0.
    #[error("invalid width")]
    InvalidWidth,
    /// The provided height is invalid, e.g. is <= 0.
    #[error("invalid height")]
    InvalidHeight,
    /// The provided size is invalid, e.g. is <= 0.
    #[error("invalid size")]
    InvalidSize,
    /// The provided mark is invalid, e.g. is <= 0 or > 10.
    #[error("invalid mark")]
    InvalidMark,
    /// The provided description is longer than 300 characters.
    #[error("description too long")]
    DescriptionTooLong,
    /// The provided media is already there.
    #[error("already exists")]
    AlreadyExists,
    /// Cannot delete because the media is still referenced somewhere.
    #[error("in use")]
    InUse,
    // Tag error occurred.
    #[error("error in tags: {0}")]
    TagError(#[from] tags::tags::Error),
    // Tthis tag is already present in this media.
    #[error("media is already tagged")]
    AlreadyTagged,
    /// The media was not tagged with this.
    #[error("tag not found")]
    TagNotFound,
    /// No tags have been provided.
    #[error("no tags have been provided")]
    NoTagsProvided,
}

pub struct Media {
    connection: DatabaseConnection,
}

pub fn media(connection: DatabaseConnection) -> Media {
    Media { connection }
}

/// Represents a media file to create.
#[derive(Insertable)]
#[diesel(table_name = media)]
pub struct CreateMediaFile {
    /// The relative path.
    pub relative_path: String,
    /// The ID of the parent base path.
    pub base_path_id: i32,
    /// The width, if an image or a video.
    pub width: Option<i16>,
    /// The height, if an image or a video.
    pub height: Option<i16>,
    /// The size in kB.
    pub size: f64,
    /// The mark, from 1 to 10.
    pub mark: Option<i16>,
    /// The description.
    pub description: String,
    /// The media type, e.g. Image, Video or Sound.
    #[diesel(serialize_as = String)]
    pub media_type: MediaType,
}

/// Represents a media file to update.
///
/// Take a look at [`CreateMediaFile`] for the values.
/// If `None` the existing values will be used.
pub struct UpdateMediaFile {
    pub width: Option<i16>,
    pub height: Option<i16>,
    pub size: Option<f64>,
    pub mark: Option<i16>,
    pub description: Option<String>,
}

impl MediaFile {
    fn validate(self) -> Result<MediaFile, Error> {
        if self.relative_path.is_empty() {
            return Err(Error::InvalidRelativePath);
        }

        if self.base_path_id <= 0 {
            return Err(Error::InvalidBasePathID);
        }

        match self.width {
            Some(val) if val <= 0 => return Err(Error::InvalidWidth),
            _ => (),
        };

        match self.height {
            Some(val) if val <= 0 => return Err(Error::InvalidHeight),
            _ => (),
        };

        if self.size <= 0.0 {
            return Err(Error::InvalidSize);
        }

        match self.mark {
            Some(val) if val <= 0 || val > 10 => return Err(Error::InvalidMark),
            _ => (),
        };

        if self.description.graphemes(true).count() > MAX_DESCRIPTION_LENGTH {
            return Err(Error::DescriptionTooLong);
        }

        Ok(self)
    }

    fn with_new_data(mut self, update_data: UpdateMediaFile) -> Self {
        self = MediaFile {
            id: self.id,
            relative_path: self.relative_path,
            base_path_id: self.base_path_id,
            width: match update_data.width {
                None => self.width,
                Some(width) => Some(width),
            },
            height: match update_data.height {
                None => self.height,
                Some(height) => Some(height),
            },
            size: update_data.size.unwrap_or(self.size),
            mark: match update_data.mark {
                None => self.mark,
                Some(mark) => Some(mark),
            },
            description: update_data.description.unwrap_or(self.description),
            media_type: self.media_type,
        };

        self
    }
}

impl From<CreateMediaFile> for MediaFile {
    fn from(value: CreateMediaFile) -> Self {
        Self {
            id: 0,
            relative_path: value.relative_path.trim_matches('/').into(),
            base_path_id: value.base_path_id,
            width: value.width,
            height: value.height,
            size: value.size,
            mark: value.mark,
            description: value.description.trim().into(),
            media_type: value.media_type,
        }
    }
}

impl Into<CreateMediaFile> for MediaFile {
    fn into(self) -> CreateMediaFile {
        CreateMediaFile {
            relative_path: self.relative_path,
            base_path_id: self.base_path_id,
            width: self.width,
            height: self.height,
            size: self.size,
            mark: self.mark,
            description: self.description,
            media_type: self.media_type,
        }
    }
}
#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = media_tags)]
struct MediaTag {
    id: i64,
    media_id: i64,
    tag_id: i32,
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

    /// Creates a media file.
    ///
    /// It returns the created `MediaFile` or an error.
    /// Look at [`CreateMediaFile`] for more clues on the errors.
    pub fn create(&self, create_data: CreateMediaFile) -> Result<MediaFile, Error> {
        if let Err(err) =
            base_paths::base_paths(self.connection.clone()).get(create_data.base_path_id)
        {
            return Err(Error::BasePathsError(err));
        }

        let data: CreateMediaFile = MediaFile::from(create_data).validate()?.into();

        {
            let conn = &mut self.connection.establish_connection()?;
            use database::schema::media::dsl::{base_path_id, relative_path};
            match media_table
                .filter(base_path_id.eq(data.base_path_id))
                .filter(relative_path.eq(&data.relative_path))
                .first::<MediaFile>(conn)
            {
                Ok(_) => return Err(Error::AlreadyExists),
                Err(err) if err == diesel::result::Error::NotFound => (),
                Err(err) => return Err(Error::DatabaseError(err)),
            }
        }

        let conn = &mut self.connection.establish_connection()?;
        match diesel::insert_into(media_table)
            .values(data)
            .get_result(conn)
        {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::DatabaseError(err)),
        }
    }

    /// Updates a media file with the provided Id with the provided new data.
    pub fn update(&self, id: i64, update_data: UpdateMediaFile) -> Result<(), Error> {
        let data = self.get(id)?.with_new_data(update_data).validate()?;

        let conn = &mut self.connection.establish_connection()?;
        use database::schema::media::dsl::id as media_id;
        match diesel::update(media_table.filter(media_id.eq(id)))
            .set(data)
            .execute(conn)
        {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::DatabaseError(err)),
        }
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

    /// Deletes a media file with the provided ID.
    pub fn delete(&self, id: i64) -> Result<(), Error> {
        let _existing = self.get(id)?;

        let conn = &mut self.connection.establish_connection()?;

        {
            use database::schema::media_tags::dsl::{media_id, media_tags as mt_table};
            match mt_table.filter(media_id.eq(id)).count().execute(conn) {
                Ok(vals) if vals == 0 => (),
                Ok(_) => return Err(Error::InUse),
                Err(err) => return Err(Error::DatabaseError(err)),
            }
        }

        use database::schema::media::dsl::id as media_id;
        match diesel::delete(media_table.filter(media_id.eq(id))).execute(conn) {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::DatabaseError(err)),
        }
    }

    /// Inserts a tag for the media id with the provided tag id.
    pub fn insert_tag(&self, media_id: i64, tag_id: i32) -> Result<(), Error> {
        if let Err(err) = self.get(media_id) {
            return Err(err);
        }

        if let Err(err) = tags::tags::tags(self.connection.clone()).get(tag_id) {
            return Err(Error::TagError(err));
        }

        use database::schema::media_tags::dsl::{
            media_id as mid, media_tags as media_tags_table, tag_id as tid,
        };
        let conn = &mut self.connection.establish_connection()?;
        match media_tags_table
            .filter(mid.eq(media_id))
            .filter(tid.eq(tag_id))
            .first::<MediaTag>(conn)
        {
            Ok(_) => return Err(Error::AlreadyTagged),
            Err(err) if err == diesel::NotFound => (),
            Err(err) => return Err(Error::DatabaseError(err)),
        };

        match diesel::insert_into(media_tags_table)
            .values(&vec![(mid.eq(media_id), tid.eq(tag_id))])
            .execute(conn)
        {
            Ok(_) => Ok(()),
            Err(err) => return Err(Error::DatabaseError(err)),
        }
    }

    /// List all the tags that are available for the provided media.
    pub fn list_tags_for_media(&self, media_id: i64) -> Result<Vec<Tag>, Error> {
        if media_id <= 0 {
            return Err(Error::InvalidID);
        }

        match self.get(media_id) {
            Ok(_) => (),
            Err(err) => return Err(err),
        }

        let conn = &mut self.connection.establish_connection()?;

        let tag_ids = {
            use database::schema::media_tags::dsl::{media_id as mid, media_tags as md_table};

            match md_table.filter(mid.eq(media_id)).load::<MediaTag>(conn) {
                Err(err) => return Err(Error::DatabaseError(err)),
                Ok(vals) => vals.into_iter().map(|mt| mt.tag_id).collect::<Vec<i32>>(),
            }
        };

        use database::schema::tags::dsl::{id, tags as tags_table};
        tags_table
            .filter(id.eq_any(tag_ids))
            .load(conn)
            .map_err(|err| Error::DatabaseError(err))
    }

    /// Removes (untags) a media.
    pub fn untag_media(&self, media_id: i64, tag_id: i32) -> Result<(), Error> {
        if let Err(err) = self.get(media_id) {
            return Err(err);
        }

        if let Err(err) = tags::tags::tags(self.connection.clone()).get(tag_id) {
            return Err(Error::TagError(err));
        }

        if !self
            .list_tags_for_media(media_id)?
            .into_iter()
            .any(|tag| tag.id == tag_id)
        {
            return Err(Error::TagNotFound);
        }

        use database::schema::media_tags::dsl::{
            media_id as mid, media_tags as md_table, tag_id as tid,
        };
        let conn = &mut self.connection.establish_connection()?;
        match diesel::delete(md_table.filter(mid.eq(mid)).filter(tid.eq(tag_id))).execute(conn) {
            Ok(_) => return Ok(()),
            Err(err) => return Err(Error::DatabaseError(err)),
        }
    }

    /// List media starting from tags
    pub fn list_media_from_tags(
        &self,
        tags: impl IntoIterator<Item = i32>,
    ) -> Result<Vec<MediaFile>, Error> {
        let tag_ids = {
            let mut t = tags.into_iter().collect::<Vec<i32>>();

            t.sort_unstable();
            t.dedup();
            t
        };

        if tag_ids.is_empty() {
            return Err(Error::NoTagsProvided);
        }

        use database::schema::media_tags::dsl::{media_id, media_tags as mt_table, tag_id as tid};
        let conn = &mut self.connection.establish_connection()?;

        let img_ids = mt_table
            .select(media_id)
            .filter(tid.eq_any(&tag_ids))
            .group_by(media_id)
            .having(count_distinct(tid).eq(tag_ids.len() as i64))
            .distinct()
            .load::<i64>(conn)?;

        use database::schema::media::dsl::id;
        media_table
            .filter(id.eq_any(img_ids))
            .load(conn)
            .map_err(|err| Error::DatabaseError(err))
    }
}
