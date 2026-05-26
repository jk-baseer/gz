use sqlx::PgPool;

pub struct AppState {
    pub pg: PgPool,
}
