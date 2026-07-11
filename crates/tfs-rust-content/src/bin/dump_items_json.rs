//! Dump merged ItemType (OTB + items.xml) as JSON Lines for offline auditing.
//!
//! Usage (repo root):
//!   cargo run -p tfs-rust-content --bin dump-items-json > /tmp/items.jsonl
//!
//! Each line is one item: `{"server_id":102,"client_id":102,"group":1,"flags":64,...}`

use std::collections::BTreeMap;
use std::path::PathBuf;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::otb::ItemType;

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn dump_item(item: &ItemType) -> String {
    // Sort xml_attributes by key for deterministic output.
    let xml: BTreeMap<&String, &String> = item.xml_attributes.iter().collect();

    let mut xml_parts = Vec::new();
    for (k, v) in &xml {
        xml_parts.push(format!("\"{}\":\"{}\"", json_escape(k), json_escape(v)));
    }

    format!(
        r#"{{"server_id":{},"client_id":{},"group":{},"flags":{},"speed":{},"weight":{},"rotate_to":{},"light_level":{},"light_color":{},"max_items":{},"weapon_type":{},"attack":{},"defense":{},"extra_defense":{},"armor":{},"ammo_type":{},"shoot_range":{},"type_tag":{},"charges":{},"min_req_level":{},"min_req_magic_level":{},"slot_position":{},"block_solid_override":{},"moveable_override":{},"block_projectile_override":{},"can_read_text_override":{},"can_write_text":{},"max_text_len":{},"allow_pickupable":{},"force_serialize":{},"replaceable":{},"walk_stack":{},"show_count":{},"show_charges":{},"show_attributes":{},"always_on_top_order":{},"floor_change":{},"name":"{}","xml_attributes":{{{}}}}}"#,
        item.server_id,
        item.client_id,
        item.group,
        item.flags,
        item.speed,
        item.weight,
        item.rotate_to,
        item.light_level,
        item.light_color,
        item.max_items,
        item.weapon_type,
        item.attack,
        item.defense,
        item.extra_defense,
        item.armor,
        item.ammo_type,
        item.shoot_range,
        item.type_tag,
        item.charges,
        item.min_req_level,
        item.min_req_magic_level,
        item.slot_position,
        match item.block_solid_override {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        },
        match item.moveable_override {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        },
        match item.block_projectile_override {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        },
        match item.can_read_text_override {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        },
        item.can_write_text,
        item.max_text_len,
        item.allow_pickupable,
        item.force_serialize,
        item.replaceable,
        item.walk_stack,
        item.show_count,
        item.show_charges,
        item.show_attributes,
        item.always_on_top_order,
        item.floor_change,
        json_escape(&item.name),
        xml_parts.join(","),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let otb = repo_root.join("data/items/items.otb");
    let xml = repo_root.join("data/items/items.xml");

    let db = ItemDatabase::load(&otb, &xml)?;

    let mut ids: Vec<u16> = db.items.keys().copied().collect();
    ids.sort();

    for id in ids {
        let item = &db.items[&id];
        println!("{}", dump_item(item));
    }

    eprintln!("dumped {} items", db.items.len());
    Ok(())
}
