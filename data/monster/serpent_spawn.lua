-- Generated from XML. Source: monsters/serpent spawn.xml
return {
  schema = 1,
  name = "Serpent Spawn",
  description = "a serpent spawn",
  race = "blood",
  experience = 2000,
  speed = 77,
  mana_cost = 0,
  health = 3000,
  max_health = 3000,
  outfit = {
    look_type = 220,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4388,
  },
  change_target = { chance = 25 },
  target_strategy = { nearest = 70, weakest = 30, most_damage = 0, random = 0 },
  lose_target = { chance = 25 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 275,
  },
  attacks = {
    {
      name = "melee",
      skill = 82,
      attack = 62,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poison",
      delay = 6,
      min = -50,
      max = -500,
      length = 8,
      spread = 3,
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 7,
      min = -150,
      max = -400,
      length = 8,
      spread = 0,
      effect = "rednote",
    },
    {
      name = "speed",
      delay = 5,
      range = 7,
      radius = 4,
      duration = 120000,
      speed = -65,
      speed_variation = 10,
      target = true,
      shoot = "poison",
      effect = "greenbubble",
    },
    {
      name = "outfit",
      delay = 120,
      range = 7,
      duration = 4000,
      item = 3492,
      effect = "blueshimmer",
    },
    {
      name = "poison",
      delay = 8,
      min = -100,
      max = -300,
      range = 7,
      shoot = "poison",
      effect = "greenspark",
    },
  },
  defenses = {
    armor = 38,
    defense = 40,
    spells = {
      {
        name = "speed",
        delay = 12,
        duration = 3000,
        speed = 95,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 6,
        min = 300,
        max = 400,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Ssssolus for the one", yell = false },
    { text = "HISSSS", yell = true },
    { text = "Tsssse one will risssse again", yell = false },
    { text = "I bring you deathhhh, mortalssss", yell = false },
  },
  loot = {
    { id = 3428, chance = 400 }, -- tower shield
    { id = 3373, chance = 600 }, -- strange helmet
    { id = 3029, chance = 6000 }, -- small sapphire
    { id = 3392, chance = 100 }, -- royal helmet
    { id = 3450, chance = 6000 }, -- power bolt
    { id = 4831, chance = 500 }, -- old parchment
    { id = 3052, chance = 3000 }, -- life ring
    { id = 3061, chance = 800 }, -- life crystal
    { id = 3732, chance = 18000 }, -- green mushroom
    { id = 3066, chance = 1000 }, -- snakebite rod
    { id = 2903, chance = 3000 }, -- golden mug
    { id = 3031, chance = 40000, count_max = 50 }, -- gold coin
    { id = 3031, chance = 60000, count_max = 100 }, -- gold coin
    { id = 3031, chance = 80000, count_max = 100 }, -- gold coin
    { id = 3280, chance = 300 }, -- fire sword
    { id = 3051, chance = 3000 }, -- energy ring
    { id = 3386, chance = 200 }, -- dragon scale mail
    { id = 3407, chance = 2000 }, -- charmer's tiara
    { id = 2842, chance = 9000 }, -- book
  },
}
