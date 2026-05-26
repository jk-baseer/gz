// Reporting service — queries ClickHouse for campaign performance metrics
// and exposes them via a REST API consumed by the dashboard.
//
// Phase 2 implementation.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);
    tracing::info!("reporting service starting (phase 2 — not yet implemented)");
    Ok(())
}
