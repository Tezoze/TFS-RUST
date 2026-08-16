//! Client item look text — C++ `Item::getDescription` (`src/item.cpp` ~939–1574).
//! Used for `playerLookAt` before Lua `EventCallback::onLook` wraps `"You see " ..` (`default_onLook.lua`).

use tfs_rust_common::Position;
use tfs_rust_common::enums::Skill;
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
    it.get_plural_name()
}

/// `Item::getPluralName` — `src/item.h` ~960–965.
fn item_plural_name(item: &Item, it: &ItemType) -> String {
    if let Some(p) = item
        .attributes
        .as_deref()
        .and_then(|a| a.get_plural_name_str())
    {
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
        .as_deref()
        .and_then(|a| a.get_name_str())
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
            .as_deref()
            .and_then(|a| a.get_article_str())
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
    item.attributes
        .as_deref()
        .and_then(|a| a.get_attack())
        .unwrap_or(it.attack)
}

#[inline]
fn eff_defense(item: &Item, it: &ItemType) -> i32 {
    item.attributes
        .as_deref()
        .and_then(|a| a.get_defense())
        .unwrap_or(it.defense)
}

#[inline]
fn eff_extra_defense(item: &Item, it: &ItemType) -> i32 {
    item.attributes
        .as_deref()
        .and_then(|a| a.get_extra_defense())
        .unwrap_or(it.extra_defense)
}

#[inline]
fn eff_attack_speed(item: &Item, it: &ItemType) -> u32 {
    let v = item
        .attributes
        .as_deref()
        .map(|a| a.get_attack_speed())
        .unwrap_or(0);
    if v != 0 { v } else { it.attack_speed }
}

#[inline]
fn eff_shoot_range(item: &Item, it: &ItemType) -> i32 {
    item.attributes
        .as_deref()
        .and_then(|a| a.get_shoot_range_attr())
        .unwrap_or(it.shoot_range)
}

#[inline]
fn eff_hit_chance(item: &Item, it: &ItemType) -> i32 {
    item.attributes
        .as_deref()
        .and_then(|a| a.get_hit_chance_attr())
        .unwrap_or(i32::from(it.hit_chance))
}

#[inline]
fn eff_armor(item: &Item, it: &ItemType) -> i32 {
    item.attributes
        .as_deref()
        .and_then(|a| a.get_armor())
        .unwrap_or(it.armor)
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
        parts.push(format!("Atk Spd:{:.1}s", f64::from(atk_spd) / 1000.0));
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

/// Fluid-container look arm — TFS `item.cpp` ~1407–1413 / TVP `item.cpp` ~1026–1031.
/// `fluid_type_name` is `items[subType].name` when `sub_type > 0`.
fn fluid_container_suffix(sub_type: u16, fluid_type_name: Option<&str>) -> String {
    if sub_type > 0 {
        let name = fluid_type_name
            .filter(|n| !n.is_empty())
            .unwrap_or("unknown");
        format!(" of {name}")
    } else {
        ". It is empty".to_string()
    }
}

/// Splash look arm — TFS `item.cpp` ~1414–1421.
fn splash_suffix(sub_type: u16, fluid_type_name: Option<&str>) -> String {
    let name = if sub_type > 0 {
        fluid_type_name
            .filter(|n| !n.is_empty())
            .unwrap_or("unknown")
    } else {
        "unknown"
    };
    format!(" of {name}")
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
    if item
        .attributes
        .as_deref()
        .is_some_and(|a| a.has_unique_id())
    {
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

/// Rune look vocation names — C++ `Item::getDescription` rune arm (`item.cpp` ~960–985).
/// Empty → `"players"`. Names lowercased + pluralized like C++ `asLowerCaseString` + `'s'`.
fn build_rune_vocation_list(names: &[String]) -> String {
    let pluralized: Vec<String> = names
        .iter()
        .map(|n| pluralize_vocation_name(&n.to_ascii_lowercase()))
        .collect();
    match pluralized.len() {
        0 => "players".to_string(),
        1 => pluralized[0].clone(),
        2 => format!("{} and {}", pluralized[0], pluralized[1]),
        _ => {
            let (last, rest) = pluralized.split_last().expect("len checked");
            format!("{} and {}", rest.join(", "), last)
        }
    }
}

/// C++ `Item::getDescription` allowDistRead arm — `item.cpp` ~1422–1449.
/// Ids 7369–7371 (paper messages) are handled later; excluded here like C++.
fn allow_dist_read_suffix(item: &Item, it: &ItemType, look_distance: i32) -> Option<String> {
    if !it.allow_dist_read() || (it.id >= 7369 && it.id <= 7371) {
        return None;
    }
    let mut s = String::from(".\n");
    if look_distance > 4 {
        s.push_str("You are too far away to read it");
        return Some(s);
    }
    let text = item.text();
    if text.is_empty() {
        s.push_str("Nothing is written on it");
        return Some(s);
    }
    let writer = item
        .attributes
        .as_deref()
        .map(|a| a.get_writer())
        .unwrap_or("");
    if !writer.is_empty() {
        s.push_str(writer);
        s.push_str(" wrote");
        let date = item
            .attributes
            .as_deref()
            .map(|a| a.get_date())
            .unwrap_or(0);
        if date != 0 {
            s.push_str(" on ");
            s.push_str(&format_date_short(date));
        }
        s.push_str(": ");
    } else {
        s.push_str("You read: ");
    }
    s.push_str(text);
    Some(s)
}

/// C++ `formatDateShort` — `tools.cpp:362` (`strftime` `%d %b %Y`).
fn format_date_short(unix_secs: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let Some(instant) = UNIX_EPOCH.checked_add(Duration::from_secs(unix_secs.max(0) as u64)) else {
        return String::new();
    };
    let dt: chrono::DateTime<chrono::Local> = instant.into();
    dt.format("%d %b %Y").to_string()
}

/// C++ `Item::getDescription` rune branch — `item.cpp` ~951–1003.
/// Requires `it.rune_level` / `it.rune_mag_level` patched at rune `spell:register()`.
fn rune_description_suffix(it: &ItemType, item: &Item, vocations: &[String]) -> Option<String> {
    if !it.is_rune() || (it.rune_level <= 0 && it.rune_mag_level <= 0) {
        return None;
    }
    let sub_type = i32::from(item.count.max(1));
    let pronoun = if it.stackable() && sub_type > 1 {
        "They"
    } else {
        "It"
    };
    let mut s = format!(
        " (\"{}\"). {} can only be used by {}",
        it.rune_spell_name,
        pronoun,
        build_rune_vocation_list(vocations)
    );
    s.push_str(" with");
    if it.rune_level > 0 {
        use std::fmt::Write;
        let _ = write!(s, " level {}", it.rune_level);
    }
    if it.rune_mag_level > 0 {
        if it.rune_level > 0 {
            s.push_str(" and");
        }
        use std::fmt::Write;
        let _ = write!(s, " magic level {}", it.rune_mag_level);
    }
    s.push_str(" or higher");
    Some(s)
}

/// Full `Item::getDescription(it, lookDistance, item, subType)`.
///
/// `rune_vocations`: vocation names from the registered `RuneSpell` (`None` / empty →
/// `"players"` in the rune arm). Non-rune items ignore this.
///
/// `show_duration_ms`: remaining duration for `showduration` look text (`None` = no duration attr →
/// "brand-new" when `it.show_duration`). Callers pass scheduler remaining when `DecayState::True`.
///
/// `fluid_type_name`: `items[subType].name` for fluid containers / splashes (`None` →
/// `"unknown"` when filled). Non-fluid items ignore this.
pub fn item_get_description_cpp(
    item: &Item,
    it: &ItemType,
    total_weight_hundredths: u32,
    look_distance: i32,
    hydrated_container_capacity: Option<u32>,
    show_duration_ms: Option<i32>,
    rune_vocations: Option<&[String]>,
    fluid_type_name: Option<&str>,
) -> String {
    let mut s = item_name_description(item, it, true);

    let mut allow_dist_read_emitted = false;
    if let Some(rune_sfx) = rune_description_suffix(it, item, rune_vocations.unwrap_or(&[])) {
        s.push_str(&rune_sfx);
    } else if it.weapon_type != WEAPON_NONE {
        if let Some(w) = weapon_suffix(item, it) {
            s.push_str(&w);
        }
    } else if let Some(st) = stats_and_abilities_suffix(item, it) {
        s.push_str(&st);
    } else if let Some(vol) = container_volume_suffix(item, it, hydrated_container_capacity) {
        s.push_str(&vol);
    } else if it.is_fluid_container() {
        // C++ `item.cpp` ~1407–1413 — before allowDistRead / after abilities.
        let sub = item.get_sub_type(it);
        s.push_str(&fluid_container_suffix(sub, fluid_type_name));
    } else if it.is_splash() {
        let sub = item.get_sub_type(it);
        s.push_str(&splash_suffix(sub, fluid_type_name));
    } else if let Some(adr) = allow_dist_read_suffix(item, it, look_distance) {
        s.push_str(&adr);
        allow_dist_read_emitted = true;
    }

    if it.show_charges {
        let charges = item
            .attributes
            .as_deref()
            .map(|a| a.get_charges())
            .unwrap_or(0);
        if charges > 0 {
            let plural = if charges == 1 { "" } else { "s" };
            s.push_str(&format!(" that has {} charge{} left", charges, plural));
        }
    }

    // TFS `item.cpp` ~1463–1498 — `it.showDuration`.
    if it.show_duration {
        match show_duration_ms {
            Some(ms) => {
                s.push_str(" that will expire in ");
                s.push_str(&format_duration_remaining(ms.max(0) as u32 / 1000));
            }
            None => s.push_str(" that is brand-new"),
        }
    }

    // `item.cpp` ~1500–1509 — skip trailing '.' when allowDistRead already printed
    // `.\nYou read: …` with non-empty text (period was included in that branch).
    let skip_period = allow_dist_read_emitted && !item.text().is_empty();
    if !skip_period {
        s.push('.');
    }

    // Paper messages 7369–7371 — `item.cpp` ~1564–1571 (text after weight/desc).
    let paper_msg = it.allow_dist_read() && (7369..=7371).contains(&it.id);

    // 772 `operate.cc:2120-2129`: weight when in range 1, `TAKE`, and `GetWeight > 0`.
    // `corpsetype` rows with XML weight still show oz if OTB omitted `FLAG_PICKUPABLE`.
    if look_distance <= 1
        && total_weight_hundredths != 0
        && (it.pickupable() || it.xml_attributes.contains_key("corpsetype"))
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

    if item
        .attributes
        .as_deref()
        .is_some_and(|a| a.has_description())
        && !item
            .attributes
            .as_deref()
            .expect("has desc")
            .get_description()
            .is_empty()
    {
        s.push('\n');
        s.push_str(item.attributes.as_deref().unwrap().get_description());
    } else if look_distance <= 1 && !it.description.is_empty() {
        s.push('\n');
        s.push_str(&it.description);
    }

    // C++ `item.cpp` ~1564–1571 — paper message text after description.
    if paper_msg {
        let text = item.text();
        if !text.is_empty() {
            s.push('\n');
            s.push_str(text);
        }
    }

    s
}

/// Format remaining seconds for showduration look text (`item.cpp` ~1466–1494).
fn format_duration_remaining(duration_sec: u32) -> String {
    if duration_sec >= 86400 {
        let days = duration_sec / 86400;
        let hours = (duration_sec % 86400) / 3600;
        let mut s = format!("{} day{}", days, if days != 1 { "s" } else { "" });
        if hours > 0 {
            s.push_str(&format!(
                " and {} hour{}",
                hours,
                if hours != 1 { "s" } else { "" }
            ));
        }
        s
    } else if duration_sec >= 3600 {
        let hours = duration_sec / 3600;
        let minutes = (duration_sec % 3600) / 60;
        let mut s = format!("{} hour{}", hours, if hours != 1 { "s" } else { "" });
        if minutes > 0 {
            s.push_str(&format!(
                " and {} minute{}",
                minutes,
                if minutes != 1 { "s" } else { "" }
            ));
        }
        s
    } else if duration_sec >= 60 {
        let minutes = duration_sec / 60;
        let seconds = duration_sec % 60;
        let mut s = format!("{} minute{}", minutes, if minutes != 1 { "s" } else { "" });
        if seconds > 0 {
            s.push_str(&format!(
                " and {} second{}",
                seconds,
                if seconds != 1 { "s" } else { "" }
            ));
        }
        s
    } else {
        format!(
            "{} second{}",
            duration_sec,
            if duration_sec != 1 { "s" } else { "" }
        )
    }
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

        let item = Item::new(it.id, 4);
        let total = 8000u32;
        let s = item_get_description_cpp(&item, &it, total, 1, None, None, None, None);
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

        let item = Item::new(it.id, 1);
        let s = item_get_description_cpp(&item, &it, 3500, 1, None, None, None, None);
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

        let item = Item::new(it.id, 1);
        let s = item_get_description_cpp(&item, &it, 1800, 1, None, None, None, None);
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

        let mut item = Item::new(it.id, 1);
        item.set_charges(50);
        let s = item_get_description_cpp(&item, &it, 500, 1, None, None, None, None);
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

        let item = Item::new_single(it.id);
        let s = item_get_description_cpp(&item, &it, it.weight, 3, None, None, None, None);
        assert_eq!(s, "water.");
    }

    #[test]
    fn showduration_expire_text_and_brand_new() {
        let it = ItemType {
            id: 2169,
            name: "time ring".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            show_duration: true,
            weight: 90,
            ..Default::default()
        };
        let item = Item::new_single(it.id);
        let brand = item_get_description_cpp(&item, &it, 90, 1, None, None, None, None);
        assert!(brand.contains("that is brand-new"), "brand-new: {brand}");

        let with_dur = item_get_description_cpp(&item, &it, 90, 1, None, Some(125_000), None, None);
        assert!(
            with_dur.contains("that will expire in 2 minutes and 5 seconds"),
            "expire: {with_dur}"
        );
    }

    /// C++ `Item::getDescription` rune arm — `item.cpp` ~951–1003.
    #[test]
    fn sudden_death_rune_look_includes_spell_words_and_maglevel() {
        let it = ItemType {
            id: 2268,
            name: "spell rune".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            type_tag: 10, // ITEM_TYPE_RUNE
            weight: 120,
            rune_spell_name: "adori vita vis".into(),
            rune_level: 0,
            rune_mag_level: 15,
            ..Default::default()
        };
        let item = Item::new_single(it.id);
        let s = item_get_description_cpp(&item, &it, 120, 1, None, None, None, None);
        assert!(
            s.starts_with("a spell rune (\"adori vita vis\"). It can only be used by players with magic level 15 or higher."),
            "got: {s}"
        );
        assert!(s.contains("It weighs 1.20 oz."), "weight: {s}");
    }

    /// C++ `item.cpp` ~1407–1413 / TVP `item.cpp` ~1026–1031 — vial of manafluid.
    #[test]
    fn fluid_container_look_shows_of_fluid_name() {
        let it = ItemType {
            id: 2006,
            name: "vial".into(),
            article: "a".into(),
            group: ItemType::GROUP_FLUID,
            flags: FLAG_PICKUPABLE,
            weight: 180,
            ..Default::default()
        };
        // 772 `FLUID_MANAFLUID` = 10; items.xml id 10 = "manafluid".
        let mut item = Item::new(it.id, 10);
        item.set_fluid_type(10);
        let s = item_get_description_cpp(&item, &it, 180, 1, None, None, None, Some("manafluid"));
        assert_eq!(s, "a vial of manafluid.\nIt weighs 1.80 oz.", "got: {s}");
    }

    /// Persistence often stores fluid only in `count` (ATTR_COUNT) — look must still resolve.
    #[test]
    fn fluid_container_look_uses_count_when_fluid_attr_unset() {
        let it = ItemType {
            id: 2006,
            name: "vial".into(),
            article: "a".into(),
            group: ItemType::GROUP_FLUID,
            flags: FLAG_PICKUPABLE,
            weight: 180,
            ..Default::default()
        };
        let item = Item::new(it.id, 10); // no set_fluid_type
        assert_eq!(item.get_sub_type(&it), 10);
        let s = item_get_description_cpp(&item, &it, 180, 1, None, None, None, Some("manafluid"));
        assert!(s.starts_with("a vial of manafluid."), "got: {s}");
    }

    #[test]
    fn fluid_container_look_empty() {
        let it = ItemType {
            id: 2006,
            name: "vial".into(),
            article: "a".into(),
            group: ItemType::GROUP_FLUID,
            flags: FLAG_PICKUPABLE,
            weight: 180,
            ..Default::default()
        };
        let mut item = Item::new(it.id, 0);
        item.set_fluid_type(0);
        let s = item_get_description_cpp(&item, &it, 180, 1, None, None, None, None);
        assert_eq!(s, "a vial. It is empty.\nIt weighs 1.80 oz.", "got: {s}");
    }

    /// C++ `item.cpp` ~1422–1449 — signs / blackboards (`allowDistRead`).
    #[test]
    fn sign_allow_dist_read_shows_you_read_text() {
        let it = ItemType {
            id: 1429,
            name: "sign".into(),
            article: "a".into(),
            allow_dist_read_override: Some(true),
            weight: 0,
            ..Default::default()
        };
        let mut item = Item::new_single(it.id);
        item.set_text("Temple Street");
        let s = item_get_description_cpp(&item, &it, 0, 1, None, None, None, None);
        assert_eq!(s, "a sign.\nYou read: Temple Street");
    }

    #[test]
    fn sign_allow_dist_read_empty_and_far() {
        let it = ItemType {
            id: 1810,
            name: "blackboard".into(),
            article: "a".into(),
            allow_dist_read_override: Some(true),
            ..Default::default()
        };
        let item = Item::new_single(it.id);
        let near = item_get_description_cpp(&item, &it, 0, 1, None, None, None, None);
        assert_eq!(near, "a blackboard.\nNothing is written on it.");
        let far = item_get_description_cpp(&item, &it, 0, 5, None, None, None, None);
        assert_eq!(far, "a blackboard.\nYou are too far away to read it.");
    }

    #[test]
    fn corpse_look_shows_weight_and_killed_by() {
        let mut it = ItemType {
            id: 2813,
            name: "dead rat".into(),
            article: "a".into(),
            flags: FLAG_PICKUPABLE,
            weight: 6300,
            max_items: 5,
            group: ItemType::GROUP_CONTAINER,
            ..Default::default()
        };
        it.xml_attributes
            .insert("corpsetype".into(), "blood".into());
        let item = Item::new(it.id, 1);
        let s = item_get_description_cpp(&item, &it, 6300, 1, Some(5), None, None, None);
        assert!(s.contains("It weighs 63.00 oz."), "weight: {s}");
        assert!(s.contains("(Vol:5)"), "volume: {s}");
    }

    #[test]
    fn corpse_look_shows_weight_without_otb_pickupable() {
        let mut it = ItemType {
            id: 2813,
            name: "dead rat".into(),
            article: "a".into(),
            flags: 0,
            weight: 6300,
            ..Default::default()
        };
        it.xml_attributes
            .insert("corpsetype".into(), "blood".into());
        let item = Item::new(it.id, 1);
        let s = item_get_description_cpp(&item, &it, 6300, 1, None, None, None, None);
        assert!(s.contains("It weighs 63.00 oz."), "weight without FLAG_PICKUPABLE: {s}");
    }

    #[test]
    fn player_corpse_look_appends_killed_by() {
        let mut it = ItemType {
            id: 3128,
            name: "dead human".into(),
            article: "a".into(),
            max_items: 10,
            group: ItemType::GROUP_CONTAINER,
            ..Default::default()
        };
        it.xml_attributes
            .insert("corpsetype".into(), "blood".into());
        let mut item = Item::new(it.id, 1);
        item.set_description("You recognize Alice. He was killed by Bob.");
        let s = item_get_description_cpp(&item, &it, 0, 1, Some(10), None, None, None);
        assert!(
            s.contains("You recognize Alice. He was killed by Bob."),
            "killed-by: {s}"
        );
    }
}
