use errors::Error;
use std::fmt;

/// Represents a MIME content type for a file
///
/// # Examples
/// ```
/// use value_object::ContentType;
///
/// let content_type = ContentType::new("text/plain").unwrap();
/// assert_eq!(content_type.to_string(), "text/plain");
///
/// // Error cases
/// assert!(ContentType::new("").is_err());
/// assert!(ContentType::new("text").is_err());
/// assert!(ContentType::new("text plain").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType(String);

impl ContentType {
    /// Creates a new ContentType from a string value
    ///
    /// # Errors
    /// Returns `Error::BadRequest` if:
    /// - value is empty
    /// - value does not contain '/'
    /// - value contains invalid characters
    pub fn new(value: &str) -> Result<Self, Error> {
        if value.is_empty() {
            return Err(Error::parse_from_string(
                "Content type cannot be empty",
            ));
        }

        if !value.contains('/') {
            return Err(Error::parse_from_string(
                "Content type must contain '/' (e.g., 'text/plain')",
            ));
        }

        // TODO: add English comment
        if value.contains(|c: char| {
            c == '<'
                || c == '>'
                || c == '"'
                || c == '{'
                || c == '}'
                || c == '|'
                || c == '\\'
                || c.is_whitespace()
        }) {
            return Err(Error::parse_from_string(
                "Content type contains invalid characters",
            ));
        }

        Ok(Self(value.to_string()))
    }

    /// Determines the content type from a file extension
    ///
    /// # Examples
    /// ```
    /// use value_object::ContentType;
    ///
    /// let content_type = ContentType::from_extension("txt");
    /// assert_eq!(content_type.to_string(), "text/plain");
    /// ```
    pub fn from_extension(extension: &str) -> Self {
        let content_type = match extension.to_lowercase().as_str() {
            "txt" => "text/plain",
            "html" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "doc" | "docx" => "application/msword",
            "xls" | "xlsx" => "application/vnd.ms-excel",
            "ppt" | "pptx" => "application/vnd.ms-powerpoint",
            _ => "application/octet-stream",
        };
        // TODO: add English comment
        ContentType::new(content_type)
            .expect("Invalid predefined content type")
    }

    /// Determines the content type from a filename
    ///
    /// # Examples
    /// ```
    /// use value_object::ContentType;
    ///
    /// let content_type = ContentType::from_filename("document.txt");
    /// assert_eq!(content_type.to_string(), "text/plain");
    /// ```
    pub fn from_filename(filename: &str) -> Self {
        let extension = filename.split('.').next_back().unwrap_or("");
        Self::from_extension(extension)
    }

    /// Returns the content type as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ContentType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for ContentType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ContentType::new(s)
    }
}

/// Represents an in-memory file with its metadata and content
///
/// # Examples
/// ```
/// # use value_object::{InMemoryFile, ContentType};
/// # use bytes::Bytes;
///
/// // Create a new file with explicit content type
/// let file = InMemoryFile::new(
///     "test.txt".to_string(),
///     Some("text/plain".to_string()),
///     Bytes::from("Hello, World!")
/// ).unwrap();
///
/// // Verify the file properties
/// assert_eq!(file.filename(), "test.txt");
/// assert_eq!(file.content_type().to_string(), "text/plain");
///
/// // Create a file without content type (auto-detected from extension)
/// let auto_file = InMemoryFile::new(
///     "image.png".to_string(),
///     None,
///     Bytes::from("image data")
/// ).unwrap();
///
/// assert_eq!(auto_file.filename(), "image.png");
/// assert_eq!(auto_file.content_type().to_string(), "image/png");
///
/// // Error cases
/// let err = InMemoryFile::new(
///     "".to_string(),
///     None,
///     Bytes::from("empty filename")
/// ).unwrap_err();
/// assert!(matches!(err, errors::Error::BadRequest { .. }));
///
/// let err = InMemoryFile::new(
///     "invalid/filename.txt".to_string(),
///     None,
///     Bytes::from("invalid filename")
/// ).unwrap_err();
/// assert!(matches!(err, errors::Error::BadRequest { .. }));
/// ```
#[derive(Clone)]
pub struct InMemoryFile {
    filename: String,
    content_type: ContentType,
    content: bytes::Bytes,
}

impl fmt::Debug for InMemoryFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InMemoryFile")
            .field("filename", &self.filename)
            .field("content_type", &self.content_type)
            .field("content", &format!("[{} bytes]", self.content.len()))
            .finish()
    }
}

