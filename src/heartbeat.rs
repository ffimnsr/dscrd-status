use anyhow::Result;
use rand::Rng;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Shared heartbeat state between the heartbeat task and the main gateway loop.
pub struct HeartbeatState {
    /// The last acknowledged sequence number (or -1 for null).
    pub last_seq: AtomicI64,
    /// Whether the last heartbeat was acknowledged by the server.
    pub acked: AtomicBool,
}

impl HeartbeatState {
    pub fn new() -> Self {
        Self {
            last_seq: AtomicI64::new(-1),
            acked: AtomicBool::new(true),
        }
    }

    /// Build OP 1 heartbeat payload using the current sequence number.
    pub fn heartbeat_payload(&self) -> Value {
        let seq = self.last_seq.load(Ordering::SeqCst);
        let d = if seq < 0 {
            Value::Null
        } else {
            Value::Number(seq.into())
        };
        json!({ "op": 1, "d": d })
    }
}

impl Default for HeartbeatState {
    fn default() -> Self {
        Self::new()
    }
}

/// Spawn the heartbeat loop task.
///
/// Returns a channel sender that, when dropped or sent `true`, triggers a
/// reconnect signal back to the gateway loop.
pub fn spawn_heartbeat_task(
    interval_ms: u64,
    state: Arc<HeartbeatState>,
    ws_tx: mpsc::Sender<String>,
    reconnect_tx: mpsc::Sender<()>,
) {
    tokio::spawn(async move {
        if let Err(e) = heartbeat_loop(interval_ms, state, ws_tx, reconnect_tx).await {
            warn!("Heartbeat loop exited: {}", e);
        }
    });
}

async fn heartbeat_loop(
    interval_ms: u64,
    state: Arc<HeartbeatState>,
    ws_tx: mpsc::Sender<String>,
    reconnect_tx: mpsc::Sender<()>,
) -> Result<()> {
    loop {
        // Add ±500 ms jitter to appear more human.
        // The rng is created and dropped before the await to satisfy Send bounds.
        let sleep_ms = {
            let jitter_ms: i64 = rand::thread_rng().gen_range(-500..=500);
            (interval_ms as i64 + jitter_ms).max(1000) as u64
        };

        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

        // Check that the previous heartbeat was acknowledged.
        if !state.acked.swap(false, Ordering::SeqCst) {
            warn!("Heartbeat ACK not received — triggering reconnect");
            let _ = reconnect_tx.send(()).await;
            return Ok(());
        }

        let payload = serde_json::to_string(&state.heartbeat_payload())?;
        debug!("Sending heartbeat: {}", payload);

        if ws_tx.send(payload).await.is_err() {
            // WebSocket sender dropped — gateway loop is shutting down.
            return Ok(());
        }
    }
}
