//! Monster definitions from `data/monster/*.lua` (Lua-as-data → [`MonsterType`]).
//! Domain: `src/monsters.cpp` `Monsters::loadMonster`, `loadLootItem`, `deserializeSpell`.
//! Runtime load: [`crate::monster_lua::load_monster_dir`].

use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::Result;

use crate::items::ItemDatabase;

/// Same cap as TFS `MAX_LOOTCHANCE` (`src/monsters.h`).
pub const MAX_LOOTCHANCE: i32 = 100_000;

#[derive(Debug, Clone)]
pub struct LootBlock {
    pub id: u32,
    pub countmax: i32,
    pub chance: i32,
    pub sub_type: i32,
    pub action_id: i32,
    pub text: String,
    pub child_loot: Vec<LootBlock>,
}

#[derive(Debug, Clone)]
pub struct MonsterSpellNode {
    /// Element local name, e.g. `attack`, `defense`, `melee`.
    pub element: String,
    pub attributes: HashMap<String, String>,
    /// Nested `<attribute key="..." value="..."/>` pairs.
    pub attribute_children: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct MonsterDefenses {
    pub armor: Option<i32>,
    pub defense: Option<i32>,
    pub spells: Vec<MonsterSpellNode>,
    /// `<immunity poison="1"/>` — `crmain.cc:548` `RaceData[Race].NoPoison`.
    pub immunity_poison: bool,
    /// `<immunity fire="1"/>` — `crmain.cc:549` `RaceData[Race].NoBurning`.
    pub immunity_fire: bool,
    /// `<immunity energy="1"/>` — `crmain.cc:550` `RaceData[Race].NoEnergy`.
    pub immunity_energy: bool,
    /// `<immunity lifedrain="1"/>` — `crmain.cc:619` `RaceData[Race].NoLifeDrain`. PC-3 (M3′):
    /// non-physical immunity for `DAMAGE_LIFEDRAIN`; `Damage(LIFEDRAIN)` emits `EFFECT_BLOCK_HIT`
    /// and returns 0.
    pub immunity_life_drain: bool,
    /// `<immunity invisible="1"/>` — `crmain.cc:1493` `RaceData[Race].SeeInvisible`.
    pub see_invisible: bool,
    /// `<immunity physical="1"/>` — `crmain.cc:615` `RaceData[Race].NoHit`. Physical damage
    /// immunity: `Damage(PHYSICAL)` emits `EFFECT_BLOCK_HIT` and returns 0.
    pub immunity_physical: bool,
    /// XML `paralyze`; 772 `NoParalyze` (`crmain.cc:1515` / `magic.cc` speed impact).
    pub immunity_paralyze: bool,
    /// XML `outfit`; TFS/TVP surface — no 772 `RaceData` twin; store for the data pack.
    pub immunity_outfit: bool,
}

/// Monster `<look>` block — C++ `MonsterType` look fields (`monsters.cpp` `loadMonster`).
#[derive(Debug, Clone)]
pub struct MonsterOutfit {
    pub look_type: i32,
    pub look_head: i32,
    pub look_body: i32,
    pub look_legs: i32,
    pub look_feet: i32,
    pub look_addons: i32,
    pub look_type_ex: i32,
    pub look_mount: i32,
    /// TVP `<look corpse="…">` — race corpse item id (`crmain.cc:204`).
    pub corpse_id: u16,
}

impl Default for MonsterOutfit {
    fn default() -> Self {
        Self {
            look_type: 136,
            look_head: 0,
            look_body: 0,
            look_legs: 0,
            look_feet: 0,
            look_addons: 0,
            look_type_ex: 0,
            look_mount: 0,
            corpse_id: 0,
        }
    }
}

/// AI/movement flags from `<flags>` — C++ `MonsterType` (`monsters.h`).
#[derive(Debug, Clone, Copy)]
pub struct MonsterTypeFlags {
    /// `<flag targetdistance=…>` — default 1 (`monsters.h`).
    pub target_distance: i32,
    /// `<flag runonhealth=…>` — default 0.
    pub run_away_health: i32,
    /// `<flag staticattack=…>` — default 95.
    pub static_attack_chance: u32,
    pub can_push_creatures: bool,
    pub can_push_items: bool,
    /// `<flag pushable=…>` — default true (`monsters.h`); forced false when `can_push_creatures`.
    pub pushable: bool,
    /// `<flag hostile=…>` — default true for wild monsters.
    pub is_hostile: bool,
    /// `<flag illusionable=…>` — default false (`monsters.h`).
    pub illusionable: bool,
    /// `<flag challengeable=…>` — default true (`monsters.h`).
    pub is_challengeable: bool,
    /// `<flag summonable=…>` — default false (`monsters.h`).
    pub summonable: bool,
    /// `<flag convinceable=…>` — default false (`monsters.h`).
    pub convinceable: bool,
    /// `<targetchange interval/speed=…>` — default 0 (`monsters.h`).
    pub change_target_speed: u32,
    /// `<targetchange chance=…>` — default 0.
    pub change_target_chance: i32,
    /// `<losetarget chance=…>` — 772 `RaceData.LoseTarget` (`crmain.cc:1244`, `:1473`).
    pub lose_target_percent: u8,
    /// `<targetstrategy nearest=…>` — `Strategy[0]` (`crmain.cc:1245`, default 100).
    pub strategy_nearest: u8,
    /// `<targetstrategy weakest=…>` — `Strategy[1]`.
    pub strategy_health: u8,
    /// `<targetstrategy mostdamage=…>` — `Strategy[2]`.
    pub strategy_damage: u8,
    /// `<targetstrategy random=…>` — residual `Strategy[3]` (never compared; leftover bucket).
    pub strategy_random: u8,
}

impl Default for MonsterTypeFlags {
    fn default() -> Self {
        Self {
            target_distance: 1,
            run_away_health: 0,
            static_attack_chance: 95,
            can_push_creatures: false,
            can_push_items: false,
            pushable: true,
            is_hostile: true,
            illusionable: false,
            is_challengeable: true,
            summonable: false,
            convinceable: false,
            change_target_speed: 0,
            change_target_chance: 0,
            lose_target_percent: 0,
            strategy_nearest: 100,
            strategy_health: 0,
            strategy_damage: 0,
            strategy_random: 0,
        }
    }
}

/// One `<summon>` entry under `<summons>` — TVP/TFS XML domain; CASTING maps to 772
/// `IMPACT_SUMMON` (`crnonpl.cc:2647`, `magic.cc` `TSummonImpact`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummonBlock {
    pub name: String,
    /// Cast gate modulus — XML `delay` (default 1); 772 `SpellData.Delay` / `rand() % Delay`.
    pub delay: i32,
    /// Cap for `SummonedCreatures < Maximum` — XML `max` (defaults to parent `maxSummons`).
    pub max: u32,
    /// Passed to place-creature when set — XML `force`.
    pub force: bool,
    /// XML `chance` (default 100); unused by 772 CASTING (delay modulus only).
    pub chance: i32,
}

