//! Client item look text — C++ `Item::getDescription` (`src/item.cpp` ~939–1574).
//! Used for `playerLookAt` before Lua `EventCallback::onLook` wraps `"You see " ..` (`default_onLook.lua`).

use tfs_rust_common::enums::Skill;
use tfs_rust_common::Position;
use tfs_rust_content::item_abilities::{
    COMBAT_ABSORB_COUNT, STAT_MAGICPOINTS, STAT_MAXHITPOINTS, STAT_MAXMANAPOINTS,
};
use tfs_rust_content::otb::ItemType;

use crate::item::Item;

// `WeaponType_t` — `src/const.h`
const WEAPON_NONE: u8 = 0;
const WEAPON_DISTANCE: u8 = 5;
const WEAPON_AMMO: u8 = 7;

/// `Game::playerLookAt` look distance — `game.cpp` ~3177–3185.
pub fn look_distance_tfs(player_pos: Position, thing_pos: Position) -> i32 {
    let dx = (player_pos.x as i32 - thing_pos.x as i32).abs();
    let dy = (player_pos.y as i32 - thing_pos.y as i32).abs();
    let mut d = std::cmp::max(dx, dy);
    if player_pos.z != thing_pos.z {
        d += 15;
    }
    d
}

/// `Item::getWeightDescription` formatting — `item.cpp` ~1623–1643.
fn format_weight_oz_tfs(weight: u32) -> String {
    if weight < 10 {
        format!("0.0{}", weight)
    } else if weight < 100 {
        format!("0.{:02}", weight)
    } else {
        let mut s = weight.to_string();
        let len = s.len();
        if len >= 2 {
            s.insert(len - 2, '.');
        }
        s
    }
}

fn weight_description_line(it: &ItemType, total_weight_hundredths: u32, count: u16) -> String {
    let they = it.stackable() && count > 1 && it.show_count;
    let prefix = if they { "They weigh " } else { "It weighs " };
    format!(
        "{}{} oz.",
        prefix,
        format_weight_oz_tfs(total_weight_hundredths)
    )
}

/// `ItemType::getPluralName` — `src/items.h` ~268–286.
fn type_plural_name(it: &ItemType) -> String {
    if !it.plural_name.is_empty() {
        return it.plural_name.clone();
    }
    if !it.show_count {
        return it.name.clone();
    }
    if it.name.is_empty() || it.name.ends_with('s') {
        return it.name.clone();
    }
    format!("{}s", it.name)
}

/// `Item::getPluralName` — `src/item.h` ~960–965.
fn item_plural_name(item: &Item, it: &ItemType) -> String {
    if let Some(p) = item.attributes.get_plural_name_str() {
        return p.to_string();
    }
    type_plural_name(it)
}

/// `Item::getNameDescription` — `src/item.cpp` ~1582–1615.
fn item_name_description(item: &Item, it: &ItemType, add_article: bool) -> String {
    let sub_type = i32::from(item.count.max(1));
    if it.stackable() && sub_type > 1 {
        let mut s = String::new();
        if it.show_count {
            s.push_str(&format!("{sub_type} "));
        }
        s.push_str(&item_plural_name(item, it));
        return s;
    }

    let name = item
        .attributes
        .get_name_str()
        .unwrap_or(it.name.as_str());
    if name.is_empty() {
        return if add_article {
            format!("an item of type {}", it.id)
        } else {
            format!("item of type {}", it.id)
        };
    }

    let mut s = String::new();
    if add_article {
        let art = item
            .attributes
            .get_article_str()
            .filter(|a| !a.is_empty())
            .unwrap_or(it.article.as_str());
        if !art.is_empty() {
            s.push_str(art);
            s.push(' ');
        }
    }
    s.push_str(name);
    s
}

#[inline]
fn eff_attack(item: &Item, it: &ItemType) -> i32 {
    item.attributes.get_attack().unwrap_or(it.attack)
}

#[inline]
fn eff_defense(item: &Item, it: &ItemType) -> i32 {
    item.attributes.get_defense().unwrap_or(it.defense)
}

#[inline]
fn eff_extra_defense(item: &Item, it: &ItemType) -> i32 {
    item.attributes.get_extra_defense().unwrap_or(it.extra_defense)
}

