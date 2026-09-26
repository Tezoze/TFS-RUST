-- Generated from XML. Source: monsters/dark monk.xml
return {
  schema = 1,
  name = "Dark Monk",
  description = "a dark monk",
  race = "blood",
  experience = 145,
  speed = 75,
  mana_cost = 480,
  health = 190,
  max_health = 190,
  outfit = {
    look_type = 225,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4240,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 20, most_damage = 10, random = 0 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 37,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 9,
      min = -25,
      max = -49,
      range = 1,
    },
  },
  defenses = {
    armor = 22,
    defense = 43,
    spells = {
      {
        name = "speed",
        delay = 10,
        duration = 2000,
        speed = 55,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 9,
        min = 25,
        max = 49,
        effect = "blueshimmer",
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
    invisible = true,
  },
  voices = {
    { text = "This is where your path will end!", yell = false },
    { text = "Your end has come.", yell = false },
    { text = "You are no match for us!", yell = false },
  },
  loot = {
    { id = 3289, chance = 11000 }, -- staff
    { id = 2815, chance = 20000 }, -- scroll
    { id = 3551, chance = 8000 }, -- sandals
    { id = 3050, chance = 100 }, -- power ring
    { id = 2914, chance = 10000 }, -- lamp
    { id = 3061, chance = 1000 }, -- life crystal
    { id = 3361, chance = 5500 }, -- leather armor
    { id = 3031, chance = 15000, count_max = 18 }, -- gold coin
    { id = 2853, chance = 13000 }, -- bag
    { id = 2885, chance = 9000 }, -- brown flask
    { id = 3600, chance = 20000 }, -- bread
    { id = 3077, chance = 100 }, -- ankh
  },
}
