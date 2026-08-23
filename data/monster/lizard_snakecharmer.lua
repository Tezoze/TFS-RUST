-- Generated from XML. Source: monsters/lizard snakecharmer.xml
return {
  schema = 1,
  name = "Lizard Snakecharmer",
  description = "a lizard snakecharmer",
  race = "blood",
  experience = 200,
  speed = 52,
  mana_cost = 0,
  health = 325,
  max_health = 325,
  outfit = {
    look_type = 115,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4262,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = true,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 28,
      attack = 18,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poison",
      delay = 6,
      min = -50,
      max = -100,
      range = 7,
      radius = 1,
      target = true,
      shoot = "poison",
      effect = "greenbubble",
    },
    {
      name = "poisoncondition",
      delay = 9,
      range = 7,
      cycle = 170,
      min_cycle = 30,
      shoot = "poison",
      effect = "poison",
    },
  },
  defenses = {
    armor = 22,
    defense = 15,
    spells = {
      {
        name = "healing",
        delay = 3,
        min = 50,
        max = 100,
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
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Shhhhhhhh.", yell = false },
    { text = "I ssssmell warm blood!", yell = false },
  },
  summons = {
    max = 6,
    { name = "Cobra", delay = 4, max = 6 },
  },
  loot = {
    { id = 2154, chance = 200 }, -- yellow gem
    { id = 2182, chance = 100 }, -- snakebite rod
    { id = 2150, chance = 500 }, -- small amethyst
    { id = 2181, chance = 1000 }, -- quagmire rod
    { id = 2168, chance = 200 }, -- life ring
    { id = 2177, chance = 1000 }, -- life crystal
    { id = 2148, chance = 80000, count_max = 25 }, -- gold coin
    { id = 2237, chance = 19900 }, -- dirty cape
    { id = 2817, chance = 70000 }, -- dead snake
    { id = 3971, chance = 100 }, -- charmer's tiara
    { id = 2654, chance = 9000 }, -- cape
  },
}