#[inline]
fn eff_attack_speed(item: &Item, it: &ItemType) -> u32 {
    let v = item.attributes.get_attack_speed();
    if v != 0 {
        v
    } else {
        it.attack_speed
    }
}

#[inline]
fn eff_shoot_range(item: &Item, it: &ItemType) -> i32 {
    item
        .attributes
        .get_shoot_range_attr()
        .unwrap_or(it.shoot_range)
}

#[inline]
fn eff_hit_chance(item: &Item, it: &ItemType) -> i32 {
    item
        .attributes
        .get_hit_chance_attr()
        .unwrap_or(i32::from(it.hit_chance))
}

#[inline]
fn eff_armor(item: &Item, it: &ItemType) -> i32 {
    item.attributes.get_armor().unwrap_or(it.armor)
}

/// Ranged weapon with ammunition type — `item.cpp` ~1006–1027.
fn weapon_suffix_distance_ammo(item: &Item, it: &ItemType) -> Option<String> {
    let range = eff_shoot_range(item, it).max(0);
    let attack = eff_attack(item, it);
    let hit = eff_hit_chance(item, it);
    let mut inner = format!("Range:{}", range);
    if attack != 0 {
        use std::fmt::Write;
        let _ = write!(inner, ", Atk{:+}", attack);
    }
    if hit != 0 {
        use std::fmt::Write;
        let _ = write!(inner, ", Hit%{:+}", hit);
    }
    Some(format!(" ({})", inner))
}

/// Melee / distance-without-ammo / wand (non-ammo) — `item.cpp` ~1028–1074 (no `abilities` block).
fn weapon_suffix_non_ammo(item: &Item, it: &ItemType) -> Option<String> {
    let attack = eff_attack(item, it);
    let defense = eff_defense(item, it);
    let extra = eff_extra_defense(item, it);
    let atk_spd = eff_attack_speed(item, it);

    let mut parts: Vec<String> = Vec::new();
    if attack != 0 {
        parts.push(format!("Atk:{}", attack));
    }
    if atk_spd != 0 {
        parts.push(format!(
            "Atk Spd:{:.1}s",
            f64::from(atk_spd) / 1000.0
        ));
    }
    if defense != 0 || extra != 0 {
        let mut d = format!("Def:{}", defense);
        if extra != 0 {
            use std::fmt::Write;
            let _ = write!(d, " {:+}", extra);
        }
        parts.push(d);
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!(" ({})", parts.join(", ")))
    }
}

fn weapon_suffix(item: &Item, it: &ItemType) -> Option<String> {
    if it.weapon_type == WEAPON_NONE {
        return None;
    }
    if it.weapon_type == WEAPON_AMMO {
        return None;
    }
    if it.weapon_type == WEAPON_DISTANCE && it.ammo_type != 0 {
        weapon_suffix_distance_ammo(item, it)
    } else {
        weapon_suffix_non_ammo(item, it)
    }
}

/// C++ `combatTypeToIndex` order — display name for `abilities.absorbPercent[i]` in look text.
fn combat_absorb_display_name(i: usize) -> &'static str {
    match i {
        0 => "physical",
        1 => "energy",
        2 => "earth",
        3 => "fire",
        4 => "undefined",
        5 => "life drain",
        6 => "mana drain",
        7 => "healing",
        8 => "drown",
        9 => "ice",
        10 => "holy",
        11 => "death",
        _ => "unknown",
    }
}

/// Non-weapon suffix: `Arm`, `showAttributes` stats/skills/speed, then protection absorbs — `item.cpp` ~1075+.
fn stats_and_abilities_suffix(item: &Item, it: &ItemType) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let armor = eff_armor(item, it);
    if armor != 0 {
        parts.push(format!("Arm:{}", armor));
    }

    let ab = &it.abilities;
    if it.show_attributes {
        let ml = ab.stats[STAT_MAGICPOINTS];
        if ml != 0 {
            parts.push(format!("magic level {:+}", ml));
        }
        let hp = ab.stats[STAT_MAXHITPOINTS];
        if hp != 0 {
            parts.push(format!("hit points {:+}", hp));
        }
        let mana = ab.stats[STAT_MAXMANAPOINTS];
        if mana != 0 {
            parts.push(format!("mana {:+}", mana));
        }

        let skill_parts: [(Skill, &str); 7] = [
            (Skill::Sword, "sword fighting"),
            (Skill::Club, "club fighting"),
            (Skill::Axe, "axe fighting"),
            (Skill::Distance, "distance fighting"),
            (Skill::Shield, "shielding"),
            (Skill::Fist, "fist fighting"),
            (Skill::Fishing, "fishing"),
        ];
        for (sk, label) in skill_parts {
            let v = ab.skills[sk as usize];
            if v != 0 {
                parts.push(format!("{} {:+}", label, v));
            }
        }

        if ab.speed != 0 {
            parts.push(format!("speed {:+}", ab.speed));
        }
    }

    for i in 0..COMBAT_ABSORB_COUNT {
        let pct = ab.absorb_percent[i];
        if pct != 0 {
            parts.push(format!(
                "protection {} {:+}%",
                combat_absorb_display_name(i),
                pct
            ));
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!(" ({})", parts.join(", ")))
    }
}

