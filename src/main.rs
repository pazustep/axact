use std::{convert::Infallible, error::Error, sync::Arc};

use axum::{
    debug_handler,
    extract::State,
    http::header,
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Response, Sse,
    },
    routing::get,
    Router, Server,
};
use futures::Stream;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::watch::{self, Sender};
use tokio_stream::{wrappers::WatchStream, StreamExt};

#[tokio::main]
async fn main() -> Result<(), impl Error> {
    let tx = Arc::new(watch::channel(vec![]).0);
    let state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/index.mjs", get(index_mjs_get))
        .route("/index.css", get(index_css_get))
        .route("/api/cpus", get(cpus_get))
        .with_state(state.clone());

    let server = Server::bind(&"0.0.0.0:7032".parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    // Update CPU stats in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new();

        loop {
            sys.refresh_cpu();
            let v: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            tx.send_replace(v);
            std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    });

    server.await
}

type Snapshot = Vec<f32>;

#[derive(Clone)]
struct AppState {
    // Sender is thread-safe, no need to wrap in Mutex/RwLock
    tx: Arc<Sender<Snapshot>>,
}

#[debug_handler]
async fn root_get() -> impl IntoResponse {
    let str = tokio::fs::read_to_string("src/index.html").await.unwrap();
    Html(str)
}

#[debug_handler]
async fn index_mjs_get() -> impl IntoResponse {
    let str = tokio::fs::read_to_string("src/index.mjs").await.unwrap();
    Response::builder()
        .header(header::CONTENT_TYPE, "application/javascript;charset=utf-8")
        .body(str)
        .unwrap()
}

#[debug_handler]
async fn index_css_get() -> impl IntoResponse {
    let str = tokio::fs::read_to_string("src/index.css").await.unwrap();
    Response::builder()
        .header(header::CONTENT_TYPE, "text/css;charset=utf-8")
        .body(str)
        .unwrap()
}

#[debug_handler]
async fn cpus_get(
    State(AppState { tx }): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = tx.subscribe();

    let stream = WatchStream::new(receiver).map(|snapshot| {
        let data = serde_json::to_string(&snapshot).unwrap();
        Ok(Event::default().data(data))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