#[derive(Debug, Clone)]
pub struct MonsterType {
    pub name: String,
    pub filename: String,
    pub name_description: String,
    pub race: String,
    pub experience: u32,
    pub speed: u32,
    pub health_now: u32,
    pub health_max: u32,
    pub outfit: MonsterOutfit,
    pub flags: MonsterTypeFlags,
    /// XML `manacost=` — summon/convince mana (`monsters.h` `manaCost`).
    pub mana_cost: u32,
    pub loot: Vec<LootBlock>,
    pub attack_spells: Vec<MonsterSpellNode>,
    pub defenses: MonsterDefenses,
    /// `<summons maxSummons=…>` — cap 100 (`monsters.cpp`).
    pub max_summons: u32,
    /// `<summons><summon …/></summons>` — merged into CASTING as `SpellImpact::Summon`.
    pub summons: Vec<SummonBlock>,
    /// `<voices><voice sentence="…"/></voices>` — 772 `RaceData.Talk` list (`crnonpl.cc:2442`).
    /// Empty when the monster has no `<voices>` block; the idle talk gate still draws `rand()%50`
    /// + `random(1, Talks)` for RNG parity but emits no packet (matches C++ `Talks == 0` return).
    ///   Each entry may carry a `#y `/`#Y ` prefix (decompile yell marker, `crnonpl.cc:2450`) — the
    ///   idle talk path strips it and switches to `TALKTYPE_MONSTER_YELL` on hit.
    pub talk_texts: Vec<String>,
}

impl MonsterType {
    /// Blood family for on-hit effect + splash, derived from the XML `race` attribute.
    /// See [`tfs_rust_common::enums::BloodType::from_race_str`].
    pub fn blood_type(&self) -> tfs_rust_common::enums::BloodType {
        tfs_rust_common::enums::BloodType::from_race_str(&self.race)
    }
}

pub struct MonsterDatabase {
    pub monsters: HashMap<String, MonsterType>,
}

impl MonsterDatabase {
    /// Load `data/monster/**/*.lua` (skip `#` in filename). Lookup key = Lua `name`.
    pub fn load_dir(dir: &Path, items: &ItemDatabase) -> Result<Self> {
        crate::monster_lua::load_monster_dir(dir, items)
    }

