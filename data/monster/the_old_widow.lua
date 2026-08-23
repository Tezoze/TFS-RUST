-- Generated from XML. Source: monsters/the old widow.xml
return {
  schema = 1,
  name = "The Old Widow",
  description = "",
  race = "venom",
  experience = 2800,
  speed = 99,
  mana_cost = 0,
  health = 3550,
  max_health = 3550,
  outfit = {
    look_type = 208,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2857,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 70, weakest = 20, most_damage = 0, random = 10 },
  lose_target = { chance = 10 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
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
      skill = 120,
      attack = 95,
      poison_cycles = 450,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poisonfield",
      delay = 11,
      range = 7,
      radius = 4,
      target = true,
      shoot = "poison",
    },
    {
      name = "speed",
      delay = 5,
      range = 7,
      duration = 25000,
      speed = -90,
      speed_variation = 10,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "poison",
      delay = 7,
      min = -250,
      max = -300,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
  },
  defenses = {
    armor = 45,
    defense = 60,
    spells = {
      {
        name = "speed",
        delay = 13,
        duration = 6000,
        speed = 155,
        speed_variation = 45,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 6,
        min = 225,
        max = 275,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  summons = {
    max = 2,
    { name = "Giant Spider", delay = 8, max = 2 },
  },
  loot = {
    { id = 2169, chance = 1400 }, -- time ring
    { id = 2457, chance = 10000 }, -- steel helmet
    { id = 2171, chance = 200 }, -- platinum amulet
    { id = 2463, chance = 20000 }, -- plate armor
    { id = 2477, chance = 600 }, -- knight legs
    { id = 2476, chance = 600 }, -- knight armor
    { id = 2148, chance = 99900, count_max = 22 }, -- gold coin
    { id = 2148, chance = 99900, count_max = 66 }, -- gold coin
    { id = 2148, chance = 66600, count_max = 77 }, -- gold coin
    { id = 2478, chance = 16000 }, -- brass legs
  },
}
