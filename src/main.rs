use std::error::Error;

use axum::{
    debug_handler,
    extract::State,
    http::header,
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Response, Sse,
    },
    routing::get,
    BoxError, Router, Server,
};
use futures::{Stream, TryStreamExt};
use sysinfo::{CpuExt, System, SystemExt};
use tokio_stream::wrappers::BroadcastStream;

#[tokio::main]
async fn main() -> Result<(), impl Error> {
    let (sender, _) = tokio::sync::broadcast::channel::<Snapshot>(1);

    let state = AppState {
        sender: sender.clone(),
    };

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
            let _ = sender.send(v);
            std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    });

    server.await
}

type Snapshot = Vec<f32>;

#[derive(Clone)]
struct AppState {
    sender: tokio::sync::broadcast::Sender<Snapshot>,
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
    State(AppState { sender }): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, impl Into<BoxError>>>> {
    let receiver = sender.subscribe();

    let stream = BroadcastStream::new(receiver).map_ok(|snapshot| {
        let data = serde_json::to_string(&snapshot).unwrap();
        Event::default().data(data)
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