    /// Lookup by index name (case-insensitive) — C++ `Monsters::getMonsterType`.
    pub fn get_by_name(&self, name: &str) -> Option<&MonsterType> {
        self.monsters.get(&name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::ItemDatabase;
    use crate::monster_lua::parse_monster_lua;
    use std::path::PathBuf;

    fn empty_items() -> ItemDatabase {
        ItemDatabase {
            items: HashMap::new(),
            client_to_server: HashMap::new(),
        }
    }

    fn parse_lua(src: &str, file: &str) -> MonsterType {
        parse_monster_lua(src, file, &empty_items()).expect("parse lua")
    }

    #[test]
    fn parses_monster_ai_flags() {
        let src = r#"
return {
  schema = 1,
  name = "Test",
  speed = 100,
  health = 20,
  max_health = 20,
  flags = {
    target_distance = 4,
    run_health = 5,
    static_attack = 90,
    can_push_creatures = true,
    can_push_items = true,
    hostile = true,
  },
}
"#;
        let m = parse_lua(src, "test.lua");
        assert_eq!(m.flags.target_distance, 4);
        assert_eq!(m.flags.run_away_health, 5);
        assert_eq!(m.flags.static_attack_chance, 90);
        assert!(m.flags.can_push_creatures);
        assert!(m.flags.can_push_items);
        assert!(m.flags.is_hostile);
        assert!(
            !m.flags.pushable,
            "can_push_creatures forces pushable false"
        );
    }

    #[test]
    fn parses_targetstrategy_and_losetarget() {
        let src = r#"
return {
  schema = 1,
  name = "Ferumbras",
  speed = 100,
  health = 20,
  max_health = 20,
  target_strategy = { nearest = 60, weakest = 5, most_damage = 30, random = 5 },
  lose_target = { chance = 10 },
}
"#;
        let m = parse_lua(src, "ferumbras.lua");
        assert_eq!(m.flags.strategy_nearest, 60);
        assert_eq!(m.flags.strategy_health, 5);
        assert_eq!(m.flags.strategy_damage, 30);
        assert_eq!(m.flags.strategy_random, 5);
        assert_eq!(m.flags.lose_target_percent, 10);
    }

    #[test]
    fn parses_monster_summons_block() {
        let src = r#"
return {
  schema = 1,
  name = "Giant Spider",
  speed = 80,
  health = 1300,
  max_health = 1300,
  summons = {
    max = 2,
    { name = "Poison Spider", max = 2, delay = 10 },
  },
}
"#;
        let m = parse_lua(src, "giant_spider.lua");
        assert_eq!(m.max_summons, 2);
        assert_eq!(m.summons.len(), 1);
        assert_eq!(m.summons[0].name, "Poison Spider");
        assert_eq!(m.summons[0].delay, 10);
        assert_eq!(m.summons[0].max, 2);
    }

    #[test]
    fn index_name_is_lookup_key_not_file_name_attr() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("monster/red_butterfly.lua").is_file() {
            return;
        }
        let db = MonsterDatabase::load_dir(&data.join("monster"), &empty_items())
            .expect("load monsters");
        let red = db
            .monsters
            .get("red butterfly")
            .expect("lookup key from lua name");
        assert_eq!(red.name, "Butterfly", "display name comes from lua title");
        assert!(
            !db.monsters.contains_key("butterfly"),
            "title must not be the lookup key"
        );
    }

    #[test]
    fn voice_yell_prefixes_talk_text() {
        let src = r#"
return {
  schema = 1,
  name = "Yeller",
  speed = 100,
  health = 20,
  max_health = 20,
  voices = {
    { text = "GROOAAARRR", yell = true },
    { text = "hello", yell = false },
  },
}
"#;
        let m = parse_lua(src, "yeller.lua");
        assert_eq!(
            m.talk_texts,
            vec!["#y GROOAAARRR".to_string(), "hello".to_string()]
        );
    }

    #[test]
    fn parses_all_eight_immunity_flags() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/monster");
        if !dir.join("amazon.lua").is_file() {
            return;
        }
        let db = MonsterDatabase::load_dir(&dir, &empty_items()).expect("load");
        let amazon = db.get_by_name("amazon").expect("amazon");
        assert!(!amazon.defenses.immunity_fire);
        assert!(!amazon.defenses.immunity_energy);
        assert!(!amazon.defenses.immunity_poison);
        assert!(!amazon.defenses.immunity_physical);
        assert!(!amazon.defenses.immunity_outfit);
        assert!(!amazon.defenses.immunity_life_drain);
        assert!(!amazon.defenses.immunity_paralyze);
        assert!(!amazon.defenses.see_invisible);

        let scarab = db.get_by_name("ancient scarab").expect("ancient scarab");
        assert!(scarab.defenses.immunity_paralyze);
        assert!(scarab.defenses.immunity_outfit);
        assert!(scarab.defenses.immunity_life_drain);
        assert!(scarab.defenses.see_invisible);

        let dragon = db.get_by_name("dragon").expect("dragon");
        assert!(dragon.defenses.immunity_paralyze);
        assert!(!dragon.defenses.immunity_outfit, "dragon.lua outfit=false");
        assert!(dragon.defenses.immunity_fire);
        assert!(dragon.defenses.see_invisible);
    }
}
