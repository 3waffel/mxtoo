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
use serde::Serialize;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

type Snapshot = WsData;

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
        .route("/realtime/data", get(realtime_data_get))
        .with_state(app_state.clone());

    // Update CPU usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new();
        loop {
            if tx.receiver_count() > 0 {
                sys.refresh_cpu();
                sys.refresh_memory();

                let cpu_data: Vec<_> = sys
                    .cpus()
                    .iter()
                    .enumerate()
                    .map(|cpu| (cpu.0 as u32, cpu.1.cpu_usage()))
                    .collect();
                let mem_data: MemoryData = MemoryData::new(
                    sys.total_memory(),
                    sys.free_memory(),
                    sys.available_memory(),
                    sys.used_memory(),
                );
                let data = WsData::new(cpu_data, mem_data);

                let _ = tx.send(data);
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
async fn realtime_data_get(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_data_stream(state, ws).await })
}

async fn realtime_data_stream(app_state: AppState, mut ws: WebSocket) {
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

#[derive(Clone, Debug, Serialize)]
pub struct MemoryData {
    total: u64,
    free: u64,
    available: u64,
    used: u64,
}

impl MemoryData {
    pub fn new(total: u64, free: u64, available: u64, used: u64) -> Self {
        Self {
            total,
            free,
            available,
            used,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct WsData {
    cpu_data: Vec<(u32, f32)>,
    mem_data: MemoryData,
}

impl WsData {
    pub fn new(cpu_data: Vec<(u32, f32)>, mem_data: MemoryData) -> Self {
        Self { cpu_data, mem_data }
    }
}
