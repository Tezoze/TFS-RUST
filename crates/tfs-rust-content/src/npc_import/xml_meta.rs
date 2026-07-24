//! Optional XML metadata overlay for split `data/npc/*.xml` + behavior mode.

use std::collections::HashMap;
use std::path::Path;

use crate::npc_import::error::{ImportError, ImportResult};
use crate::npcs::{NpcAppearance, NpcMovement, PendingNpcDefinition};

/// Parsed NPC XML metadata (TFS split layout).
#[derive(Debug, Clone, Default)]
pub struct XmlNpcMeta {
    pub name: String,
    pub behavior: Option<String>,
    pub script: Option<String>,
    pub appearance: NpcAppearance,
    pub movement: NpcMovement,
    pub parameters: HashMap<String, String>,
}

/// Load all `*.xml` NPC definitions under `npc_dir` (non-recursive except ignoring scripts/).
pub fn load_xml_npc_dir(npc_dir: &Path) -> ImportResult<Vec<XmlNpcMeta>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(npc_dir).map_err(|e| ImportError::io(npc_dir, e.to_string()))?;
    for ent in entries {
        let ent = ent.map_err(|e| ImportError::io(npc_dir, e.to_string()))?;
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        out.push(parse_npc_xml(&path)?);
    }
    out.sort_by_key(|a| a.name.to_ascii_lowercase());
    Ok(out)
}

/// Apply XML look/walk onto a pending definition (split import mode).
pub fn apply_xml_meta(pending: &mut PendingNpcDefinition, meta: &XmlNpcMeta) {
    pending.appearance = meta.appearance.clone();
    pending.movement.radius = meta.movement.radius;
    if meta.movement.speed != 0 {
        pending.movement.speed = meta.movement.speed;
    }
    pending.parameters.extend(meta.parameters.clone());
}

fn parse_npc_xml(path: &Path) -> ImportResult<XmlNpcMeta> {
    let text = std::fs::read_to_string(path).map_err(|e| ImportError::io(path, e.to_string()))?;
    // Prefer UTF-8; XML often declares iso-8859-1 — fall back via byte decode.
    let text = if text.contains('\u{FFFD}') {
        let bytes = std::fs::read(path).map_err(|e| ImportError::io(path, e.to_string()))?;
        crate::npc_import::decode::decode_npc_bytes(&bytes)
    } else {
        text
    };

    let doc = roxmltree::Document::parse(&text).map_err(|e| {
        ImportError::io(path, format!("xml parse error: {e}"))
    })?;
    let root = doc.root_element();
    if root.tag_name().name() != "npc" {
        return Err(ImportError::io(path, "root element must be <npc>"));
    }

    let mut meta = XmlNpcMeta {
        name: root.attribute("name").unwrap_or("").to_string(),
        behavior: root.attribute("behavior").map(str::to_string),
        script: root.attribute("script").map(str::to_string),
        ..Default::default()
    };
    if let Some(r) = root.attribute("walkradius") {
        meta.movement.radius = r.parse().unwrap_or(0);
    }
    if let Some(s) = root.attribute("speed") {
        meta.movement.speed = s.parse().unwrap_or(100);
    }

    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "look" => {
                meta.appearance.look_type = attr_u16(child, "type").unwrap_or(136);
                meta.appearance.look_head = attr_u8(child, "head").unwrap_or(0);
                meta.appearance.look_body = attr_u8(child, "body").unwrap_or(0);
                meta.appearance.look_legs = attr_u8(child, "legs").unwrap_or(0);
                meta.appearance.look_feet = attr_u8(child, "feet").unwrap_or(0);
                meta.appearance.look_addons = attr_u8(child, "addons").unwrap_or(0);
            }
            "parameter" => {
                if let (Some(k), Some(v)) = (child.attribute("key"), child.attribute("value")) {
                    meta.parameters.insert(k.to_string(), v.to_string());
                }
            }
            _ => {}
        }
    }

    if meta.name.is_empty() {
        return Err(ImportError::io(path, "npc name attribute missing"));
    }
    Ok(meta)
}

fn attr_u16(node: roxmltree::Node<'_, '_>, name: &str) -> Option<u16> {
    node.attribute(name)?.parse().ok()
}

fn attr_u8(node: roxmltree::Node<'_, '_>, name: &str) -> Option<u8> {
    node.attribute(name)?.parse().ok()
}
