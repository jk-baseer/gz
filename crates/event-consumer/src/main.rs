// Event consumer — reads impression/click/win events from Redpanda (Kafka)
// and writes them to ClickHouse for analytics.
//
// Phase 2 implementation.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);
    tracing::info!("event consumer starting (phase 2 — not yet implemented)");
    Ok(())
}
