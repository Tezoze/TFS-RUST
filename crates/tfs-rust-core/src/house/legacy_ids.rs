//! Pre-cutover server id → client id for house `tile_store` rows.
//!
//! House blobs saved before the catalog flip still store the old server id.
//! `1638` was a passthrough there and is a closed door in the client-id catalog.

use std::collections::HashMap;
use std::sync::OnceLock;

const PAIRS_SQL: &str = include_str!(
    "../../../tfs-rust-db/migrations/20260926000000_client_item_ids.sql"
);

fn map() -> &'static HashMap<u16, u16> {
    static MAP: OnceLock<HashMap<u16, u16>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut out = HashMap::new();
        for line in PAIRS_SQL.lines() {
            let Some(rest) = line.trim().strip_prefix("WHEN ") else {
                continue;
            };
            let Some((old, new)) = rest.split_once(" THEN ") else {
                continue;
            };
            let Ok(old) = old.parse::<u16>() else {
                continue;
            };
            let Ok(new) = new.trim().parse::<u16>() else {
                continue;
            };
            out.insert(old, new);
        }
        out
    })
}

/// Client id of a pre-cutover server id. `None` when `id` was not remapped.
pub(crate) fn client_id(server_id: u16) -> Option<u16> {
    map().get(&server_id).copied()
}
