use chrono::{DateTime, Duration, Utc};
use errors::Error;
use std::str::FromStr;
use url::Url;

/// A pre-signed URL with an expiration time.
///
/// Pre-signed URLs provide temporary access to private resources.
/// They are commonly used with object storage services like S3.
///
/// # Examples
///
/// ```
/// use value_object::PresignedUrl;
/// use url::Url;
/// use chrono::{Utc, Duration};
///
/// let url = Url::parse("https://example.com/file.pdf").unwrap();
/// let expires_in = Duration::hours(1);
/// let presigned = PresignedUrl::with_duration(url, expires_in);
///
/// assert!(!presigned.is_expired());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresignedUrl {
    url: Url,
    expires_at: DateTime<Utc>,
}

impl PresignedUrl {
    /// Creates a new PresignedUrl with the specified URL and expiration time.
    ///
    /// # Arguments
    ///
    /// * `url` - The pre-signed URL
    /// * `expires_at` - When the URL will expire
    pub fn new(url: Url, expires_at: DateTime<Utc>) -> Self {
        Self { url, expires_at }
    }

    /// Creates a new PresignedUrl that expires after the specified duration.
    ///
    /// # Arguments
    ///
    /// * `url` - The pre-signed URL
    /// * `duration` - How long the URL should be valid for
    pub fn with_duration(url: Url, duration: Duration) -> Self {
        let expires_at = Utc::now() + duration;
        Self::new(url, expires_at)
    }

    /// Returns the URL string.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Returns when the URL will expire.
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Returns true if the URL has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Returns how long until the URL expires.
    /// Returns None if already expired.
    pub fn time_remaining(&self) -> Option<Duration> {
        let now = Utc::now();
        if now > self.expires_at {
            None
        } else {
            Some(self.expires_at - now)
        }
    }

    /// Returns true if the URL will expire within the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration to check against
    pub fn expires_within(&self, duration: Duration) -> bool {
        match self.time_remaining() {
            Some(remaining) => remaining <= duration,
            None => true, // Already expired
        }
    }
}

impl FromStr for PresignedUrl {
    type Err = Error;

    /// Creates a PresignedUrl from a string, with a default expiration of 1 hour.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s)
            .map_err(|e| Error::type_error(format!("Invalid URL: {e}")))?;
        let expires_at = Utc::now() + Duration::hours(1);
        Ok(Self { url, expires_at })
    }
}

impl std::fmt::Display for PresignedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl AsRef<str> for PresignedUrl {
    fn as_ref(&self) -> &str {
        self.url.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expiration() {
        let url = Url::parse("https://example.com/test").unwrap();
        let presigned =
            PresignedUrl::with_duration(url, Duration::seconds(1));

        assert!(!presigned.is_expired());
        assert!(presigned.time_remaining().is_some());

        std::thread::sleep(std::time::Duration::from_secs(2));

        assert!(presigned.is_expired());
        assert!(presigned.time_remaining().is_none());
    }

    #[test]
    fn test_expires_within() {
        let url = Url::parse("https://example.com/test").unwrap();
        let presigned =
            PresignedUrl::with_duration(url, Duration::hours(2));

        assert!(!presigned.expires_within(Duration::hours(1)));
        assert!(presigned.expires_within(Duration::hours(3)));
    }
}
