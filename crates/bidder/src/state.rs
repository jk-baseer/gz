use redis::aio::ConnectionManager;
use sqlx::PgPool;
use common::Config;

use crate::index::CampaignIndex;

pub struct AppState {
    pub pg: PgPool,
    pub redis: ConnectionManager,
    pub index: CampaignIndex,
    pub cfg: Config,
}

impl AppState {
    pub fn new(pg: PgPool, redis: ConnectionManager, cfg: Config) -> Self {
        Self {
            pg,
            redis,
            index: CampaignIndex::new(),
            cfg,
        }
    }
}
