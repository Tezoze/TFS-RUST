//! Client-id item catalog (`items.ron`).
//!
//! One row per id. `id`, `server_id`, and `client_id` are the same u16.
//! C++ reference: `src/items.cpp` `Items::loadFromOtb` + `parseItemNode` (flag bits and XML keys).
//! Pack surface: the typed fields of `data/items/clientid_output/items.ron`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tfs_rust_common::ProtocolVersion;
use tfs_rust_common::error::{Result, TfsRustError};

use crate::items::{ItemDatabase, apply_xml_attribute, link_bed_transforms};
use crate::otb::ItemType;

#[derive(Debug, Deserialize)]
struct ItemCatalog {
    items: Vec<RonItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Item", deny_unknown_fields)]
struct RonItem {
    id: u16,
    name: String,
    #[serde(default)]
    article: String,
    #[serde(default)]
    plural: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    editor_suffix: String,
    #[serde(default)]
    group: Option<ItemGroup>,
    #[serde(default)]
    flags: Vec<RonFlag>,
    #[serde(default)]
    speed: Option<u16>,
    #[serde(default)]
    bonus_speed: Option<i32>,
    #[serde(default)]
    light_level: Option<u16>,
    #[serde(default)]
    light_color: Option<u16>,
    #[serde(default)]
    always_on_top_order: Option<u8>,
    #[serde(default)]
    weight: Option<u32>,
    #[serde(default)]
    container_size: Option<u16>,
    #[serde(default)]
    decay_to: Option<i32>,
    #[serde(default)]
    duration: Option<u32>,
    #[serde(default)]
    corpse_type: Option<CorpseType>,
    #[serde(default)]
    floor_change: Option<FloorChange>,
    #[serde(default)]
    destroy_to: Option<u16>,
    #[serde(default)]
    fluid_source: Option<FluidSource>,
    #[serde(default)]
    field: Option<RonField>,
    #[serde(default)]
    effect: Option<MagicEffectName>,
    #[serde(default, rename = "type")]
    type_name: Option<RonType>,
    #[serde(default)]
    weapon_type: Option<WeaponType>,
    #[serde(default)]
    ammo_type: Option<AmmoType>,
    #[serde(default)]
    shoot_type: Option<ShootTypeName>,
    #[serde(default)]
    slot_type: Option<SlotType>,
    #[serde(default)]
    attack: Option<i32>,
    #[serde(default)]
    defense: Option<i32>,
    #[serde(default)]
    armor: Option<i32>,
    #[serde(default)]
    charges: Option<u32>,
    #[serde(default)]
    range: Option<i32>,
    #[serde(default)]
    rotate_to: Option<u16>,
    #[serde(default)]
    write_once_item_id: Option<u16>,
    #[serde(default)]
    male_sleeper: Option<u16>,
    #[serde(default)]
    partner_direction: Option<DirectionName>,
    #[serde(default)]
    level_door: Option<u32>,
    #[serde(default)]
    poison_damage_cycles: Option<u32>,
    #[serde(default)]
    rune_spell_name: String,
    #[serde(default)]
    transform_equip_to: Option<u16>,
    #[serde(default)]
    transform_deequip_to: Option<u16>,
    #[serde(default)]
    max_text_len: Option<u16>,
    #[serde(default)]
    skill_axe: Option<i32>,
    #[serde(default)]
    skill_club: Option<i32>,
    #[serde(default)]
    skill_fist: Option<i32>,
    #[serde(default)]
    skill_sword: Option<i32>,
    #[serde(default)]
    skill_dist: Option<i32>,
    #[serde(default)]
    skill_shield: Option<i32>,
    #[serde(default)]
    magic_points: Option<i32>,
    #[serde(default)]
    hit_chance: Option<i32>,
    #[serde(default)]
    level_required: Option<u32>,
    #[serde(default)]
    absorb_percent_energy: Option<i32>,
    #[serde(default)]
    absorb_percent_fire: Option<i32>,
    #[serde(default)]
    absorb_percent_life_drain: Option<i32>,
    #[serde(default)]
    absorb_percent_magic: Option<i32>,
    #[serde(default)]
    absorb_percent_mana_drain: Option<i32>,
    #[serde(default)]
    absorb_percent_physical: Option<i32>,
    #[serde(default)]
    absorb_percent_poison: Option<i32>,
    #[serde(default)]
    absorb_percent_drown: Option<i32>,
    #[serde(default)]
    health_gain: Option<u32>,
    #[serde(default)]
    health_ticks: Option<u32>,
    #[serde(default)]
    mana_gain: Option<u32>,
    #[serde(default)]
    mana_ticks: Option<u32>,
    #[serde(default)]
    allow_dist_read: Option<bool>,
    #[serde(default)]
    allow_pickupable: Option<bool>,
    #[serde(default)]
    block_path_find: Option<bool>,
    #[serde(default)]
    block_projectile: Option<bool>,
    #[serde(default)]
    force_serialize: Option<bool>,
    #[serde(default)]
    force_use: Option<bool>,
    #[serde(default)]
    invisible: Option<bool>,
    #[serde(default)]
    mana_shield: Option<bool>,
    #[serde(default)]
    readable: Option<bool>,
    #[serde(default)]
    replace_magic_fields: Option<bool>,
    #[serde(default)]
    replaceable: Option<bool>,
    #[serde(default)]
    show_attributes: Option<bool>,
    #[serde(default)]
    show_charges: Option<bool>,
    #[serde(default)]
    show_duration: Option<bool>,
    #[serde(default)]
    special_field_block_path: Option<bool>,
    #[serde(default)]
    stop_duration: Option<bool>,
    #[serde(default)]
    suppress_drunk: Option<bool>,
    #[serde(default)]
    unlay: Option<bool>,
    #[serde(default)]
    writeable: Option<bool>,
    #[serde(default)]
    chest: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Field")]
struct RonField {
    kind: FieldKind,
    #[serde(default)]
    init_damage: i32,
    #[serde(default)]
    cycles: i32,
    #[serde(default)]
    skip_peaceful: bool,
}

#[derive(Debug, Deserialize)]
enum ItemGroup {
    Container,
    Fluid,
    Ground,
    Splash,
}

#[derive(Debug, Deserialize)]
enum RonFlag {
    AlwaysOnTop,
    Animation,
    BlockPathFind,
    BlockProjectile,
    BlockSolid,
    Hangable,
    HasHeight,
    Horizontal,
    Moveable,
    Pickupable,
    Readable,
    Rotatable,
    Stackable,
    Useable,
    Vertical,
}

#[derive(Debug, Deserialize)]
enum CorpseType {
    Blood,
    Fire,
    Undead,
    Venom,
}

#[derive(Debug, Deserialize)]
enum FloorChange {
    Down,
    East,
    North,
    South,
    West,
}

#[derive(Debug, Deserialize)]
enum FluidSource {
    Beer,
    Blood,
    Lemonade,
    Mud,
    Rum,
    Slime,
    Water,
    Wine,
}

#[derive(Debug, Deserialize)]
enum FieldKind {
    Energy,
    Fire,
    Poison,
}

#[derive(Debug, Deserialize)]
enum MagicEffectName {
    Bluebubble,
    Fire,
    Greenbubble,
    Poff,
    Teleport,
}

#[derive(Debug, Deserialize)]
enum RonType {
    Ammunition,
    Arrow,
    Axe,
    Backpack,
    Bed,
    Blood,
    Body,
    Bolt,
    Burstarrow,
    Club,
    Container,
    Depot,
    Distance,
    Door,
    Energy,
    Feet,
    Fire,
    Head,
    Key,
    Legs,
    Magicfield,
    Mailbox,
    Necklace,
    Poison,
    Poisonarrow,
    Powerbolt,
    Ring,
    Rune,
    Shield,
    Smallstone,
    Snowball,
    Spear,
    Sword,
    Teleport,
    Throwingknife,
    Throwingstar,
    Trashholder,
    Undead,
    Venom,
    Wand,
}

#[derive(Debug, Deserialize)]
enum WeaponType {
    Ammunition,
    Axe,
    Club,
    Distance,
    Shield,
    Sword,
    Wand,
}

#[derive(Debug, Deserialize)]
enum AmmoType {
    Arrow,
    Bolt,
}

#[derive(Debug, Deserialize)]
enum ShootTypeName {
    Arrow,
    Bolt,
    Burstarrow,
    Energy,
    Fire,
    Poison,
    Poisonarrow,
    Powerbolt,
    Smallstone,
    Snowball,
    Spear,
    Throwingknife,
    Throwingstar,
}

#[derive(Debug, Deserialize)]
enum SlotKind {
    Backpack,
    Body,
    Feet,
    Head,
    Legs,
    Necklace,
    Ring,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SlotType {
    Kind(SlotKind),
    Raw(String),
}

#[derive(Debug, Deserialize)]
enum DirectionName {
    East,
    North,
    South,
    West,
}

impl ItemDatabase {
    /// `data/items/<clientVersion>/items.ron`.
    pub fn ron_path(data_dir: &Path, version: ProtocolVersion) -> PathBuf {
        data_dir
            .join("items")
            .join(version.raw().to_string())
            .join("items.ron")
    }

