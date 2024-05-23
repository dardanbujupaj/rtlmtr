use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tokio::time::Instant;

const DEFAULT_REFILL_PERIOD: usize = 10;
const DEFAULT_CAPACITY: usize = 10;

#[tokio::main]
async fn main() {
    let store = RwLock::new(HashMap::<String, Bucket>::default());

    let refill_period =
        env::var("REFILL_PERIOD").map_or(DEFAULT_REFILL_PERIOD, |v| v.parse::<usize>().unwrap());
    let capacity = env::var("CAPACITY").map_or(DEFAULT_CAPACITY, |v| v.parse::<usize>().unwrap());

    let state = Arc::new(AppState {
        store,
        refill_period,
        capacity,
    });

    let app = Router::new()
        .route("/:id", get(handle_request))
        .with_state(Arc::clone(&state));

    let port = env::var("PORT").unwrap_or("3000".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

struct Bucket {
    tokens: usize,
    last_refill: Instant,
}

struct AppState {
    store: RwLock<HashMap<String, Bucket>>,
    refill_period: usize,
    capacity: usize,
}

type SharedState = Arc<AppState>;

async fn handle_request(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let now = Instant::now();
    let mut store = state.store.write().unwrap();
    let tokens = store.get(&key).map_or(state.capacity, |bucket| {
        let elapsed = now - bucket.last_refill;
        let refill = state.capacity * (elapsed.as_secs() as usize) / state.refill_period;

        usize::min(bucket.tokens + refill, state.capacity)
    });

    if tokens > 0 {
        store.insert(
            key,
            Bucket {
                tokens: tokens - 1,
                last_refill: now,
            },
        );
    }

    (
        match tokens {
            0 => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::OK,
        },
        [
            ("x-ratelimit-limit", state.capacity.to_string()),
            ("x-ratelimit-remaining", (tokens - 1).to_string()),
        ],
    )
}
