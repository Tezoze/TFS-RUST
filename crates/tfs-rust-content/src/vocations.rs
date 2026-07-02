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
    /// `gainhpticks` — rounds between HP regen ticks (`TSkillFed::Event` `SecsPerHP`,
    /// `crskill.cc:828-874`). `0` ⇒ no HP regen from food.
    pub gain_hp_ticks: u32,
    /// `gainhpamount` — HP gained per regen tick (`crskill.cc:880`).
    pub gain_hp_amount: i32,
    /// `gainmanaticks` — rounds between mana regen ticks (`crskill.cc:828-874`). `0` ⇒ no mana regen.
    pub gain_mana_ticks: u32,
    /// `gainmanaamount` — mana gained per regen tick (`crskill.cc:884`).
    pub gain_mana_amount: i32,
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

    /// `TSkillFed::Event` regen cadence for a vocation (`crskill.cc:828-885`).
    /// Returns `(hp_ticks, hp_amount, mana_ticks, mana_amount)`. When the vocation
    /// is absent from the database, falls back to the C++ `default:` case
    /// (`SecsPerHP = 12`, `SecsPerMana = 6`) with the hardcoded `Change(1)`/`Change(2)`
    /// amounts (`crskill.cc:871,880,884`).
    pub fn fed_regen_params(&self, vocation_id: i32) -> (u32, i32, u32, i32) {
        if vocation_id < 0 {
            return (12, 1, 6, 2);
        }
        self.vocations
            .get(&(vocation_id as u16))
            .map(|v| {
                (
                    v.gain_hp_ticks,
                    v.gain_hp_amount,
                    v.gain_mana_ticks,
                    v.gain_mana_amount,
                )
            })
            .unwrap_or((12, 1, 6, 2))
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
                    let mut gain_hp_ticks = 0u32;
                    let mut gain_hp_amount = 0i32;
                    let mut gain_mana_ticks = 0u32;
                    let mut gain_mana_amount = 0i32;

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
                            b"gainhpticks" => {
                                gain_hp_ticks =
                                    value.parse::<u32>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!(
                                            "invalid gainhpticks '{value}': {err}"
                                        ),
                                    })?
                            }
                            b"gainhpamount" => {
                                gain_hp_amount =
                                    value.parse::<i32>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!(
                                            "invalid gainhpamount '{value}': {err}"
                                        ),
                                    })?
                            }
                            b"gainmanaticks" => {
                                gain_mana_ticks =
                                    value.parse::<u32>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!(
                                            "invalid gainmanaticks '{value}': {err}"
                                        ),
                                    })?
                            }
                            b"gainmanaamount" => {
                                gain_mana_amount =
                                    value.parse::<i32>().map_err(|err| TfsRustError::Content {
                                        file: path.to_string_lossy().into_owned(),
                                        message: format!(
                                            "invalid gainmanaamount '{value}': {err}"
                                        ),
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
                            gain_hp_ticks,
                            gain_hp_amount,
                            gain_mana_ticks,
                            gain_mana_amount,
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
