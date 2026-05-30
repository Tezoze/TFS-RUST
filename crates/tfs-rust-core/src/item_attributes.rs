//! Item attribute system - full implementation matching C++ TFS 1.4.2
// C++ reference: `src/item.h` (AttrTypes_t enum, ItemAttributes class)
//                `src/enums.h` (itemAttrTypes bitflags)

use std::collections::HashMap;

/// Attribute type identifiers for serialization (AttrTypes_t in C++)
// C++ ref: `src/item.h:55-101`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttrType {
    TileFlags = 3,
    ActionId = 4,
    UniqueId = 5,
    Text = 6,
    Description = 7,
    TeleDest = 8,
    Item = 9,
    DepotId = 10,
    RuneCharges = 12,
    HouseDoorId = 14,
    Count = 15,
    Duration = 16,
    DecayingState = 17,
    WrittenDate = 18,
    WrittenBy = 19,
    SleeperGuid = 20,
    SleepStart = 21,
    Charges = 22,
    ContainerItems = 23,
    Name = 24,
    Article = 25,
    PluralName = 26,
    Weight = 27,
    Attack = 28,
    Defense = 29,
    ExtraDefense = 30,
    Armor = 31,
    HitChance = 32,
    ShootRange = 33,
    CustomAttributes = 34,
    DecayTo = 35,
    WrapId = 36,
    StoreItem = 37,
    AttackSpeed = 38,
    OpenContainer = 39,
    PodiumOutfit = 40,
    Tier = 41,
    ContainerSize = 42,
}

/// Item attribute bitflags for internal tracking (itemAttrTypes in C++)
// C++ ref: `src/enums.h:50-85`
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ItemAttrFlags: u32 {
        const ACTION_ID          = 1 << 0;
        const UNIQUE_ID          = 1 << 1;
        const DESCRIPTION        = 1 << 2;
        const TEXT               = 1 << 3;
        const DATE               = 1 << 4;
        const WRITER             = 1 << 5;
        const NAME               = 1 << 6;
        const ARTICLE            = 1 << 7;
        const PLURAL_NAME        = 1 << 8;
        const WEIGHT             = 1 << 9;
        const ATTACK             = 1 << 10;
        const DEFENSE            = 1 << 11;
        const EXTRA_DEFENSE      = 1 << 12;
        const ARMOR              = 1 << 13;
        const HIT_CHANCE         = 1 << 14;
        const SHOOT_RANGE        = 1 << 15;
        const OWNER              = 1 << 16;
        const DURATION           = 1 << 17;
        const DECAY_STATE        = 1 << 18;
        const CORPSE_OWNER       = 1 << 19;
        const CHARGES            = 1 << 20;
        const FLUID_TYPE         = 1 << 21;
        const DOOR_ID            = 1 << 22;
        const DECAY_TO           = 1 << 23;
        const WRAP_ID            = 1 << 24;
        const STORE_ITEM         = 1 << 25;
        const ATTACK_SPEED       = 1 << 26;
        const AUTO_OPEN          = 1 << 27;
        const DURATION_TIMESTAMP = 1 << 28;
        const CONTAINER_SIZE     = 1 << 29;
        const DEPOT_ID           = 1 << 30;
        const CUSTOM             = 1 << 31;
    }
}

impl Default for ItemAttrFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Decay state for items with duration (ItemDecayState_t in C++)
// C++ ref: `src/item.h:48-53`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DecayState {
    #[default]
    False = 0,
    True = 1,
    Pending = 2,
    Stopping = 3,
}

