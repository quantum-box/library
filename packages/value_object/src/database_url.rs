use derive_getters::Getters;
use std::fmt::{self, Display};
use std::str::FromStr;
use url::Url;

#[derive(Debug, Clone, Getters)]
pub struct DatabaseUrl {
    scheme: String,
    username: String,
    password: String,
    host: String,
    port: u16,
    database: Option<String>,
}

impl DatabaseUrl {
    pub fn new(dsn: impl ToString) -> Result<Self, errors::Error> {
        parse_dsn(dsn.to_string().as_str())
    }

    pub fn use_database(&self, database: impl ToString) -> Self {
        Self {
            database: Some(database.to_string()),
            ..self.clone()
        }
    }

    pub fn new_from_env() -> Self {
        let dsn = match std::env::var("DATABASE_URL") {
            Ok(dsn) => dsn,
            Err(_) => std::env::var("DEV_DATABASE_URL")
                .expect("DATABASE_URL and DEV_DATABASE_URL are not set"),
        };
        parse_dsn(dsn.as_str()).expect("Failed to parse DSN")
    }
}

impl FromStr for DatabaseUrl {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_dsn(s)
    }
}

// TODO: add English comment
fn parse_dsn(dsn: &str) -> errors::Result<DatabaseUrl> {
    let url = Url::parse(dsn).map_err(|e| {
        errors::Error::invalid(format!("Failed to parse DSN: {e}"))
    })?;

    if !url.scheme().eq("mysql")
        && !url.scheme().eq("postgres")
        && !url.scheme().eq("sqlite")
    {
        return Err(errors::Error::invalid(
            "Protocol is not valid. DSN must start mysql or postgres, sqlite.",
        ));
    }

    Ok(DatabaseUrl {
        scheme: url.scheme().to_string(),
        username: url.username().to_string(),
        password: url.password().unwrap_or("").to_string(),
        host: url
            .host_str()
            .ok_or(errors::Error::invalid(
                "Failed to parse DSN. Not found host.",
            ))?
            .to_string(),
        port: url.port().unwrap_or(5432),
        database: if url.path().contains("/") {
            Some(url.path().trim_start_matches('/').to_string())
        } else {
            None
        },
    })
}

impl Display for DatabaseUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}://{}:{}@{}:{}{}",
            self.scheme,
            self.username,
            self.password,
            self.host,
            self.port,
            self.database
                .clone()
                .map(|s| format!("/{s}"))
                .unwrap_or("".to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dsn() {
        let dsn = "postgres://user:password@host:5432/database";
        let config = parse_dsn(dsn).unwrap();
        assert_eq!(config.scheme, "postgres");
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "password");
        assert_eq!(config.host, "host");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, Some("database".to_string()));
    }

    #[test]
    fn test_parse_dsn_with_no_database() {
        let dsn = "postgres://user:password@host:5432";
        let config = parse_dsn(dsn).unwrap();
        assert_eq!(config.scheme, "postgres");
        assert_eq!(config.username, "user");
        assert_eq!(config.password, "password");
        assert_eq!(config.host, "host");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, None);
    }

    #[test]
    fn test_to_string() {
        let dsn = "postgres://user:password@host:5432/database";
        let config = parse_dsn(dsn).unwrap();
        assert_eq!(config.to_string(), dsn);
    }
}