    /// Load a client-id `items.ron`. Refuses a duplicate `id`.
    pub fn load_ron(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;
        Self::from_ron_str(&text, path)
    }

    pub fn from_ron_str(text: &str, path: &Path) -> Result<Self> {
        let catalog: ItemCatalog = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: e.to_string(),
            })?;
        let mut items = std::collections::HashMap::new();
        for row in catalog.items {
            let id = row.id;
            let item = row.into_item_type();
            if items.insert(id, item).is_some() {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: format!("duplicate item id {id} in items.ron"),
                });
            }
        }
        link_bed_transforms(&mut items);
        let mut client_to_server = std::collections::HashMap::with_capacity(items.len());
        for id in items.keys() {
            client_to_server.insert(*id, *id);
        }
        Ok(Self {
            items,
            client_to_server,
        })
    }
}

impl RonItem {
    fn into_item_type(self) -> ItemType {
        let mut item = ItemType {
            id: self.id,
            server_id: self.id,
            client_id: self.id,
            name: self.name,
            article: self.article,
            plural_name: self.plural,
            description: self.description,
            speed: self.speed.unwrap_or(0),
            light_level: self.light_level.unwrap_or(0) as u8,
            light_color: self.light_color.unwrap_or(0) as u8,
            always_on_top_order: self.always_on_top_order.unwrap_or(0),
            distuse: false,
            ..ItemType::default()
        };
        if let Some(group) = self.group {
            item.group = match group {
                ItemGroup::Ground => ItemType::GROUP_GROUND,
                ItemGroup::Container => ItemType::GROUP_CONTAINER,
                ItemGroup::Splash => ItemType::GROUP_SPLASH,
                ItemGroup::Fluid => ItemType::GROUP_FLUID,
            };
        }
        for flag in &self.flags {
            let name = match flag {
                RonFlag::AlwaysOnTop => "AlwaysOnTop",
                RonFlag::Animation => "Animation",
                RonFlag::BlockPathFind => "BlockPathFind",
                RonFlag::BlockProjectile => "BlockProjectile",
                RonFlag::BlockSolid => "BlockSolid",
                RonFlag::Hangable => "Hangable",
                RonFlag::HasHeight => "HasHeight",
                RonFlag::Horizontal => "Horizontal",
                RonFlag::Moveable => "Moveable",
                RonFlag::Pickupable => "Pickupable",
                RonFlag::Readable => "Readable",
                RonFlag::Rotatable => "Rotatable",
                RonFlag::Stackable => "Stackable",
                RonFlag::Useable => "Useable",
                RonFlag::Vertical => "Vertical",
            };
            if let Some(bit) = ItemType::flag_bit_from_name(name) {
                item.flags |= bit;
            }
        }
        if self.block_path_find == Some(true)
            && let Some(bit) = ItemType::flag_bit_from_name("BlockPathFind")
        {
            item.flags |= bit;
        }
        let id = item.id;
        let mut attrs: Vec<(&str, String)> = Vec::new();
        push_opt(&mut attrs, "weight", self.weight);
        push_opt(&mut attrs, "containersize", self.container_size);
        push_opt(&mut attrs, "decayto", self.decay_to);
        push_opt(&mut attrs, "duration", self.duration);
        push_opt(&mut attrs, "destroyto", self.destroy_to);
        push_opt(&mut attrs, "attack", self.attack);
        push_opt(&mut attrs, "defense", self.defense);
        push_opt(&mut attrs, "armor", self.armor);
        push_opt(&mut attrs, "charges", self.charges);
        push_opt(&mut attrs, "range", self.range);
        push_opt(&mut attrs, "rotateto", self.rotate_to);
        push_opt(&mut attrs, "writeonceitemid", self.write_once_item_id);
        push_opt(&mut attrs, "malesleeper", self.male_sleeper);
        push_opt(&mut attrs, "leveldoor", self.level_door);
        push_opt(&mut attrs, "poisondamagecycles", self.poison_damage_cycles);
        push_opt(&mut attrs, "transformequipto", self.transform_equip_to);
        push_opt(&mut attrs, "transformdeequipto", self.transform_deequip_to);
        push_opt(&mut attrs, "maxtextlen", self.max_text_len);
        push_opt(&mut attrs, "speed", self.bonus_speed);
        push_opt(&mut attrs, "skillaxe", self.skill_axe);
        push_opt(&mut attrs, "skillclub", self.skill_club);
        push_opt(&mut attrs, "skillfist", self.skill_fist);
        push_opt(&mut attrs, "skillsword", self.skill_sword);
        push_opt(&mut attrs, "skilldist", self.skill_dist);
        push_opt(&mut attrs, "skillshield", self.skill_shield);
        push_opt(&mut attrs, "magicpoints", self.magic_points);
        push_opt(&mut attrs, "hitchance", self.hit_chance);
        push_opt(&mut attrs, "levelrequired", self.level_required);
        push_opt(
            &mut attrs,
            "absorbpercentenergy",
            self.absorb_percent_energy,
        );
        push_opt(&mut attrs, "absorbpercentfire", self.absorb_percent_fire);
        push_opt(
            &mut attrs,
            "absorbpercentlifedrain",
            self.absorb_percent_life_drain,
        );
        push_opt(&mut attrs, "absorbpercentmagic", self.absorb_percent_magic);
        push_opt(
            &mut attrs,
            "absorbpercentmanadrain",
            self.absorb_percent_mana_drain,
        );
        push_opt(
            &mut attrs,
            "absorbpercentphysical",
            self.absorb_percent_physical,
        );
        push_opt(
            &mut attrs,
            "absorbpercentpoison",
            self.absorb_percent_poison,
        );
        push_opt(&mut attrs, "absorbpercentdrown", self.absorb_percent_drown);
        push_opt(&mut attrs, "healthgain", self.health_gain);
        push_opt(&mut attrs, "healthticks", self.health_ticks);
        push_opt(&mut attrs, "managain", self.mana_gain);
        push_opt(&mut attrs, "manaticks", self.mana_ticks);
        if let Some(kind) = self.corpse_type {
            attrs.push((
                "corpsetype",
                match kind {
                    CorpseType::Blood => "blood",
                    CorpseType::Fire => "fire",
                    CorpseType::Undead => "undead",
                    CorpseType::Venom => "venom",
                }
                .to_string(),
            ));
        }
        if let Some(fc) = self.floor_change {
            attrs.push((
                "floorchange",
                match fc {
                    FloorChange::Down => "down",
                    FloorChange::East => "east",
                    FloorChange::North => "north",
                    FloorChange::South => "south",
                    FloorChange::West => "west",
                }
                .to_string(),
            ));
        }
        if let Some(fluid) = self.fluid_source {
            attrs.push((
                "fluidsource",
                match fluid {
                    FluidSource::Beer => "beer",
                    FluidSource::Blood => "blood",
                    FluidSource::Lemonade => "lemonade",
                    FluidSource::Mud => "mud",
                    FluidSource::Rum => "rum",
                    FluidSource::Slime => "slime",
                    FluidSource::Water => "water",
                    FluidSource::Wine => "wine",
                }
                .to_string(),
            ));
        }
        if let Some(effect) = self.effect {
            attrs.push((
                "effect",
                match effect {
                    MagicEffectName::Bluebubble => "bluebubble",
                    MagicEffectName::Fire => "fire",
                    MagicEffectName::Greenbubble => "greenbubble",
                    MagicEffectName::Poff => "poff",
                    MagicEffectName::Teleport => "teleport",
                }
                .to_string(),
            ));
        }
        if let Some(token) = self.type_name.as_ref().and_then(RonType::item_type_token) {
            attrs.push(("type", token.to_string()));
        }
        if let Some(weapon) = self.weapon_type {
            attrs.push((
                "weapontype",
                match weapon {
                    WeaponType::Ammunition => "ammunition",
                    WeaponType::Axe => "axe",
                    WeaponType::Club => "club",
                    WeaponType::Distance => "distance",
                    WeaponType::Shield => "shield",
                    WeaponType::Sword => "sword",
                    WeaponType::Wand => "wand",
                }
                .to_string(),
            ));
        }
        if let Some(ammo) = self.ammo_type {
            attrs.push((
                "ammotype",
                match ammo {
                    AmmoType::Arrow => "arrow",
                    AmmoType::Bolt => "bolt",
                }
                .to_string(),
            ));
        }
        if let Some(shoot) = self.shoot_type {
            attrs.push(("shoottype", shoot_token(shoot).to_string()));
        }
        if let Some(slot) = self.slot_type {
            attrs.push(("slottype", slot.as_token().to_string()));
        }
        if let Some(dir) = self.partner_direction {
            attrs.push((
                "partnerdirection",
                match dir {
                    DirectionName::East => "east",
                    DirectionName::North => "north",
                    DirectionName::South => "south",
                    DirectionName::West => "west",
                }
                .to_string(),
            ));
        }
        if !self.rune_spell_name.is_empty() {
            attrs.push(("runespellname", self.rune_spell_name));
        }
        push_bool(&mut attrs, "allowdistread", self.allow_dist_read);
        push_bool(&mut attrs, "allowpickupable", self.allow_pickupable);
        push_bool(&mut attrs, "blockprojectile", self.block_projectile);
        push_bool(&mut attrs, "forceserialize", self.force_serialize);
        push_bool(&mut attrs, "forceuse", self.force_use);
        push_bool(&mut attrs, "invisible", self.invisible);
        push_bool(&mut attrs, "manashield", self.mana_shield);
        push_bool(&mut attrs, "readable", self.readable);
        push_bool(&mut attrs, "replacemagicfields", self.replace_magic_fields);
        push_bool(&mut attrs, "replaceable", self.replaceable);
        push_bool(&mut attrs, "showattributes", self.show_attributes);
        push_bool(&mut attrs, "showcharges", self.show_charges);
        push_bool(&mut attrs, "showduration", self.show_duration);
        push_bool(
            &mut attrs,
            "specialfieldblockpath",
            self.special_field_block_path,
        );
        push_bool(&mut attrs, "stopduration", self.stop_duration);
        push_bool(&mut attrs, "suppressdrunk", self.suppress_drunk);
        push_bool(&mut attrs, "unlay", self.unlay);
        push_bool(&mut attrs, "writeable", self.writeable);
        push_bool(&mut attrs, "chest", self.chest);
        push_bool(&mut attrs, "blockpathfind", self.block_path_find);
        for (key, value) in attrs {
            apply_xml_attribute(&mut item, key, &value, id);
        }
        if let Some(field) = self.field {
            let kind = match field.kind {
                FieldKind::Fire => "fire",
                FieldKind::Poison => "poison",
                FieldKind::Energy => "energy",
            };
            apply_xml_attribute(&mut item, "field", kind, id);
            item.xml_attributes
                .insert("field.initdamage".into(), field.init_damage.to_string());
            item.xml_attributes
                .insert("field.cycles".into(), field.cycles.to_string());
            if field.skip_peaceful {
                item.xml_attributes
                    .insert("field.skippeaceful".into(), "1".into());
            }
        }
        if !self.editor_suffix.is_empty() {
            item.xml_attributes
                .insert("editorsuffix".into(), self.editor_suffix);
        }
        item
    }
}

