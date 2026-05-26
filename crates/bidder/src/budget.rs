use chrono::Utc;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Atomically checks whether the campaign has budget remaining and, if so,
/// reserves the bid amount. Returns true if the bid is approved.
///
/// Uses a Lua script to make the check-and-increment atomic, preventing
/// overspend under concurrent bidding.
pub async fn try_reserve(
    redis: &mut ConnectionManager,
    campaign_id: Uuid,
    bid_cents: i64,
    daily_limit_cents: Option<i64>,
    total_limit_cents: i64,
) -> anyhow::Result<bool> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let daily_key = format!("spend:daily:{campaign_id}:{today}");
    let total_key = format!("spend:total:{campaign_id}");

    // Lua: check both limits atomically, increment only if within budget
    let script = redis::Script::new(
        r#"
        local daily_key   = KEYS[1]
        local total_key   = KEYS[2]
        local amount      = tonumber(ARGV[1])
        local daily_limit = tonumber(ARGV[2])  -- 0 means no daily limit
        local total_limit = tonumber(ARGV[3])

        local daily_spend = tonumber(redis.call('GET', daily_key) or 0)
        local total_spend = tonumber(redis.call('GET', total_key) or 0)

        if daily_limit > 0 and (daily_spend + amount) > daily_limit then return 0 end
        if (total_spend + amount) > total_limit then return 0 end

        redis.call('INCRBY', daily_key, amount)
        redis.call('EXPIRE', daily_key, 86400)
        redis.call('INCRBY', total_key, amount)
        return 1
        "#,
    );

    let approved: i64 = script
        .key(&daily_key)
        .key(&total_key)
        .arg(bid_cents)
        .arg(daily_limit_cents.unwrap_or(0))
        .arg(total_limit_cents)
        .invoke_async(redis)
        .await?;

    Ok(approved == 1)
}

/// Adjusts the reserved spend to the actual clearing price after a win notice.
/// The exchange clears at or below our bid price; we correct the delta in Redis.
pub async fn record_win(
    redis: &mut ConnectionManager,
    campaign_id: Uuid,
    clearing_cents: i64,
    bid_cents: i64,
) -> anyhow::Result<()> {
    let delta = clearing_cents - bid_cents;
    if delta == 0 {
        return Ok(());
    }

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let daily_key = format!("spend:daily:{campaign_id}:{today}");
    let total_key = format!("spend:total:{campaign_id}");

    // Adjust both keys by the delta (negative delta = refund, positive = extra charge)
    let script = redis::Script::new(
        r#"
        redis.call('INCRBY', KEYS[1], ARGV[1])
        redis.call('INCRBY', KEYS[2], ARGV[1])
        return 1
        "#,
    );

    let _: i64 = script
        .key(&daily_key)
        .key(&total_key)
        .arg(delta)
        .invoke_async(redis)
        .await?;

    Ok(())
}
