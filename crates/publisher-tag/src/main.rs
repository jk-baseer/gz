// Publisher tag service — serves the JavaScript snippet that publishers paste
// on their sites, and handles direct ad requests from those sites.
//
// Phase 2 implementation (direct publisher integration).

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);
    tracing::info!("publisher-tag service starting (phase 2 — not yet implemented)");
    Ok(())
}
