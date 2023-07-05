use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router, Server,
};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

type Snapshot = Vec<f32>;

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<Snapshot>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState {
        broadcast_tx: tx.clone(),
    };

    let public_dir = match env::var("MXTOO_PUBLIC_DIR") {
        Ok(s) => s,
        Err(_) => "public".into(),
    };
    let router = Router::new()
        .nest_service("/", ServeDir::new(public_dir))
        .route("/realtime/cpus", get(realtime_cpus_get))
        .with_state(app_state.clone());

    // Update CPU usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new();
        loop {
            if tx.receiver_count() > 0 {
                sys.refresh_cpu();
                let cpu_data: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
                let _ = tx.send(cpu_data);
                std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
            } else {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    });

    let port: u16 = match env::var("MXTOO_PORT") {
        Ok(s) => s.parse().unwrap_or(7032),
        Err(_) => 7032,
    };
    let bind_addr = format!("0.0.0.0:{port}");

    let server = Server::bind(&bind_addr.parse().unwrap()).serve(router.into_make_service());
    let addr = server.local_addr();
    println!("Listening on {addr}");

    server.await.unwrap();
}

#[derive(Clone)]
struct AppState {
    broadcast_tx: broadcast::Sender<Snapshot>,
}

#[axum::debug_handler]
async fn realtime_cpus_get(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    let mut rx = app_state.broadcast_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        let res = ws
            .send(Message::Text(serde_json::to_string(&msg).unwrap()))
            .await;
        match res {
            Ok(_) => {}
            Err(_) => break,
        }
    }
}