/// A single custom attribute value (CustomAttribute in C++)
// C++ ref: `src/item.h:217-347`
#[derive(Debug, Clone, PartialEq)]
pub enum CustomAttrValue {
    None,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl Default for CustomAttrValue {
    fn default() -> Self {
        Self::None
    }
}

/// Custom attribute map type
pub type CustomAttributeMap = HashMap<String, CustomAttrValue>;

/// Item attribute storage matching C++ ItemAttributes class
// C++ ref: `src/item.h:109-400`
#[derive(Debug, Clone, Default)]
pub struct ItemAttributes {
    /// Bitmask of which attributes are set
    attribute_bits: ItemAttrFlags,
    /// Integer attributes stored directly
    action_id: u16,
    unique_id: u16,
    date: i64,
    weight: u32,
    attack: i32,
    defense: i32,
    extra_defense: i32,
    armor: i32,
    hit_chance: i32,
    shoot_range: i32,
    owner: u32,
    duration: i32,
    decay_state: u8, // DecayState as u8
    corpse_owner: u32,
    charges: u16,
    fluid_type: u16,
    door_id: u8,
    decay_to: u32,
    wrap_id: u32,
    store_item: u32,
    attack_speed: u32,
    auto_open: u8,
    duration_timestamp: i64,
    container_size: u8,
    depot_id: u16,
    /// String attributes
    description: Option<String>,
    text: Option<String>,
    writer: Option<String>,
    name: Option<String>,
    article: Option<String>,
    plural_name: Option<String>,
    /// Custom attributes map (lazily initialized)
    custom: Option<Box<CustomAttributeMap>>,
}

impl ItemAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    // === Attribute Checkers ===

