-- Generated from XML. Source: monsters/dworc voodoomaster.xml
return {
  schema = 1,
  name = "Dworc Voodoomaster",
  description = "a dworc voodoomaster",
  race = "blood",
  experience = 50,
  speed = 35,
  mana_cost = 300,
  health = 80,
  max_health = 80,
  outfit = {
    look_type = 214,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4304,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 22,
      attack = 13,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisonfield",
      delay = 8,
      range = 7,
      radius = 1,
      target = true,
      shoot = "poison",
    },
    {
      name = "poison",
      delay = 5,
      min = -6,
      max = -18,
      radius = 6,
      target = false,
      effect = "greenbubble",
    },
    {
      name = "outfit",
      delay = 12,
      range = 7,
      duration = 5000,
      monster = "Chicken",
      effect = "blueshimmer",
    },
    {
      name = "drunk",
      delay = 11,
      range = 7,
      duration = 6000,
      drunkness = 60,
      shoot = "energy",
      effect = "teleport",
    },
    {
      name = "speed",
      delay = 14,
      range = 7,
      duration = 5000,
      speed = -90,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 18,
      min = -1,
      max = -39,
      range = 1,
    },
  },
  defenses = {
    armor = 3,
    defense = 8,
    spells = {
      {
        name = "invisible",
        delay = 22,
        duration = 3000,
        effect = "blueshimmer",
      },
      {
        name = "speed",
        delay = 13,
        duration = 4000,
        speed = 185,
        speed_variation = 15,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 10,
        min = 3,
        max = 9,
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
    invisible = false,
  },
  voices = {
    { text = "Grak brrretz!", yell = false },
    { text = "Grow truk grrrrr.", yell = false },
    { text = "Prek tars, dekklep zurk.", yell = false },
  },
  loot = {
    { id = 3955, chance = 100 }, -- voodoo doll
    { id = 3967, chance = 500 }, -- tribal mask
    { id = 2050, chance = 5500 }, -- torch
    { id = 2174, chance = 500 }, -- strange symbol
    { id = 2229, chance = 3000, count_max = 3 }, -- skull
    { id = 2411, chance = 1000 }, -- poison dagger
    { id = 2467, chance = 10000 }, -- leather armor
    { id = 2148, chance = 80000, count_max = 10 }, -- gold coin
    { id = 2230, chance = 10000 }, -- bone
    { id = 2231, chance = 7000 }, -- big bone
  },
}
