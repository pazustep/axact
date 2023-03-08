use std::{error::Error, sync::Arc};

use axum::{
    debug_handler,
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    routing::get,
    BoxError, Router, Server,
};
use futures::Stream;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::watch::{self, Sender};
use tokio_stream::{wrappers::WatchStream, StreamExt};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), impl Error> {
    let tx = Arc::new(watch::channel(vec![]).0);
    let state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/api/cpus", get(cpus_get))
        .fallback_service(ServeDir::new("assets"))
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
async fn cpus_get(
    State(AppState { tx }): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, impl Into<BoxError>>>> {
    let receiver = tx.subscribe();
    let stream = WatchStream::new(receiver).map(|snapshot| Event::default().json_data(snapshot));
    Sse::new(stream).keep_alive(KeepAlive::default())
}
