use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::fingerprint::Fingerprint;
use crate::heartbeat::{spawn_heartbeat_task, HeartbeatState};
use crate::presence::presence_update_payload;

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";
const PRESENCE_UPDATE_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Run the Discord gateway loop, reconnecting as needed.
pub async fn run_gateway(token: String, status: String, fingerprint: Fingerprint) -> Result<()> {
    let mut session_id: Option<String> = None;
    let mut resume_url: Option<String> = None;

    loop {
        let url = resume_url.as_deref().unwrap_or(GATEWAY_URL).to_string();

        match connect_once(&url, &token, &status, &fingerprint, session_id.clone()).await {
            Ok(GatewayExit::Reconnect {
                new_session_id,
                new_resume_url,
            }) => {
                info!("Reconnecting with resume...");
                session_id = new_session_id.or(session_id);
                resume_url = new_resume_url.or(resume_url);

                let delay_ms: u64 = rand::thread_rng().gen_range(1000..=5000);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Ok(GatewayExit::InvalidSession) => {
                info!("Invalid session — re-identifying from scratch...");
                session_id = None;
                resume_url = None;

                let delay_ms: u64 = rand::thread_rng().gen_range(1000..=5000);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(e) => {
                error!("Gateway error: {}. Reconnecting in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

enum GatewayExit {
    Reconnect {
        new_session_id: Option<String>,
        new_resume_url: Option<String>,
    },
    InvalidSession,
}

async fn connect_once(
    url: &str,
    token: &str,
    status: &str,
    fingerprint: &Fingerprint,
    existing_session: Option<String>,
) -> Result<GatewayExit> {
    info!("Connecting to Discord gateway: {}", url);

    let (ws_stream, _) = connect_async(url)
        .await
        .context("WebSocket connect failed")?;

    let (mut ws_sink, mut ws_source) = ws_stream.split();

    // Channel for heartbeat task → gateway (outgoing WS messages).
    let (ws_tx, mut ws_rx) = mpsc::channel::<String>(32);
    // Channel for heartbeat task → gateway (reconnect signal).
    let (reconnect_tx, mut reconnect_rx) = mpsc::channel::<()>(1);

    let hb_state = Arc::new(HeartbeatState::new());

    let mut session_id: Option<String> = None;
    let mut resume_url: Option<String> = None;
    let mut heartbeat_started = false;

    // Spawn a task for periodic presence updates (every 5 minutes).
    let presence_ws_tx = ws_tx.clone();
    let presence_status = status.to_string();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(PRESENCE_UPDATE_INTERVAL_SECS)).await;
            let payload = serde_json::to_string(&presence_update_payload(&presence_status))
                .unwrap_or_default();
            debug!("Sending periodic presence update");
            if presence_ws_tx.send(payload).await.is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            // Forward outgoing messages from background tasks to the WebSocket.
            Some(msg) = ws_rx.recv() => {
                ws_sink.send(Message::Text(msg.into())).await?;
            }

            // Heartbeat task requested a reconnect.
            Some(()) = reconnect_rx.recv() => {
                return Ok(GatewayExit::Reconnect {
                    new_session_id: session_id,
                    new_resume_url: resume_url,
                });
            }

            // Incoming WebSocket message.
            Some(msg_result) = ws_source.next() => {
                let msg = msg_result.context("WebSocket read error")?;

                match msg {
                    Message::Text(text) => {
                        let value: Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                warn!("Failed to parse gateway message: {}", e);
                                continue;
                            }
                        };

                        // Update sequence number for any dispatch event.
                        if let Some(s) = value.get("s").and_then(|v| v.as_i64()) {
                            hb_state.last_seq.store(s, Ordering::SeqCst);
                        }

                        let op = value.get("op").and_then(|v| v.as_u64()).unwrap_or(255);

                        match op {
                            // OP 10: Hello — start heartbeat + identify/resume.
                            10 => {
                                let interval = value["d"]["heartbeat_interval"]
                                    .as_u64()
                                    .unwrap_or(41250);
                                info!("Received Hello, heartbeat_interval={}ms", interval);

                                if !heartbeat_started {
                                    spawn_heartbeat_task(
                                        interval,
                                        Arc::clone(&hb_state),
                                        ws_tx.clone(),
                                        reconnect_tx.clone(),
                                    );
                                    heartbeat_started = true;
                                }

                                // Resume or fresh identify.
                                let payload = if let Some(ref sid) = existing_session {
                                    let seq = hb_state.last_seq.load(Ordering::SeqCst);
                                    let seq_val = if seq < 0 { Value::Null } else { seq.into() };
                                    info!("Resuming session {}", sid);
                                    json!({
                                        "op": 6,
                                        "d": {
                                            "token": token,
                                            "session_id": sid,
                                            "seq": seq_val
                                        }
                                    })
                                } else {
                                    info!("Sending Identify");
                                    build_identify_payload(token, status, fingerprint)
                                };

                                ws_tx
                                    .send(serde_json::to_string(&payload)?)
                                    .await?;
                            }

                            // OP 11: Heartbeat ACK.
                            11 => {
                                debug!("Heartbeat ACK received");
                                hb_state.acked.store(true, Ordering::SeqCst);
                            }

                            // OP 0: Dispatch.
                            0 => {
                                let event_name = value
                                    .get("t")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("UNKNOWN");
                                debug!("Dispatch: {}", event_name);

                                if event_name == "READY" {
                                    if let Some(sid) = value["d"]["session_id"].as_str() {
                                        session_id = Some(sid.to_string());
                                        info!("Session established: {}", sid);
                                    }
                                    if let Some(rurl) = value["d"]["resume_gateway_url"].as_str() {
                                        resume_url = Some(format!(
                                            "{}?v=10&encoding=json",
                                            rurl.trim_end_matches('/')
                                        ));
                                    }
                                    info!("Connected and online");
                                }
                            }

                            // OP 1: Heartbeat request from server.
                            1 => {
                                debug!("Server requested heartbeat");
                                let payload = hb_state.heartbeat_payload();
                                ws_tx
                                    .send(serde_json::to_string(&payload)?)
                                    .await?;
                            }

                            // OP 7: Reconnect.
                            7 => {
                                info!("Received Reconnect (OP 7)");
                                return Ok(GatewayExit::Reconnect {
                                    new_session_id: session_id,
                                    new_resume_url: resume_url,
                                });
                            }

                            // OP 9: Invalid Session.
                            9 => {
                                let resumable = value["d"].as_bool().unwrap_or(false);
                                warn!("Invalid Session (OP 9), resumable={}", resumable);
                                if resumable {
                                    return Ok(GatewayExit::Reconnect {
                                        new_session_id: session_id,
                                        new_resume_url: resume_url,
                                    });
                                } else {
                                    return Ok(GatewayExit::InvalidSession);
                                }
                            }

                            other => {
                                debug!("Unhandled opcode: {}", other);
                            }
                        }
                    }

                    Message::Close(frame) => {
                        warn!("WebSocket closed: {:?}", frame);
                        return Ok(GatewayExit::Reconnect {
                            new_session_id: session_id,
                            new_resume_url: resume_url,
                        });
                    }

                    Message::Ping(data) => {
                        ws_sink.send(Message::Pong(data)).await?;
                    }

                    _ => {}
                }
            }

            else => {
                warn!("WebSocket stream ended unexpectedly");
                return Ok(GatewayExit::Reconnect {
                    new_session_id: session_id,
                    new_resume_url: resume_url,
                });
            }
        }
    }
}

fn build_identify_payload(token: &str, status: &str, fp: &Fingerprint) -> Value {
    json!({
        "op": 2,
        "d": {
            "token": token,
            "capabilities": 8189,
            "properties": fp,
            "presence": {
                "status": status,
                "since": 0,
                "activities": [],
                "afk": false
            },
            "compress": false,
            "client_state": {
                "guild_hashes": {},
                "highest_last_message_id": "0",
                "read_state_version": 0,
                "user_guild_settings_version": -1,
                "user_settings_version": -1
            }
        }
    })
}
