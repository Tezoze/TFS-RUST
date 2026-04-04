use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Debug, Clone)]
pub struct Group {
    pub id: u16,
    pub name: String,
    pub access: bool,
    pub max_depot_items: u32,
    pub max_vip_entries: u32,
    pub flags: HashMap<String, bool>,
}

pub struct GroupDatabase {
    pub groups: HashMap<u16, Group>,
}

impl GroupDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading groups from {:?}", path);
        let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut groups = HashMap::new();
        let mut current_group_id: Option<u16> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) if e.name().as_ref() == b"group" => {
                    let mut id = None;
                    let mut name = String::new();
                    let mut access = false;
                    let mut max_depot_items = 0u32;
                    let mut max_vip_entries = 0u32;

                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        let key = attr.key.as_ref();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match key {
                            b"id" => {
                                id = Some(value.parse::<u16>().map_err(|err| {
                                    TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid group id '{value}': {err}"),
                                    }
                                })?)
                            }
                            b"name" => name = value,
                            b"access" => access = matches!(value.as_str(), "1" | "yes" | "true"),
                            b"maxdepotitems" => max_depot_items = value.parse::<u32>().unwrap_or(0),
                            b"maxvipentries" => max_vip_entries = value.parse::<u32>().unwrap_or(0),
                            _ => {}
                        }
                    }

                    let id = id.ok_or_else(|| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: "group entry missing required 'id'".to_string(),
                    })?;
                    groups.insert(
                        id,
                        Group {
                            id,
                            name,
                            access,
                            max_depot_items,
                            max_vip_entries,
                            flags: HashMap::new(),
                        },
                    );
                    current_group_id = Some(id);
                }
                Ok(Event::Empty(e)) if e.name().as_ref() == b"flag" => {
                    let Some(group_id) = current_group_id else {
                        buf.clear();
                        continue;
                    };
                    let mut key_name = None;
                    let mut enabled = false;
                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                        if key != "key" {
                            key_name = Some(key);
                            enabled = matches!(value.as_str(), "1" | "yes" | "true");
                        }
                    }
                    if let Some(key_name) = key_name {
                        if let Some(group) = groups.get_mut(&group_id) {
                            group.flags.insert(key_name, enabled);
                        }
                    }
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"group" => {
                    current_group_id = None;
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(Self { groups })
    }
}
