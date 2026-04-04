use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

#[derive(Debug, Clone)]
pub struct Outfit {
    pub looktype: u16,
    pub outfit_type: u8,
    pub name: String,
    pub premium: bool,
    pub unlocked: bool,
    pub enabled: bool,
}

pub struct OutfitDatabase {
    pub outfits: HashMap<u16, Outfit>,
}

impl OutfitDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading outfits from {:?}", path);
        let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut outfits = HashMap::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().as_ref() == b"outfit" => {
                    let mut looktype = None;
                    let mut outfit_type = 0u8;
                    let mut name = String::new();
                    let mut premium = false;
                    let mut unlocked = false;
                    let mut enabled = true;

                    for attr in e.attributes() {
                        let attr = attr.map_err(|err| TfsRustError::Content {
                            file: path.to_string_lossy().into_owned(),
                            message: err.to_string(),
                        })?;
                        let key = attr.key.as_ref();
                        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match key {
                            b"type" => {
                                outfit_type =
                                    value.parse::<u8>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid outfit type '{value}': {err}"),
                                    })?
                            }
                            b"looktype" => {
                                looktype = Some(value.parse::<u16>().map_err(|err| {
                                    TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!("invalid looktype '{value}': {err}"),
                                    }
                                })?)
                            }
                            b"name" => name = value,
                            b"premium" => premium = parse_xml_bool(&value),
                            b"unlocked" => unlocked = parse_xml_bool(&value),
                            b"enabled" => enabled = parse_xml_bool(&value),
                            _ => {}
                        }
                    }

                    let looktype = looktype.ok_or_else(|| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: "outfit entry missing required 'looktype'".to_string(),
                    })?;
                    if name.is_empty() {
                        return Err(TfsRustError::Content {
                            file: path.to_string_lossy().into_owned(),
                            message: format!("outfit looktype {looktype} missing required 'name'"),
                        });
                    }

                    outfits.insert(
                        looktype,
                        Outfit {
                            looktype,
                            outfit_type,
                            name,
                            premium,
                            unlocked,
                            enabled,
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

        Ok(Self { outfits })
    }
}

fn parse_xml_bool(value: &str) -> bool {
    matches!(value, "yes" | "true" | "1" | "YES" | "TRUE")
}
