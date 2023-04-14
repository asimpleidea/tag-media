use crate::database::schema::media;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    sql_types::Text,
    AsChangeset, Queryable,
};
use serde::Serialize;

/// This represents a media type.
#[derive(Debug, Serialize)]
pub enum MediaType {
    Unknown,
    Image,
    Video,
    Sound,
}

impl Into<MediaType> for String {
    fn into(self) -> MediaType {
        match self.as_str() {
            "image" => MediaType::Image,
            "video" => MediaType::Video,
            "sound" => MediaType::Sound,
            _ => MediaType::Unknown,
        }
    }
}

impl From<MediaType> for String {
    fn from(value: MediaType) -> String {
        match value {
            MediaType::Image => String::from("image"),
            MediaType::Video => String::from("video"),
            MediaType::Sound => String::from("sound"),
            _ => String::from(""),
        }
    }
}

// Needed for a good deserialization for [`MediaType`].
impl<DB> Queryable<Text, DB> for MediaType
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    type Row = String;

    fn build(s: String) -> deserialize::Result<Self> {
        Ok(s.into())
    }
}

/// This represents a media file.
#[derive(Debug, Queryable, Serialize, AsChangeset)]
#[diesel(table_name = media)]
pub struct MediaFile {
    /// The ID of the file.
    pub id: i64,
    /// The path relative to its base path.
    pub relative_path: String,
    /// The ID of the path that this belongs to.
    pub base_path_id: i32,
    /// The width, if an image or video.
    pub width: Option<i16>,
    /// The height, if an image or video.
    pub height: Option<i16>,
    /// The size of the file in kB.
    pub size: f64,
    /// The mark, from 1 to 10.
    pub mark: Option<i16>,
    /// The description for this file.
    pub description: String,
    /// The type of this file, e.g. `Image`, `Video` or `Sound`.
    /// See [`MediaType`]
    #[diesel(serialize_as = String)]
    pub media_type: MediaType,
}