fn pluralize_vocation_name(name: &str) -> String {
    if name.ends_with('s') {
        name.to_string()
    } else {
        format!("{name}s")
    }
}

/// Join vocation names like C++ `Item::getDescription` — `item.cpp` ~1400+.
fn build_vocation_list(names: &[String]) -> String {
    let pluralized: Vec<String> = names.iter().map(|n| pluralize_vocation_name(n)).collect();
    match pluralized.len() {
        0 => "players".to_string(),
        1 => pluralized[0].clone(),
        2 => format!("{} and {}", pluralized[0], pluralized[1]),
        _ => {
            let (last, rest) = pluralized.split_last().expect("len checked");
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

/// `(Vol:N)` for containers — `item.cpp` ~1367–1379.
fn container_volume_suffix(
    item: &Item,
    it: &ItemType,
    hydrated_capacity: Option<u32>,
) -> Option<String> {
    let is_container_type = it.group == ItemType::GROUP_CONTAINER;
    if !is_container_type && hydrated_capacity.is_none() {
        return None;
    }
    if item.attributes.has_unique_id() {
        return None;
    }
    let volume = hydrated_capacity.unwrap_or_else(|| {
        if is_container_type {
            u32::from(it.max_items)
        } else {
            0
        }
    });
    if volume == 0 {
        None
    } else {
        Some(format!(" (Vol:{volume})"))
    }
}

fn append_equip_requirements(s: &mut String, it: &ItemType) {
    if it.min_req_level > 0 || !it.voc_equip_names.is_empty() {
        let voc_part = if it.voc_equip_names.is_empty() {
            "players".to_string()
        } else {
            build_vocation_list(&it.voc_equip_names)
        };
        let mut req = format!("\nIt can only be wielded properly by {}", voc_part);
        if it.min_req_level > 0 {
            use std::fmt::Write;
            let _ = write!(req, " of level {} or higher", it.min_req_level);
        }
        req.push('.');
        s.push_str(&req);
    }
    if it.min_req_magic_level > 0 {
        use std::fmt::Write;
        let _ = write!(
            s,
            "\nIt can only be used properly by players of magic level {} or higher.",
            it.min_req_magic_level
        );
    }
}

/// Full `Item::getDescription(it, lookDistance, item, subType)` for normal items (no runes / no spells).
pub fn item_get_description_cpp(
    item: &Item,
    it: &ItemType,
    total_weight_hundredths: u32,
    look_distance: i32,
    hydrated_container_capacity: Option<u32>,
) -> String {
    let mut s = item_name_description(item, it, true);

    if it.weapon_type != WEAPON_NONE {
        if let Some(w) = weapon_suffix(item, it) {
            s.push_str(&w);
        }
    } else if let Some(st) = stats_and_abilities_suffix(item, it) {
        s.push_str(&st);
    } else if let Some(vol) = container_volume_suffix(item, it, hydrated_container_capacity) {
        s.push_str(&vol);
    }

    if it.show_charges {
        let charges = item.attributes.get_charges();
        if charges > 0 {
            let plural = if charges == 1 { "" } else { "s" };
            s.push_str(&format!(" that has {} charge{} left", charges, plural));
        }
    }

    // `item.cpp` ~1500–1509 — full `allowDistRead` / scroll text branching not ported; period always appended here.
    s.push('.');

    if look_distance <= 1
        && total_weight_hundredths != 0
        && it.pickupable()
    {
        s.push('\n');
        s.push_str(&weight_description_line(
            it,
            total_weight_hundredths,
            item.count.max(1),
        ));
    }

    if look_distance <= 1 {
        append_equip_requirements(&mut s, it);
    }

    if item.attributes.has_description() && !item.attributes.get_description().is_empty() {
        s.push('\n');
        s.push_str(item.attributes.get_description());
    } else if look_distance <= 1 && !it.description.is_empty() {
        s.push('\n');
        s.push_str(&it.description);
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ItemId;
    use crate::item::Item;

    const FLAG_STACKABLE: u32 = 1 << 7;
    const FLAG_PICKUPABLE: u32 = 1 << 5;

    #[test]
    fn spear_stack_description_like_tfs_item_cpp() {
        let it = ItemType {
            id: 2389,
            name: "spear".into(),
            article: "a".into(),
            flags: FLAG_STACKABLE | FLAG_PICKUPABLE,
            weapon_type: WEAPON_DISTANCE,
            attack: 25,
            weight: 2000,
            ammo_type: 0,
            ..Default::default()
        };

        let item = Item::new(ItemId::default(), it.id, 4);
        let total = 8000u32;
        let s = item_get_description_cpp(&item, &it, total, 1, None);
        assert_eq!(s, "4 spears (Atk:25).\nThey weigh 80.00 oz.");
    }

    #[test]
    fn weight_format_matches_item_cpp() {
        assert_eq!(format_weight_oz_tfs(8000), "80.00");
        assert_eq!(format_weight_oz_tfs(5), "0.05");
        assert_eq!(format_weight_oz_tfs(50), "0.50");
    }

    /// Regression: armor + `showattributes` magic level + vocation/level lines (`item.cpp` getDescription).
    #[test]
    fn armor_shows_magic_level_and_requirements_like_tfs() {
        use tfs_rust_content::item_abilities::STAT_MAGICPOINTS;

        let mut it = ItemType {
            name: "yalahari mask".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            weight: 3500,
            armor: 5,
            show_attributes: true,
            voc_equip_names: vec!["sorcerer".into(), "druid".into()],
            min_req_level: 80,
            ..Default::default()
        };
        it.abilities.stats[STAT_MAGICPOINTS] = 2;

        let item = Item::new(ItemId::default(), it.id, 1);
        let s = item_get_description_cpp(&item, &it, 3500, 1, None);
        assert_eq!(
            s,
            "a yalahari mask (Arm:5, magic level +2).\n\
It weighs 35.00 oz.\n\
It can only be wielded properly by sorcerers and druids of level 80 or higher."
        );
    }

    /// Regression: container `(Vol:N)` — `item.cpp` ~1367–1379.
    #[test]
    fn backpack_shows_volume_like_tfs() {
        let it = ItemType {
            id: 1988,
            name: "backpack".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            group: ItemType::GROUP_CONTAINER,
            max_items: 20,
            weight: 1800,
            ..Default::default()
        };

        let item = Item::new(ItemId::default(), it.id, 1);
        let s = item_get_description_cpp(&item, &it, 1800, 1, None);
        assert_eq!(s, "a backpack (Vol:20).\nIt weighs 18.00 oz.");
    }

    /// Regression: non-armor absorb + charges + level (`item.cpp` getDescription).
    #[test]
    fn necklace_shows_protection_charges_and_level_requirement() {
        let mut it = ItemType {
            name: "necklace of the deep".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            weight: 500,
            show_charges: true,
            min_req_level: 120,
            ..Default::default()
        };
        it.abilities.absorb_percent[5] = 50; // `CombatType::LifeDrain` index

        let mut item = Item::new(ItemId::default(), it.id, 1);
        item.set_charges(50);
        let s = item_get_description_cpp(&item, &it, 500, 1, None);
        assert_eq!(
            s,
            "a necklace of the deep (protection life drain +50%) that has 50 charges left.\n\
It weighs 5.00 oz.\n\
It can only be wielded properly by players of level 120 or higher."
        );
    }

    /// Ground tiles use ephemeral items — `Item::getDescription` with type-only weight (`item.cpp` ~1548).
    #[test]
    fn ground_water_description() {
        let it = ItemType {
            id: 1,
            name: "water".into(),
            ..Default::default()
        };

        let item = Item::new_single(ItemId::default(), it.id);
        let s = item_get_description_cpp(&item, &it, it.weight, 3, None);
        assert_eq!(s, "water.");
    }
}
