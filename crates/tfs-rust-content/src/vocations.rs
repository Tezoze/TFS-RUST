use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Debug, Clone)]
pub struct Vocation {
    pub id: u16,
    pub client_id: u16,
    pub name: String,
    pub description: String,
    pub from_vocation: u16,
}

#[derive(Debug, Clone)]
pub struct VocationDatabase {
    pub vocations: HashMap<u16, Vocation>,
}

impl VocationDatabase {
    /// `Player::vocation` id → protocol `u8` client id (`ProtocolGame::sendBasicData`).
    pub fn client_id_u8(&self, vocation_id: i32) -> u8 {
        if vocation_id < 0 {
            return 0;
        }
        let id = vocation_id as u16;
        self.vocations
            .get(&id)
            .map(|v| (v.client_id.min(255)) as u8)
            .unwrap_or(0)
    }

    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading vocations from {:?}", path);
        let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut vocations = HashMap::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"vocation" => {
                    let mut id = None;
                    let mut client_id = 0;
                    let mut name = String::new();
                    let mut description = String::new();
                    let mut from_vocation = 0;

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
                                        message: format!("invalid vocation id '{value}': {err}"),
                                    }
                                })?)
                            }
                            b"clientid" => {
                                client_id =
                                    value.parse::<u16>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid clientid '{value}': {err}"),
                                    })?
                            }
                            b"name" => name = value,
                            b"description" => description = value,
                            b"fromvoc" => {
                                from_vocation =
                                    value.parse::<u16>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid fromvoc '{value}': {err}"),
                                    })?
                            }
                            _ => {}
                        }
                    }

                    let vocation_id = id.ok_or_else(|| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: "vocation entry missing required 'id'".to_string(),
                    })?;
                    if name.is_empty() {
                        return Err(TfsRustError::Content {
                            file: path.to_string_lossy().into_owned(),
                            message: format!("vocation {vocation_id} missing required 'name'"),
                        });
                    }

                    vocations.insert(
                        vocation_id,
                        Vocation {
                            id: vocation_id,
                            client_id,
                            name,
                            description,
                            from_vocation,
                        },
                    );
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

        Ok(Self { vocations })
    }
}
