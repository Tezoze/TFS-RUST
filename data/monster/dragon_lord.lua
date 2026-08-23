-- Generated from XML. Source: monsters/dragon lord.xml
return {
  schema = 1,
  name = "Dragon Lord",
  description = "a dragon lord",
  race = "blood",
  experience = 2100,
  speed = 60,
  mana_cost = 0,
  health = 1900,
  max_health = 1900,
  outfit = {
    look_type = 39,
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
      skill = 65,
      attack = 55,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "fire",
      delay = 6,
      min = -150,
      max = -250,
      length = 8,
      spread = 3,
      effect = "firearea",
    },
    {
      name = "firefield",
      delay = 7,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 6,
      min = -120,
      max = -180,
      range = 7,
      radius = 4,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
  },
  defenses = {
    armor = 32,
    defense = 48,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 57,
        max = 93,
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
    { text = "YOU WILL BURN!", yell = true },
  },
  loot = {
    { id = 2528, chance = 300 }, -- tower shield
    { id = 2479, chance = 400 }, -- strange helmet
    { id = 2146, chance = 5000 }, -- small sapphire
    { id = 2498, chance = 200 }, -- royal helmet
    { id = 2547, chance = 6000 }, -- power bolt
    { id = 2177, chance = 600 }, -- life crystal
    { id = 2796, chance = 12000 }, -- green mushroom
    { id = 2033, chance = 3000 }, -- golden mug
    { id = 2148, chance = 40000, count_max = 50 }, -- gold coin
    { id = 2148, chance = 60000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 100 }, -- gold coin
    { id = 2392, chance = 300 }, -- fire sword
    { id = 2167, chance = 5000 }, -- energy ring
    { id = 2492, chance = 100 }, -- dragon scale mail
    { id = 2672, chance = 60000, count_max = 5 }, -- dragon ham
    { id = 1976, chance = 9000 }, -- book
  },
}
