use clap::Parser;

#[derive(Parser, Clone)]
pub struct Config {
    #[clap(long = "port", env = "PORT", default_value = "50053")]
    pub port: u16,
    #[clap(
        long = "environment",
        env = "ENVIRONMENT",
        default_value = "development"
    )]
    pub environment: String,
    #[clap(
        long = "database_url",
        env = "DATABASE_URL",
        default_value = "mysql://root:@localhost:15000"
    )]
    pub database_url: String,

    #[clap(
        long = "property_value_storage_mode",
        env = "PROPERTY_VALUE_STORAGE_MODE",
        default_value = "legacy_only"
    )]
    pub property_value_storage_mode: String,

    #[clap(long = "cognito_jwk_url", env = "COGNITO_JWK_URL")]
    pub cognito_jwk_url: String,

    #[clap(
        long = "otel_exporter_otlp_endpoint",
        env = "OTEL_EXPORTER_OTLP_ENDPOINT"
    )]
    pub otel_exporter_otlp_endpoint: Option<String>,

    #[clap(long = "sentry_dsn", env = "SENTRY_DSN")]
    pub sentry_dsn: Option<String>,

    #[clap(
        long = "cognito_user_pool_id",
        env = "COGNITO_USER_POOL_ID",
        default_value = "ap-northeast-1_8Ga4bK5M4"
    )]
    pub cognito_user_pool_id: String,

    /// Base URL of tachyon-api for SDK REST calls
    #[clap(
        long = "tachyon_api_url",
        env = "TACHYON_API_URL",
        default_value = "https://api.n1.tachy.one"
    )]
    pub tachyon_api_url: String,

    /// Service authentication token for tachyon-api
    #[clap(
        long = "service_auth_token",
        env = "SERVICE_AUTH_TOKEN",
        default_value = "dummy-token"
    )]
    pub service_auth_token: String,
}

impl Config {
    pub fn validate_for_server_startup(
        &self,
    ) -> Result<(), std::io::Error> {
        self.property_value_storage_mode
            .parse::<database_manager::property_value_rollout::PropertyValueStorageMode>()
            .map_err(|error| invalid_config(&error.to_string()))?;
        let environment = self.environment.to_ascii_lowercase();
        let is_production =
            environment == "prod" || environment == "production";
        let is_lambda = std::env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok();

        if is_lambda && !is_production {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ENVIRONMENT must be production when running on AWS Lambda",
            ));
        }

        if is_production {
            reject_empty(
                "COGNITO_JWK_URL",
                &self.cognito_jwk_url,
                "COGNITO_JWK_URL must be configured in production",
            )?;
            reject_empty(
                "COGNITO_USER_POOL_ID",
                &self.cognito_user_pool_id,
                "COGNITO_USER_POOL_ID must be configured in production",
            )?;
            reject_empty(
                "TACHYON_API_URL",
                &self.tachyon_api_url,
                "TACHYON_API_URL must be configured in production",
            )?;

            if self.service_auth_token.trim().is_empty()
                || is_dangerous_secret(&self.service_auth_token)
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "SERVICE_AUTH_TOKEN must be configured in production",
                ));
            }

            if self.database_url.contains("localhost")
                || self.database_url.contains("127.0.0.1")
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "DATABASE_URL must not point at localhost in production",
                ));
            }
            reject_empty(
                "DATABASE_URL",
                &self.database_url,
                "DATABASE_URL must be configured in production",
            )?;
            require_non_zero_u32_env("DB_POOL_MAX_CONNECTIONS")?;
            require_non_zero_u64_env("DB_POOL_ACQUIRE_TIMEOUT_SECS")?;

            validate_https_url("COGNITO_JWK_URL", &self.cognito_jwk_url)?;
            validate_https_url("TACHYON_API_URL", &self.tachyon_api_url)?;
            if self.cognito_jwk_url.contains("localhost")
                || self.cognito_jwk_url.contains("127.0.0.1")
            {
                return Err(invalid_config(
                    "COGNITO_JWK_URL must not point at localhost in production",
                ));
            }
            if !self.cognito_jwk_url.contains(&self.cognito_user_pool_id) {
                return Err(invalid_config(
                    "COGNITO_JWK_URL must match COGNITO_USER_POOL_ID in production",
                ));
            }
            if self.tachyon_api_url.contains("localhost")
                || self.tachyon_api_url.contains("127.0.0.1")
                || self.tachyon_api_url.contains("pages.dev")
            {
                return Err(invalid_config(
                    "TACHYON_API_URL must point at the production API origin",
                ));
            }

            let parquet_bucket =
                std::env::var("LIBRARY_PARQUET_BUCKET").map_err(|_| {
                    invalid_config(
                        "LIBRARY_PARQUET_BUCKET must be configured in production",
                    )
                })?;
            if parquet_bucket.trim().is_empty()
                || parquet_bucket == "library-parquet"
            {
                return Err(invalid_config(
                    "LIBRARY_PARQUET_BUCKET must not use the development default in production",
                ));
            }

            for name in [
                "MINIO_ENDPOINT",
                "MINIO_PUBLIC_ENDPOINT",
                "MINIO_ROOT_USER",
                "MINIO_ROOT_PASSWORD",
                "SKIP_MINIO_SETUP",
            ] {
                if std::env::var(name).is_ok() {
                    return Err(invalid_config(
                        "MINIO_* and SKIP_MINIO_SETUP must not be set in production",
                    ));
                }
            }
        }

        Ok(())
    }
}