    pub fn has_action_id(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::ACTION_ID)
    }

    pub fn has_unique_id(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::UNIQUE_ID)
    }

    pub fn has_description(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::DESCRIPTION)
    }

    pub fn has_text(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::TEXT)
    }

    pub fn has_date(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::DATE)
    }

    pub fn has_writer(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::WRITER)
    }

    pub fn has_charges(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::CHARGES)
    }

    pub fn has_duration(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::DURATION)
    }

    pub fn has_decay_state(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::DECAY_STATE)
    }

    pub fn has_fluid_type(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::FLUID_TYPE)
    }

    pub fn has_owner(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::OWNER)
    }

    pub fn has_corpse_owner(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::CORPSE_OWNER)
    }

    pub fn has_custom_attributes(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::CUSTOM)
    }

    // === Getters ===

    pub fn get_action_id(&self) -> u16 {
        if self.has_action_id() { self.action_id } else { 0 }
    }

    pub fn get_unique_id(&self) -> u16 {
        if self.has_unique_id() { self.unique_id } else { 0 }
    }

    pub fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    pub fn get_text(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    pub fn get_writer(&self) -> &str {
        self.writer.as_deref().unwrap_or("")
    }

    pub fn get_date(&self) -> i64 {
        if self.has_date() { self.date } else { 0 }
    }

    pub fn get_charges(&self) -> u16 {
        if self.has_charges() { self.charges } else { 0 }
    }

    pub fn get_fluid_type(&self) -> u16 {
        if self.has_fluid_type() { self.fluid_type } else { 0 }
    }

    pub fn get_owner(&self) -> u32 {
        if self.has_owner() { self.owner } else { 0 }
    }

    pub fn get_corpse_owner(&self) -> u32 {
        if self.has_corpse_owner() { self.corpse_owner } else { 0 }
    }

    pub fn get_duration(&self) -> i32 {
        let decay_state = self.get_decaying();
        if decay_state == DecayState::True || decay_state == DecayState::Stopping {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let remaining = self.duration_timestamp.saturating_sub(now);
            remaining.max(0) as i32
        } else {
            if self.has_duration() { self.duration } else { 0 }
        }
    }

    pub fn get_duration_raw(&self) -> i32 {
        if self.has_duration() { self.duration } else { 0 }
    }

    pub fn get_duration_timestamp(&self) -> i64 {
        self.duration_timestamp
    }

    pub fn get_decaying(&self) -> DecayState {
        if self.has_decay_state() {
            match self.decay_state {
                0 => DecayState::False,
                1 => DecayState::True,
                2 => DecayState::Pending,
                3 => DecayState::Stopping,
                _ => DecayState::False,
            }
        } else {
            DecayState::False
        }
    }

    pub fn get_door_id(&self) -> u8 {
        if self.attribute_bits.contains(ItemAttrFlags::DOOR_ID) {
            self.door_id
        } else {
            0
        }
    }

    pub fn get_decay_to(&self) -> u32 {
        if self.attribute_bits.contains(ItemAttrFlags::DECAY_TO) {
            self.decay_to
        } else {
            0
        }
    }

    pub fn get_wrap_id(&self) -> u32 {
        if self.attribute_bits.contains(ItemAttrFlags::WRAP_ID) {
            self.wrap_id
        } else {
            0
        }
    }

    pub fn get_attack_speed(&self) -> u32 {
        if self.attribute_bits.contains(ItemAttrFlags::ATTACK_SPEED) {
            self.attack_speed
        } else {
            0
        }
    }

    /// C++ `DepotLocker::getDepotId` / `ATTR_DEPOT_ID` — `depotlocker.cpp`, `item.h`.
    pub fn get_depot_id(&self) -> u16 {
        if self.attribute_bits.contains(ItemAttrFlags::DEPOT_ID) {
            self.depot_id
        } else {
            0
        }
    }

    pub fn has_depot_id(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::DEPOT_ID)
    }

    /// `Item::getAttack` — `src/item.h` (attribute overrides `ItemType::attack`).
    pub fn get_attack(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::ATTACK)
            .then_some(self.attack)
    }

    /// `Item::getDefense` — `src/item.h`.
    pub fn get_defense(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::DEFENSE)
            .then_some(self.defense)
    }

    /// `Item::getExtraDefense` — `src/item.h`.
    pub fn get_extra_defense(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::EXTRA_DEFENSE)
            .then_some(self.extra_defense)
    }

    /// `Item::getArmor` — `src/item.h`.
    pub fn get_armor(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::ARMOR)
            .then_some(self.armor)
    }

    /// `Item::getShootRange` — `src/item.h`.
    pub fn get_shoot_range_attr(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::SHOOT_RANGE)
            .then_some(self.shoot_range)
    }

    /// `Item::getHitChance` — `src/item.h`.
    pub fn get_hit_chance_attr(&self) -> Option<i32> {
        self.attribute_bits
            .contains(ItemAttrFlags::HIT_CHANCE)
            .then_some(self.hit_chance)
    }

    pub fn get_name_str(&self) -> Option<&str> {
        self.name.as_deref().filter(|s| !s.is_empty())
    }

    pub fn get_article_str(&self) -> Option<&str> {
        self.article.as_deref().filter(|s| !s.is_empty())
    }

    pub fn get_plural_name_str(&self) -> Option<&str> {
        self.plural_name.as_deref().filter(|s| !s.is_empty())
    }

    /// Base weight in 1/100 oz — `Item::getBaseWeight` (`item.cpp`).
    pub fn base_weight_oz(&self, type_weight: u32) -> u32 {
        if self.attribute_bits.contains(ItemAttrFlags::WEIGHT) {
            self.weight
        } else {
            type_weight
        }
    }

    // === Setters ===

    pub fn set_action_id(&mut self, value: u16) {
        self.attribute_bits.insert(ItemAttrFlags::ACTION_ID);
        self.action_id = value;
    }

    pub fn set_unique_id(&mut self, value: u16) {
        self.attribute_bits.insert(ItemAttrFlags::UNIQUE_ID);
        self.unique_id = value;
    }

    pub fn set_description(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::DESCRIPTION);
        self.description = Some(value.into());
    }

    pub fn set_text(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::TEXT);
        self.text = Some(value.into());
    }

    pub fn reset_text(&mut self) {
        self.attribute_bits.remove(ItemAttrFlags::TEXT);
        self.text = None;
    }

    pub fn set_writer(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::WRITER);
        self.writer = Some(value.into());
    }

    pub fn reset_writer(&mut self) {
        self.attribute_bits.remove(ItemAttrFlags::WRITER);
        self.writer = None;
    }

    pub fn set_date(&mut self, value: i64) {
        self.attribute_bits.insert(ItemAttrFlags::DATE);
        self.date = value;
    }

    pub fn reset_date(&mut self) {
        self.attribute_bits.remove(ItemAttrFlags::DATE);
        self.date = 0;
    }

    pub fn set_charges(&mut self, value: u16) {
        self.attribute_bits.insert(ItemAttrFlags::CHARGES);
        self.charges = value;
    }

    pub fn set_fluid_type(&mut self, value: u16) {
        self.attribute_bits.insert(ItemAttrFlags::FLUID_TYPE);
        self.fluid_type = value;
    }

    pub fn set_owner(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::OWNER);
        self.owner = value;
    }

    pub fn set_corpse_owner(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::CORPSE_OWNER);
        self.corpse_owner = value;
    }

    pub fn set_duration(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::DURATION);
        self.duration = value.max(0);
    }

    pub fn set_duration_timestamp(&mut self, value: i64) {
        self.attribute_bits.insert(ItemAttrFlags::DURATION_TIMESTAMP);
        self.duration_timestamp = value;
    }

    pub fn set_decaying(&mut self, state: DecayState) {
        self.attribute_bits.insert(ItemAttrFlags::DECAY_STATE);
        self.decay_state = state as u8;
        if state == DecayState::False {
            self.attribute_bits.remove(ItemAttrFlags::DURATION_TIMESTAMP);
        }
    }

    pub fn set_door_id(&mut self, value: u8) {
        self.attribute_bits.insert(ItemAttrFlags::DOOR_ID);
        self.door_id = value;
    }

    pub fn set_decay_to(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::DECAY_TO);
        self.decay_to = value;
    }

    pub fn set_wrap_id(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::WRAP_ID);
        self.wrap_id = value;
    }

    pub fn set_attack_speed(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::ATTACK_SPEED);
        self.attack_speed = value;
    }

    pub fn set_name(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::NAME);
        self.name = Some(value.into());
    }

    pub fn set_article(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::ARTICLE);
        self.article = Some(value.into());
    }

    pub fn set_plural_name(&mut self, value: impl Into<String>) {
        self.attribute_bits.insert(ItemAttrFlags::PLURAL_NAME);
        self.plural_name = Some(value.into());
    }

    pub fn set_weight_attr(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::WEIGHT);
        self.weight = value;
    }

    pub fn set_attack(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::ATTACK);
        self.attack = value;
    }

    pub fn set_defense(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::DEFENSE);
        self.defense = value;
    }

    pub fn set_extra_defense(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::EXTRA_DEFENSE);
        self.extra_defense = value;
    }

    pub fn set_armor(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::ARMOR);
        self.armor = value;
    }

    pub fn set_hit_chance(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::HIT_CHANCE);
        self.hit_chance = value;
    }

    pub fn set_shoot_range(&mut self, value: i32) {
        self.attribute_bits.insert(ItemAttrFlags::SHOOT_RANGE);
        self.shoot_range = value;
    }

    pub fn set_store_item(&mut self, value: u32) {
        self.attribute_bits.insert(ItemAttrFlags::STORE_ITEM);
        self.store_item = value;
    }

    /// TFS `Item::isStoreItem` — `src/item.h`
    #[inline]
    pub fn is_store_item(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::STORE_ITEM)
    }

    /// Byte written for `ATTR_STOREITEM` (`item.cpp` `serializeAttr`).
    #[inline]
    pub fn store_item_serial_byte(&self) -> u8 {
        self.store_item.min(u32::from(u8::MAX)) as u8
    }

    /// `ATTR_WEIGHT` payload when the weight override flag is set.
    #[inline]
    pub fn weight_serial(&self) -> Option<u32> {
        self.attribute_bits
            .contains(ItemAttrFlags::WEIGHT)
            .then_some(self.weight)
    }

    pub fn set_auto_open(&mut self, value: u8) {
        self.attribute_bits.insert(ItemAttrFlags::AUTO_OPEN);
        self.auto_open = value;
    }

    /// TFS `ATTR_OPEN_CONTAINER` — saved client window id for `Player::autoOpenContainers` (`player.cpp`).
    #[inline]
    pub fn has_auto_open(&self) -> bool {
        self.attribute_bits.contains(ItemAttrFlags::AUTO_OPEN)
    }

    #[inline]
    pub fn get_auto_open(&self) -> u8 {
        if self.has_auto_open() {
            self.auto_open
        } else {
            0
        }
    }

    pub fn set_container_size(&mut self, value: u8) {
        self.attribute_bits.insert(ItemAttrFlags::CONTAINER_SIZE);
        self.container_size = value;
    }

    pub fn set_depot_id(&mut self, value: u16) {
        self.attribute_bits.insert(ItemAttrFlags::DEPOT_ID);
        self.depot_id = value;
    }

    /// `ATTR_CONTAINERSIZE` payload when flag is set.
    #[inline]
    pub fn container_size_serial(&self) -> u8 {
        self.container_size
    }

    // === Custom Attributes ===

    pub fn get_custom_attribute(&self, key: &str) -> Option<&CustomAttrValue> {
        self.custom.as_ref()?.get(key)
    }

    pub fn set_custom_attribute(&mut self, key: impl Into<String>, value: CustomAttrValue) {
        self.attribute_bits.insert(ItemAttrFlags::CUSTOM);
        if self.custom.is_none() {
            self.custom = Some(Box::new(HashMap::new()));
        }
        self.custom.as_mut().unwrap().insert(key.into(), value);
    }

    pub fn remove_custom_attribute(&mut self, key: &str) -> Option<CustomAttrValue> {
        let removed = self.custom.as_mut()?.remove(key);
        if let Some(ref custom) = self.custom {
            if custom.is_empty() {
                self.attribute_bits.remove(ItemAttrFlags::CUSTOM);
            }
        }
        removed
    }

    pub fn custom_attributes(&self) -> Option<&CustomAttributeMap> {
        self.custom.as_deref()
    }

    // === Serialization helpers ===

    pub fn attribute_bits(&self) -> u32 {
        self.attribute_bits.bits()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_attributes() {
        let mut attrs = ItemAttributes::new();

        assert!(!attrs.has_action_id());
        attrs.set_action_id(123);
        assert!(attrs.has_action_id());
        assert_eq!(attrs.get_action_id(), 123);

        assert!(!attrs.has_unique_id());
        attrs.set_unique_id(456);
        assert!(attrs.has_unique_id());
        assert_eq!(attrs.get_unique_id(), 456);
    }

    #[test]
    fn test_string_attributes() {
        let mut attrs = ItemAttributes::new();

        assert_eq!(attrs.get_text(), "");
        attrs.set_text("Hello World");
        assert!(attrs.has_text());
        assert_eq!(attrs.get_text(), "Hello World");

        attrs.reset_text();
        assert!(!attrs.has_text());
        assert_eq!(attrs.get_text(), "");
    }

    #[test]
    fn test_decay_state() {
        let mut attrs = ItemAttributes::new();

        assert_eq!(attrs.get_decaying(), DecayState::False);
        attrs.set_decaying(DecayState::True);
        assert_eq!(attrs.get_decaying(), DecayState::True);
        attrs.set_decaying(DecayState::Pending);
        assert_eq!(attrs.get_decaying(), DecayState::Pending);
    }

    #[test]
    fn test_custom_attributes() {
        let mut attrs = ItemAttributes::new();

        assert!(!attrs.has_custom_attributes());
        attrs.set_custom_attribute("test_key", CustomAttrValue::String("test_value".to_string()));
        assert!(attrs.has_custom_attributes());

        let val = attrs.get_custom_attribute("test_key");
        assert!(matches!(val, Some(CustomAttrValue::String(s)) if s == "test_value"));

        attrs.remove_custom_attribute("test_key");
        assert!(!attrs.has_custom_attributes());
    }

    #[test]
    fn test_fluid_type() {
        let mut attrs = ItemAttributes::new();

        assert!(!attrs.has_fluid_type());
        assert_eq!(attrs.get_fluid_type(), 0);

        attrs.set_fluid_type(5);
        assert!(attrs.has_fluid_type());
        assert_eq!(attrs.get_fluid_type(), 5);
    }
}
