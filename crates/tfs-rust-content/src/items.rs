use crate::otb::{ItemType, OtbLoader};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Clone)]
pub struct ItemDatabase {
    pub items: HashMap<u16, ItemType>,
    /// C++ `clientIdToServerIdMap` — first server id wins for duplicate client ids (`src/items.cpp` `loadFromOtb`).
    pub client_to_server: HashMap<u16, u16>,
}

impl ItemDatabase {
    /// OTB / `ItemType::clientId` for map and inventory protocol (`addItem`); 0 if unknown.
    // C++: `Items::getItemType(serverId).clientId` — `src/items.cpp`
    #[inline]
    pub fn client_id_for_server(&self, server_id: u16) -> u16 {
        self.items
            .get(&server_id)
            .map(|t| t.client_id)
            .unwrap_or(0)
    }

    /// Reverse lookup: OT client sprite id → server item id (`Items::getServerId` patterns / `clientIdToServerIdMap`).
    #[inline]
    pub fn server_id_for_client(&self, client_id: u16) -> Option<u16> {
        self.client_to_server.get(&client_id).copied()
    }

    /// OTB `FLAG_ANIMATION` — extra `0xFE` before duration in `addItem` (`src/networkmessage.cpp`).
    #[inline]
    pub fn is_animation_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.is_animation())
    }

    /// OTB `FLAG_STACKABLE` — `ItemType::stackable` (`src/items.cpp`).
    #[inline]
    pub fn stackable_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.stackable())
    }

    /// Splash / fluid container group — `NetworkMessage::addItem` fluid byte (`src/networkmessage.cpp`).
    #[inline]
    pub fn is_splash_or_fluid_for_server(&self, server_id: u16) -> bool {
        self.items
            .get(&server_id)
            .is_some_and(|t| t.is_splash() || t.is_fluid_container())
    }

    /// Whether this item behaves as a container for loot nesting (`loadLootContainer` in TFS).
    /// C++ uses `ItemType::isContainer()` (OTB group); we approximate via merged `items.xml`
    /// `containerSize` / `type=container` when OTB group is not decoded yet.
    pub fn is_container(&self, id: u16) -> bool {
        self.items.get(&id).is_some_and(|t| {
            t.xml_attributes
                .keys()
                .any(|k| k.eq_ignore_ascii_case("containersize"))
                || t.xml_attributes
                    .get("type")
                    .map(|v| v.eq_ignore_ascii_case("container"))
                    .unwrap_or(false)
        })
    }

    /// Default `subType` for loot when omitted: C++ uses `ItemType::charges`.
    pub fn charges_default(&self, id: u16) -> i32 {
        self.items
            .get(&id)
            .and_then(|t| t.xml_attributes.get("charges"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Resolve `name="..."` loot references; errors if unknown or ambiguous (see `monsters.cpp` `loadLootItem`).
    pub fn item_id_by_exact_name(&self, name: &str, file: &str) -> Result<u16> {
        let lower = name.to_ascii_lowercase();
        let mut matches: Vec<u16> = self
            .items
            .iter()
            .filter(|(_, it)| !it.name.is_empty() && it.name.to_ascii_lowercase() == lower)
            .map(|(&id, _)| id)
            .collect();
        matches.sort_unstable();
        match matches.len() {
            0 => Err(TfsRustError::Content {
                file: file.to_string(),
                message: format!("unknown loot item name \"{name}\""),
            }),
            1 => Ok(matches[0]),
            _ => Err(TfsRustError::Content {
                file: file.to_string(),
                message: format!("non-unique loot item name \"{name}\""),
            }),
        }
    }

    pub fn load(otb_path: &Path, xml_path: &Path) -> Result<Self> {
        info!("Loading OTB from {:?}", otb_path);
        let mut items = OtbLoader::load_from_file(otb_path)?;

        info!("Merging items.xml from {:?}", xml_path);
        Self::merge_xml(&mut items, xml_path)?;

        let client_to_server = Self::build_client_to_server_map(&items);
        Ok(Self {
            items,
            client_to_server,
        })
    }

    fn build_client_to_server_map(items: &HashMap<u16, ItemType>) -> HashMap<u16, u16> {
        let mut m = HashMap::new();
        for (&sid, it) in items {
            if it.client_id != 0 {
                m.entry(it.client_id).or_insert(sid);
            }
        }
        m
    }

    fn merge_xml(items: &mut HashMap<u16, ItemType>, xml_path: &Path) -> Result<()> {
        let xml_str = std::fs::read_to_string(xml_path).map_err(|e| TfsRustError::Content {
            file: xml_path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml_str);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut current_ids: Vec<u16> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"item" => {
                    current_ids.clear();
                    let mut id: Option<u16> = None;
                    let mut from_id: Option<u16> = None;
                    let mut to_id: Option<u16> = None;
                    let mut name: Option<String> = None;

                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: xml_path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        let key = attr.key.as_ref();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match key {
                            b"id" => {
                                id = Some(parse_u16_attr(xml_path, "id", &value)?);
                            }
                            b"fromid" => {
                                from_id = Some(parse_u16_attr(xml_path, "fromid", &value)?);
                            }
                            b"toid" => {
                                to_id = Some(parse_u16_attr(xml_path, "toid", &value)?);
                            }
                            b"name" => name = Some(value),
                            _ => {}
                        }
                    }

                    if let Some(single) = id {
                        current_ids.push(single);
                    } else if let (Some(start), Some(end)) = (from_id, to_id) {
                        if start > end {
                            return Err(TfsRustError::Content {
                                file: xml_path.to_string_lossy().into_owned(),
                                message: format!("invalid item range: fromid {start} > toid {end}"),
                            });
                        }
                        current_ids.extend(start..=end);
                    } else {
                        return Err(TfsRustError::Content {
                            file: xml_path.to_string_lossy().into_owned(),
                            message: "item entry missing required id/fromid+toid".to_string(),
                        });
                    }

                    if let Some(name) = name {
                        for id in &current_ids {
                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                id: *id,
                                server_id: *id,
                                ..ItemType::default()
                            });
                            entry.name = name.clone();
                        }
                    }
                }
                Ok(Event::Empty(e)) if e.name().as_ref() == b"attribute" => {
                    if current_ids.is_empty() {
                        buf.clear();
                        continue;
                    }
                    let mut key: Option<String> = None;
                    let mut value: Option<String> = None;
                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: xml_path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        match attr.key.as_ref() {
                            b"key" => {
                                key =
                                    Some(String::from_utf8_lossy(attr.value.as_ref()).into_owned())
                            }
                            b"value" => {
                                value =
                                    Some(String::from_utf8_lossy(attr.value.as_ref()).into_owned())
                            }
                            _ => {}
                        }
                    }

                    if let (Some(key), Some(value)) = (key, value) {
                        for id in &current_ids {
                            let entry = items.entry(*id).or_insert_with(|| ItemType {
                                id: *id,
                                server_id: *id,
                                ..ItemType::default()
                            });
                            apply_xml_attribute(entry, &key, &value);
                        }
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"item" => {
                    current_ids.clear();
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(TfsRustError::Content {
                        file: xml_path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }
}

fn parse_u16_attr(path: &Path, name: &str, value: &str) -> Result<u16> {
    value.parse::<u16>().map_err(|err| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: format!("invalid {name} '{value}': {err}"),
    })
}

fn apply_xml_attribute(item: &mut ItemType, key: &str, value: &str) {
    let k = key.to_ascii_lowercase();
    item.xml_attributes.insert(k.clone(), value.to_string());
    match k.as_str() {
        "description" => item.description = value.to_string(),
        "weight" => {
            if let Ok(v) = value.parse::<u32>() {
                item.weight = v;
            }
        }
        "rotateto" => {
            if let Ok(v) = value.parse::<u16>() {
                item.rotate_to = v;
            }
        }
        _ => {}
    }
}