fn reject_empty(
    name: &str,
    value: &str,
    message: &str,
) -> Result<(), std::io::Error> {
    if value.trim().is_empty() {
        return Err(invalid_config(&format!("{message}: {name} is empty")));
    }
    Ok(())
}

fn validate_https_url(
    name: &str,
    value: &str,
) -> Result<(), std::io::Error> {
    let parsed = url::Url::parse(value).map_err(|_| {
        invalid_config(&format!("{name} must be a valid URL"))
    })?;
    if parsed.scheme() != "https" {
        return Err(invalid_config(&format!(
            "{name} must use https in production"
        )));
    }
    Ok(())
}

fn require_non_zero_u32_env(name: &str) -> Result<(), std::io::Error> {
    let value = std::env::var(name).map_err(|_| {
        invalid_config(&format!("{name} must be configured in production"))
    })?;
    match value.parse::<u32>() {
        Ok(parsed) if parsed > 0 => Ok(()),
        _ => Err(invalid_config(&format!(
            "{name} must be a positive integer in production"
        ))),
    }
}

fn require_non_zero_u64_env(name: &str) -> Result<(), std::io::Error> {
    let value = std::env::var(name).map_err(|_| {
        invalid_config(&format!("{name} must be configured in production"))
    })?;
    match value.parse::<u64>() {
        Ok(parsed) if parsed > 0 => Ok(()),
        _ => Err(invalid_config(&format!(
            "{name} must be a positive integer in production"
        ))),
    }
}

fn is_dangerous_secret(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "dummy-token"
            | "dummy"
            | "test"
            | "secret"
            | "changeme"
            | "placeholder"
    )
}

