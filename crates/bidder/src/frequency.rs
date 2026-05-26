/// Redis-based frequency capping.
/// Prevents a single user from seeing the same campaign more than N times per day.
/// Key: freq:{campaign_id}:{user_id}:{date}  →  impression count (TTL 25h)
use chrono::Utc;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Returns true if the impression is allowed (under cap), and increments the counter.
/// If user_id is empty (unknown), frequency capping is skipped.
pub async fn check_and_record(
    redis: &mut ConnectionManager,
    campaign_id: Uuid,
    user_id: &str,
    cap: i32,
) -> anyhow::Result<bool> {
    if user_id.is_empty() {
        return Ok(true);
    }

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let key = format!("freq:{campaign_id}:{user_id}:{today}");

    let script = redis::Script::new(
        r#"
        local current = tonumber(redis.call('GET', KEYS[1]) or 0)
        local cap     = tonumber(ARGV[1])
        if current >= cap then return 0 end
        redis.call('INCR',   KEYS[1])
        redis.call('EXPIRE', KEYS[1], 90000)  -- 25h TTL
        return 1
        "#,
    );

    let allowed: i64 = script
        .key(&key)
        .arg(cap)
        .invoke_async(redis)
        .await?;

    Ok(allowed == 1)
}
