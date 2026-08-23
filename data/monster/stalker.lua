-- Generated from XML. Source: monsters/stalker.xml
return {
  schema = 1,
  name = "Stalker",
  description = "a stalker",
  race = "blood",
  experience = 90,
  speed = 90,
  mana_cost = 0,
  health = 120,
  max_health = 120,
  outfit = {
    look_type = 128,
    look_head = 97,
    look_body = 116,
    look_legs = 95,
    look_feet = 95,
    corpse = 3058,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 60, weakest = 0, most_damage = 0, random = 40 },
  lose_target = { chance = 10 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 40,
      attack = 30,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 6,
      min = -20,
      max = -30,
      range = 1,
    },
  },
  defenses = {
    armor = 14,
    defense = 20,
    spells = {
      {
        name = "invisible",
        delay = 4,
        duration = 30000,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  loot = {
    { id = 2410, chance = 11000, count_max = 2 }, -- throwing knife
    { id = 2425, chance = 1200 }, -- obsidian lance
    { id = 2649, chance = 10000 }, -- leather legs
    { id = 2412, chance = 6000 }, -- katana
    { id = 2148, chance = 13000, count_max = 8 }, -- gold coin
    { id = 2511, chance = 5500 }, -- brass shield
    { id = 2478, chance = 3500 }, -- brass legs
    { id = 2260, chance = 9000 }, -- blank rune
    { id = 1988, chance = 4500 }, -- backpack
  },
}
