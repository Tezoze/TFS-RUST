-- Generated from XML. Source: monsters/hydra.xml
return {
  schema = 1,
  name = "Hydra",
  description = "a hydra",
  race = "blood",
  experience = 2100,
  speed = 60,
  mana_cost = 0,
  health = 2250,
  max_health = 2250,
  outfit = {
    look_type = 121,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4283,
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
      skill = 71,
      attack = 56,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poison",
      delay = 9,
      min = -60,
      max = -300,
      length = 8,
      spread = 3,
      effect = "poison",
    },
    {
      name = "physical",
      delay = 7,
      min = -100,
      max = -200,
      length = 8,
      spread = 3,
      effect = "bluebubble",
    },
    {
      name = "speed",
      delay = 6,
      range = 7,
      radius = 4,
      duration = 15000,
      speed = -80,
      speed_variation = 40,
      target = true,
      shoot = "poison",
      effect = "greenbubble",
    },
  },
  defenses = {
    armor = 27,
    defense = 45,
    spells = {
      {
        name = "healing",
        delay = 3,
        min = 200,
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
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "FCHHHHH", yell = true },
    { text = "HISSSS", yell = true },
  },
  loot = {
    { id = 2475, chance = 1000 }, -- warrior helmet
    { id = 2197, chance = 800 }, -- stone skin amulet
    { id = 2146, chance = 5000 }, -- small sapphire
    { id = 2498, chance = 200 }, -- royal helmet
    { id = 2214, chance = 1200 }, -- ring of healing
    { id = 2536, chance = 100 }, -- medusa shield
    { id = 2666, chance = 90000, count_max = 4 }, -- meat
    { id = 2177, chance = 600 }, -- life crystal
    { id = 2476, chance = 1000 }, -- knight armor
    { id = 4850, chance = 900 }, -- hydra egg
    { id = 2671, chance = 60000, count_max = 3 }, -- ham
    { id = 2148, chance = 40000, count_max = 50 }, -- gold coin
    { id = 2148, chance = 60000, count_max = 100 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 100 }, -- gold coin
    { id = 2195, chance = 100 }, -- boots of haste
  },
}
