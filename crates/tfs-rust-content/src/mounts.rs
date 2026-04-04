use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Debug, Clone)]
pub struct Mount {
    pub id: u16,
    pub client_id: u16,
    pub name: String,
    pub speed: i16,
    pub premium: bool,
}

pub struct MountDatabase {
    pub mounts: HashMap<u16, Mount>,
}

impl MountDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading mounts from {:?}", path);
        let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut mounts = HashMap::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"mount" => {
                    let mut id = None;
                    let mut client_id = 0u16;
                    let mut name = String::new();
                    let mut speed = 0i16;
                    let mut premium = false;

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
                                        message: format!("invalid mount id '{value}': {err}"),
                                    }
                                })?)
                            }
                            b"clientid" => {
                                client_id =
                                    value.parse::<u16>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid mount clientid '{value}': {err}"),
                                    })?
                            }
                            b"name" => name = value,
                            b"speed" => speed = value.parse::<i16>().unwrap_or(0),
                            b"premium" => premium = matches!(value.as_str(), "yes" | "true" | "1"),
                            _ => {}
                        }
                    }

                    let id = id.ok_or_else(|| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: "mount entry missing required 'id'".to_string(),
                    })?;

                    mounts.insert(
                        id,
                        Mount {
                            id,
                            client_id,
                            name,
                            speed,
                            premium,
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

        Ok(Self { mounts })
    }
}
