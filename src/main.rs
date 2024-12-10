use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
};
use tokio::time::Instant;

const DEFAULT_REFILL_PERIOD_SECONDS: usize = 60;
const DEFAULT_CAPACITY: usize = 100;

#[tokio::main]
async fn main() {
    println!("Starting rtlmtr...");
    let store = RwLock::new(HashMap::<String, Bucket>::default());

    let refill_period_ms =
        env::var("REFILL_PERIOD_SECONDS").map_or(DEFAULT_REFILL_PERIOD_SECONDS, |v| {
            v.parse::<usize>().unwrap()
        }) * 1000;
    let capacity = env::var("CAPACITY").map_or(DEFAULT_CAPACITY, |v| v.parse::<usize>().unwrap());

    let state = Arc::new(AppState {
        store,
        refill_period_ms,
        capacity,
    });

    let app = Router::new()
        .route("/:id", get(handle_request))
        .with_state(Arc::clone(&state));

    let port = env::var("PORT").unwrap_or("3000".to_string());

    let listener = tokio::net::TcpListener::bind(format!("[::]:{port}"))
        .await
        .unwrap();

    let addr = listener.local_addr().unwrap();
    println!("Listening on {addr}");

    axum::serve(listener, app).await.unwrap();
}

struct Bucket {
    tokens: usize,
    last_refill: Instant,
}

struct AppState {
    store: RwLock<HashMap<String, Bucket>>,
    refill_period_ms: usize,
    capacity: usize,
}

type SharedState = Arc<AppState>;

async fn handle_request(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let now = Instant::now();
    let capacity = state.capacity;

    let tokens = {
        let store = state.store.read().unwrap();

        let tokens = store.get(&key).map_or(state.capacity, |bucket| {
            let elapsed = now - bucket.last_refill;
            let refill = capacity * (elapsed.as_millis() as usize) / (state.refill_period_ms);

            usize::min(bucket.tokens + refill, state.capacity)
        });

        tokens
    };

    if tokens == 0 {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("x-ratelimit-limit", capacity.to_string()),
                ("x-ratelimit-remaining", String::from("0")),
            ],
        );
    }

    {
        let mut write_store = state.store.write().unwrap();

        write_store.insert(
            key,
            Bucket {
                tokens: tokens - 1,
                last_refill: now,
            },
        );
    }

    (
        StatusCode::OK,
        [
            ("x-ratelimit-limit", capacity.to_string()),
            ("x-ratelimit-remaining", (tokens - 1).to_string()),
        ],
    )
}
