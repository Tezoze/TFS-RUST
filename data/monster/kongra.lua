-- Generated from XML. Source: monsters/kongra.xml
return {
  schema = 1,
  name = "Kongra",
  description = "a kongra",
  race = "blood",
  experience = 110,
  speed = 52,
  mana_cost = 0,
  health = 340,
  max_health = 340,
  outfit = {
    look_type = 116,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4268,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 40,
      attack = 30,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 18,
    defense = 14,
    spells = {
      {
        name = "speed",
        delay = 15,
        duration = 3000,
        speed = 50,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Hugah!", yell = false },
    { text = "Ungh! Ungh!", yell = false },
    { text = "Huaauaauaauaa!", yell = false },
  },
  loot = {
    { id = 2200, chance = 1000 }, -- protection amulet
    { id = 2166, chance = 500 }, -- power ring
    { id = 2463, chance = 1000 }, -- plate armor
    { id = 2148, chance = 10000, count_max = 30 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 25 }, -- gold coin
    { id = 2209, chance = 200 }, -- club ring
    { id = 2676, chance = 5000, count_max = 10 }, -- banana
    { id = 2676, chance = 30000, count_max = 2 }, -- banana
  },
}
