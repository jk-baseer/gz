use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init(log_level: &str) {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| log_level.into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
