use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bidder_host: String,
    pub bidder_port: u16,
    pub api_host: String,
    pub api_port: u16,
    /// Public hostname for win notice URLs (e.g. "bidder.gz-ads.com")
    pub public_hostname: String,
    pub log_level: String,
    pub jwt_secret: String,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let cfg = config::Config::builder()
            .set_default("bidder_host", "0.0.0.0")?
            .set_default("bidder_port", 8080u16)?
            .set_default("api_host", "0.0.0.0")?
            .set_default("api_port", 8081u16)?
            .set_default("public_hostname", "localhost:8080")?
            .set_default("log_level", "info")?
            .set_default("jwt_secret", "change-me-in-production")?
            .set_default("stripe_secret_key", "")?
            .set_default("stripe_webhook_secret", "")?
            .add_source(config::Environment::default())
            .build()?;

        Ok(cfg.try_deserialize()?)
    }

    pub fn bidder_addr(&self) -> String {
        format!("{}:{}", self.bidder_host, self.bidder_port)
    }

    pub fn api_addr(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }
}
