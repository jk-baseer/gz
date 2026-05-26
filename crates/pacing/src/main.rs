// Pacing service — smooths budget spend across the campaign flight dates.
// Each minute: computes remaining budget / remaining minutes and writes the
// per-minute spend cap to Redis. The bidder reads this cap before placing bids.
//
// Phase 2 implementation.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);
    tracing::info!("pacing service starting (phase 2 — not yet implemented)");
    Ok(())
}
