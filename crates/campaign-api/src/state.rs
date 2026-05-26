use sqlx::PgPool;
use common::Config;

pub struct AppState {
    pub pg: PgPool,
    pub cfg: Config,
}
