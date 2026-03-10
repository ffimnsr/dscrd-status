use serde_json::{json, Value};

/// Build the OP 3 Presence Update payload.
pub fn presence_update_payload(status: &str) -> Value {
    json!({
        "op": 3,
        "d": {
            "since": null,
            "activities": [],
            "status": status,
            "afk": false
        }
    })
}
