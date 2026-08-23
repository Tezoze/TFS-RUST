-- Generated from XML. Source: monsters/vampire.xml
return {
  schema = 1,
  name = "Vampire",
  description = "a vampire",
  race = "undead",
  experience = 290,
  speed = 70,
  mana_cost = 0,
  health = 450,
  max_health = 450,
  outfit = {
    look_type = 68,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2956,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 30, most_damage = 0, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 65,
      attack = 42,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "speed",
      delay = 9,
      range = 7,
      duration = 30000,
      speed = -70,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -55,
      max = -105,
      range = 1,
    },
  },
  defenses = {
    armor = 27,
    defense = 38,
    spells = {
      {
        name = "outfit",
        delay = 120,
        duration = 6000,
        monster = "Bat",
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "BLOOD!", yell = true },
    { text = "Let me kiss your neck.", yell = false },
    { text = "I smell warm blood.", yell = false },
    { text = "I call you, my bats! Come!", yell = false },
  },
  loot = {
    { id = 2534, chance = 100 }, -- vampire shield
    { id = 2479, chance = 400 }, -- strange helmet
    { id = 2383, chance = 1000 }, -- spike sword
    { id = 2229, chance = 10000 }, -- skull
    { id = 2649, chance = 8000 }, -- leather legs
    { id = 2412, chance = 15000 }, -- katana
    { id = 2396, chance = 300 }, -- ice rapier
    { id = 2747, chance = 18000 }, -- grave flower
    { id = 2148, chance = 15000, count_max = 20 }, -- gold coin
    { id = 2127, chance = 200 }, -- emerald bangle
    { id = 2172, chance = 200 }, -- bronze amulet
    { id = 2032, chance = 11000 }, -- bowl
    { id = 2144, chance = 1500 }, -- black pearl
  },
}