impl RonType {
    fn item_type_token(&self) -> Option<&'static str> {
        Some(match self {
            Self::Container => "container",
            Self::Depot => "depot",
            Self::Mailbox => "mailbox",
            Self::Trashholder => "trashholder",
            Self::Magicfield => "magicfield",
            Self::Door => "door",
            Self::Teleport => "teleport",
            Self::Bed => "bed",
            Self::Key => "key",
            Self::Rune => "rune",
            _ => return None,
        })
    }
}

impl SlotType {
    fn as_token(&self) -> String {
        match self {
            Self::Kind(SlotKind::Backpack) => "backpack".into(),
            Self::Kind(SlotKind::Body) => "body".into(),
            Self::Kind(SlotKind::Feet) => "feet".into(),
            Self::Kind(SlotKind::Head) => "head".into(),
            Self::Kind(SlotKind::Legs) => "legs".into(),
            Self::Kind(SlotKind::Necklace) => "necklace".into(),
            Self::Kind(SlotKind::Ring) => "ring".into(),
            Self::Raw(s) => s.clone(),
        }
    }
}

fn shoot_token(shoot: ShootTypeName) -> &'static str {
    match shoot {
        ShootTypeName::Arrow => "arrow",
        ShootTypeName::Bolt => "bolt",
        ShootTypeName::Burstarrow => "burstarrow",
        ShootTypeName::Energy => "energy",
        ShootTypeName::Fire => "fire",
        ShootTypeName::Poison => "poison",
        ShootTypeName::Poisonarrow => "poisonarrow",
        ShootTypeName::Powerbolt => "powerbolt",
        ShootTypeName::Smallstone => "smallstone",
        ShootTypeName::Snowball => "snowball",
        ShootTypeName::Spear => "spear",
        ShootTypeName::Throwingknife => "throwingknife",
        ShootTypeName::Throwingstar => "throwingstar",
    }
}