impl InMemoryFile {
    /// Creates a new InMemoryFile instance
    ///
    /// If content_type is None, it will be determined from the filename extension
    ///
    /// # Errors
    /// Returns `Error::BadRequest` if:
    /// - filename is empty
    /// - filename contains invalid characters
    pub fn new(
        filename: String,
        content_type: Option<String>,
        content: bytes::Bytes,
    ) -> Result<Self, Error> {
        if filename.is_empty() {
            return Err(Error::parse_from_string(
                "Filename cannot be empty",
            ));
        }

        // TODO: add English comment
        if filename.contains(|c: char| {
            c == '/'
                || c == '\\'
                || c == ':'
                || c == '*'
                || c == '?'
                || c == '"'
                || c == '<'
                || c == '>'
                || c == '|'
        }) {
            return Err(Error::parse_from_string(
                "Filename contains invalid characters",
            ));
        }

        let content_type = match content_type {
            Some(ct) => ContentType::new(&ct)?,
            None => ContentType::from_filename(&filename),
        };

        Ok(InMemoryFile {
            filename,
            content_type,
            content,
        })
    }

    /// Returns the filename as a string slice
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Returns a reference to the ContentType
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// Returns a reference to the file content
    pub fn content(&self) -> &bytes::Bytes {
        &self.content
    }
}

impl Default for InMemoryFile {
    fn default() -> Self {
        InMemoryFile {
            filename: "name".into(),
            content_type: ContentType::from_extension("txt"),
            content: bytes::Bytes::new(),
        }
    }
}

#[cfg(feature = "async-graphql")]
impl TryFrom<async_graphql::UploadValue> for InMemoryFile {
    type Error = Error;

    fn try_from(
        value: async_graphql::UploadValue,
    ) -> Result<Self, Self::Error> {
        InMemoryFile::new(value.filename, value.content_type, value.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_from_extension() {
        assert_eq!(
            ContentType::from_extension("txt").to_string(),
            "text/plain"
        );
        assert_eq!(
            ContentType::from_extension("png").to_string(),
            "image/png"
        );
        assert_eq!(
            ContentType::from_extension("unknown").to_string(),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_content_type_from_filename() {
        assert_eq!(
            ContentType::from_filename("test.txt").to_string(),
            "text/plain"
        );
        assert_eq!(
            ContentType::from_filename("image.png").to_string(),
            "image/png"
        );
        assert_eq!(
            ContentType::from_filename("unknown").to_string(),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_new_with_content_type() {
        let file = InMemoryFile::new(
            "test.txt".to_string(),
            Some("text/plain".to_string()),
            bytes::Bytes::from("test content"),
        )
        .unwrap();
        assert_eq!(file.content_type().to_string(), "text/plain");
    }

    #[test]
    fn test_new_without_content_type() {
        let file = InMemoryFile::new(
            "test.txt".to_string(),
            None,
            bytes::Bytes::from("test content"),
        )
        .unwrap();
        assert_eq!(file.content_type().to_string(), "text/plain");
    }

    #[test]
    fn test_new_with_empty_filename() {
        let result = InMemoryFile::new(
            "".to_string(),
            None,
            bytes::Bytes::from("test content"),
        );
        assert!(matches!(result.unwrap_err(), Error::BadRequest { .. }));
    }

    #[test]
    fn test_new_with_invalid_filename() {
        let result = InMemoryFile::new(
            "test/file.txt".to_string(),
            None,
            bytes::Bytes::from("test content"),
        );
        assert!(matches!(result.unwrap_err(), Error::BadRequest { .. }));
    }

    #[cfg(feature = "async-graphql")]
    #[test]
    fn test_try_from_upload_value() {
        use async_graphql::UploadValue;

        // TODO: add English comment
        let upload = UploadValue {
            filename: "test.txt".to_string(),
            content_type: Some("text/plain".to_string()),
            content: bytes::Bytes::from("test content"),
        };
        let file = InMemoryFile::try_from(upload).unwrap();
        assert_eq!(file.filename(), "test.txt");
        assert_eq!(file.content_type().to_string(), "text/plain");

        // TODO: add English comment
        let upload = UploadValue {
            filename: "".to_string(),
            content_type: Some("text/plain".to_string()),
            content: bytes::Bytes::from("test content"),
        };
        let err = InMemoryFile::try_from(upload).unwrap_err();
        assert!(matches!(err, Error::BadRequest { .. }));
    }

    #[test]
    fn test_content_type_new() {
        assert!(ContentType::new("").is_err());
        assert!(ContentType::new("text").is_err());
        assert!(ContentType::new("text plain").is_err());
        assert!(ContentType::new("text/plain").is_ok());
        assert!(ContentType::new("text/<plain>").is_err());
    }

    #[test]
    fn test_content_type_from_str() {
        assert!("text/plain".parse::<ContentType>().is_ok());
        assert!("".parse::<ContentType>().is_err());
        assert!("text".parse::<ContentType>().is_err());
        assert!("text plain".parse::<ContentType>().is_err());
        assert!("text/<plain>".parse::<ContentType>().is_err());
    }
}
