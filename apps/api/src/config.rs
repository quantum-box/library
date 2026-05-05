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
            if self.service_auth_token.trim().is_empty()
                || self.service_auth_token == "dummy-token"
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
        }

        Ok(())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config").finish()
    }
}