fn invalid_config(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn valid_production_config() -> Config {
        Config {
            port: 50053,
            environment: "production".to_string(),
            database_url: "mysql://library:secret@tidb.example.com:4000"
                .to_string(),
            property_value_storage_mode: "legacy_only".to_string(),
            cognito_jwk_url: "https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_ProdPool/.well-known/jwks.json".to_string(),
            otel_exporter_otlp_endpoint: None,
            sentry_dsn: None,
            cognito_user_pool_id: "ap-northeast-1_ProdPool".to_string(),
            tachyon_api_url: "https://api.n1.tachy.one".to_string(),
            service_auth_token: "prod-service-token".to_string(),
        }
    }

    fn with_env_lock(test: impl FnOnce()) {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock poisoned");
        for name in [
            "AWS_LAMBDA_FUNCTION_NAME",
            "LIBRARY_PARQUET_BUCKET",
            "MINIO_ENDPOINT",
            "MINIO_PUBLIC_ENDPOINT",
            "MINIO_ROOT_USER",
            "MINIO_ROOT_PASSWORD",
            "SKIP_MINIO_SETUP",
            "DB_POOL_MAX_CONNECTIONS",
            "DB_POOL_ACQUIRE_TIMEOUT_SECS",
        ] {
            std::env::remove_var(name);
        }
        test();
        for name in [
            "AWS_LAMBDA_FUNCTION_NAME",
            "LIBRARY_PARQUET_BUCKET",
            "MINIO_ENDPOINT",
            "MINIO_PUBLIC_ENDPOINT",
            "MINIO_ROOT_USER",
            "MINIO_ROOT_PASSWORD",
            "SKIP_MINIO_SETUP",
            "DB_POOL_MAX_CONNECTIONS",
            "DB_POOL_ACQUIRE_TIMEOUT_SECS",
        ] {
            std::env::remove_var(name);
        }
    }

    fn set_valid_production_env() {
        std::env::set_var(
            "LIBRARY_PARQUET_BUCKET",
            "tachyon-n1-library-parquet-prod",
        );
        std::env::set_var("DB_POOL_MAX_CONNECTIONS", "16");
        std::env::set_var("DB_POOL_ACQUIRE_TIMEOUT_SECS", "10");
    }

    #[test]
    fn production_config_accepts_ga_values() {
        with_env_lock(|| {
            set_valid_production_env();
            let config = valid_production_config();

            assert!(config.validate_for_server_startup().is_ok());
        });
    }

    #[test]
    fn production_config_rejects_dummy_service_token() {
        with_env_lock(|| {
            set_valid_production_env();
            let mut config = valid_production_config();
            config.service_auth_token = "dummy-token".to_string();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_localhost_database_url() {
        with_env_lock(|| {
            set_valid_production_env();
            let mut config = valid_production_config();
            config.database_url =
                "mysql://root:@localhost:15000".to_string();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_empty_database_url() {
        with_env_lock(|| {
            set_valid_production_env();
            let mut config = valid_production_config();
            config.database_url = "  ".to_string();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_missing_parquet_bucket() {
        with_env_lock(|| {
            std::env::set_var("DB_POOL_MAX_CONNECTIONS", "16");
            std::env::set_var("DB_POOL_ACQUIRE_TIMEOUT_SECS", "10");
            let config = valid_production_config();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_minio_settings() {
        with_env_lock(|| {
            set_valid_production_env();
            std::env::set_var("MINIO_ENDPOINT", "http://localhost:9000");
            let config = valid_production_config();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_pages_dev_tachyon_api_url() {
        with_env_lock(|| {
            set_valid_production_env();
            let mut config = valid_production_config();
            config.tachyon_api_url =
                "https://tachyon-api.pages.dev".to_string();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_missing_pool_settings() {
        with_env_lock(|| {
            std::env::set_var(
                "LIBRARY_PARQUET_BUCKET",
                "tachyon-n1-library-parquet-prod",
            );
            let config = valid_production_config();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn production_config_rejects_invalid_pool_settings() {
        with_env_lock(|| {
            std::env::set_var(
                "LIBRARY_PARQUET_BUCKET",
                "tachyon-n1-library-parquet-prod",
            );
            std::env::set_var("DB_POOL_MAX_CONNECTIONS", "0");
            std::env::set_var("DB_POOL_ACQUIRE_TIMEOUT_SECS", "invalid");
            let config = valid_production_config();

            let error = config.validate_for_server_startup().unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        });
    }
}
