-- Generated from XML. Source: monsters/demodras.xml
return {
  schema = 1,
  name = "Demodras",
  description = "",
  race = "blood",
  experience = 4000,
  speed = 77,
  mana_cost = 0,
  health = 3750,
  max_health = 3750,
  outfit = {
    look_type = 204,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2881,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 10, random = 10 },
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
    run_health = 300,
  },
  attacks = {
    {
      name = "melee",
      skill = 90,
      attack = 80,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "fire",
      delay = 5,
      min = -250,
      max = -550,
      length = 8,
      spread = 3,
      effect = "firearea",
    },
    {
      name = "firefield",
      delay = 10,
      range = 7,
      radius = 6,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 5,
      min = -250,
      max = -350,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
  },
  defenses = {
    armor = 45,
    defense = 70,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 400,
        max = 700,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "ZCHHHHH", yell = true },
    { text = "I WILL SET THE WORLD IN FIRE!", yell = true },
    { text = "I WILL PROTECT MY BROOD!", yell = true },
  },
  summons = {
    max = 2,
    { name = "Dragon Lord", delay = 6, max = 2 },
  },
  loot = {
    { id = 2528, chance = 600 }, -- tower shield
    { id = 2479, chance = 800 }, -- strange helmet
    { id = 2146, chance = 10000 }, -- small sapphire
    { id = 2498, chance = 400 }, -- royal helmet
    { id = 2547, chance = 16000 }, -- power bolt
    { id = 2177, chance = 1200 }, -- life crystal
    { id = 2796, chance = 24000, count_max = 7 }, -- green mushroom
    { id = 2033, chance = 6000 }, -- golden mug
    { id = 2148, chance = 55000, count_max = 50 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 95000, count_max = 100 }, -- gold coin
    { id = 2392, chance = 600 }, -- fire sword
    { id = 2167, chance = 10000 }, -- energy ring
    { id = 2492, chance = 300 }, -- dragon scale mail
    { id = 2672, chance = 75000, count_max = 10 }, -- dragon ham
    { id = 1976, chance = 9000 }, -- book
  },
}
