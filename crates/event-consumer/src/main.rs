/// Event consumer — reads impression/click/win events from Redpanda (Kafka)
/// and writes them to ClickHouse for analytics.
///
/// Phase 1: events are written directly to PostgreSQL by the bidder.
/// Phase 2: bidder publishes to Redpanda topics; this service consumes them
/// and writes to ClickHouse for high-volume analytics at scale.
///
/// Topics (Phase 2):
///   gz.impressions  — win notices with clearing price + context
///   gz.clicks       — click events
///   gz.views        — impression pixel fires (browser render confirmation)

use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);

    info!("event-consumer: Phase 1 — bidder writes directly to PostgreSQL");
    info!("event-consumer: Phase 2 — will consume Redpanda → ClickHouse");
    info!("Redpanda topics: gz.impressions, gz.clicks, gz.views");
    info!("ClickHouse target: default.impression_events, default.click_events");

    // Phase 2 implementation:
    // 1. Connect to Redpanda with rdkafka
    // 2. Subscribe to gz.impressions, gz.clicks, gz.views
    // 3. Batch insert into ClickHouse every 1000 events or 1 second
    // 4. Commit Kafka offsets after successful ClickHouse write

    Ok(())
}