fn push_opt<T: std::fmt::Display>(
    out: &mut Vec<(&str, String)>,
    key: &'static str,
    value: Option<T>,
) {
    if let Some(value) = value {
        out.push((key, value.to_string()));
    }
}

fn push_bool(out: &mut Vec<(&str, String)>, key: &'static str, value: Option<bool>) {
    if let Some(value) = value {
        out.push((key, if value { "true" } else { "false" }.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_id_is_refused() {
        let text = r#"
            ItemCatalog(
                items: [
                    Item(id: 1, name: "water"),
                    Item(id: 1, name: "wine"),
                ],
            )
        "#;
        let err = match ItemDatabase::from_ron_str(text, Path::new("items.ron")) {
            Err(err) => err,
            Ok(_) => panic!("duplicate id must be refused"),
        };
        let msg = err.to_string();
        assert!(msg.contains("duplicate item id 1"), "{msg}");
    }

    #[test]
    fn gold_and_sandstone_and_bare_fields() {
        let text = r#"
            ItemCatalog(
                items: [
                    Item(
                        id: 3031,
                        name: "gold coin",
                        article: "a",
                        plural: "gold coins",
                        flags: [Pickupable, Moveable, Stackable],
                        weight: 10,
                    ),
                    Item(
                        id: 425,
                        name: "sandstone floor",
                        group: Ground,
                        flags: [BlockPathFind, Animation],
                        speed: 70,
                    ),
                    Item(
                        id: 2773,
                        name: "lever",
                        article: "a",
                    ),
                ],
            )
        "#;
        let db = ItemDatabase::from_ron_str(text, Path::new("items.ron")).expect("ron");
        let gold = db.items.get(&3031).expect("gold");
        assert_eq!(gold.client_id, 3031);
        assert_eq!(gold.server_id, 3031);
        assert!(gold.stackable());
        assert_eq!(gold.plural_name, "gold coins");
        assert_eq!(gold.weight, 10);
        let sand = db.items.get(&425).expect("sand");
        assert_eq!(sand.speed, 70);
        assert!(sand.is_ground_tile());
        assert!(sand.block_path_find());
        let lever = db.items.get(&2773).expect("lever");
        assert_eq!(lever.decay_time, 0);
        assert_eq!(lever.decay_to, -1);
    }

    #[test]
    fn clientid_output_ron_loads_without_duplicate_ids() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items/772/items.ron");
        let db = ItemDatabase::load_ron(&path).expect("772 items.ron");
        assert!(db.items.len() > 4000);
        let sand = db.items.get(&425).expect("sandstone");
        assert_eq!(sand.speed, 70);
        assert!(sand.block_solid());
        assert!(!sand.is_animation());
        let lever = db.items.get(&2773).expect("lever");
        assert_eq!(lever.decay_time, 0);
        assert_eq!(lever.decay_to, -1);
        assert_eq!(sand.client_id, sand.server_id);
        let gold = db.items.get(&3031).expect("gold");
        assert_eq!(gold.name, "gold coin");
        assert!(gold.stackable());
    }

    /// Every OTB `itemflags_t` bit on the live catalog is set on the matching ron item.
    /// XML flag attributes are the same bits (`flags: [...]`), not a second copy.
    #[test]
    fn ron_keeps_every_otb_flag_bit() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items");
        let otb_path = root.join("items.otb");
        if !otb_path.is_file() {
            return;
        }
        let otb =
            crate::items::ItemDatabase::load(&otb_path, &root.join("items.xml")).expect("otb+xml");
        let ron = ItemDatabase::load_ron(&root.join("772/items.ron")).expect("ron");
        const BITS: &[u32] = &[
            1 << 0,
            1 << 1,
            1 << 2,
            1 << 3,
            1 << 4,
            1 << 5,
            1 << 6,
            1 << 7,
            1 << 13,
            1 << 14,
            1 << 15,
            1 << 16,
            1 << 17,
            1 << 18,
            1 << 20,
            1 << 23,
            1 << 24,
            1 << 26,
        ];
        let known: u32 = BITS.iter().fold(0, |acc, bit| acc | bit);
        for (id, src) in &otb.items {
            let dst = ron
                .items
                .get(id)
                .unwrap_or_else(|| panic!("ron missing id {id}"));
            assert_eq!(src.flags & !known, 0, "unknown otb flag bits on {id}");
            let lost = src.flags & !dst.flags;
            assert_eq!(lost, 0, "ron dropped flag bits {lost:#x} on {id}");
            assert_eq!(src.block_solid(), dst.block_solid(), "block_solid {id}");
            assert_eq!(
                src.block_projectile(),
                dst.block_projectile(),
                "block_projectile {id}"
            );
            assert_eq!(src.moveable(), dst.moveable(), "moveable {id}");
            assert_eq!(src.pickupable(), dst.pickupable(), "pickupable {id}");
            assert_eq!(
                src.allow_dist_read(),
                dst.allow_dist_read(),
                "allow_dist_read {id}"
            );
            assert_eq!(src.force_use(), dst.force_use(), "force_use {id}");
            assert_eq!(
                src.can_read_text(),
                dst.can_read_text(),
                "can_read_text {id}"
            );
            // XML `blockpathfind="1"` on these three is stored as BlockPathFind.
            // The OTB loader keeps the attribute string and does not set the bit.
            if !matches!(*id, 2142 | 2143 | 2149) {
                assert_eq!(
                    src.block_path_find(),
                    dst.block_path_find(),
                    "block_path_find {id}"
                );
            }
        }
    }

    /// 800 `items.srv` types after the 772 catalog (5089), converted onto our flags.
    #[test]
    fn version_800_ron_appends_srv_types_after_772() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/items/800/items.ron");
        let db = ItemDatabase::load_ron(&path).expect("800 items.ron");
        let door = db.items.get(&5099).expect("open door");
        assert!(door.is_door());
        assert!(door.always_on_top());
        assert!(!door.block_solid());
        let closed = db.items.get(&5098).expect("closed door");
        assert!(closed.block_solid());
        assert!(closed.block_projectile());
        let floor = db.items.get(&17100).expect("green floor");
        assert!(floor.is_ground_tile());
        assert_eq!(floor.speed, 150);
        assert!(!floor.block_solid());
        let sword = db.items.get(&8102).expect("emerald sword");
        assert_eq!(sword.attack, 49);
        assert_eq!(sword.weapon_type, 1);
        let wall = db.items.get(&5630).expect("dirt wall");
        assert!(wall.block_solid());
        assert!(wall.block_projectile(), "Unthrow is BlockProjectile");
        let hole = db.items.get(&5731).expect("hole");
        assert!(hole.is_ground_tile());
        assert!(hole.block_path_find(), "Avoid is BlockPathFind");
        assert!(!hole.block_solid());
        let chest = db.items.get(&2472).expect("chest");
        assert!(!chest.block_solid(), "srv chest has no Unpass");
        assert!(chest.block_path_find());
        let mountain = db.items.get(&1128).expect("mountain");
        assert!(mountain.block_solid(), "srv mountain is Unpass");
        assert!(mountain.is_ground_tile());
        assert!(db.items.contains_key(&17161));
        assert!(db.items.len() > 7000);
    }
}
