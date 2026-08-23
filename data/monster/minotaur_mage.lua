-- Generated from XML. Source: monsters/minotaur mage.xml
return {
  schema = 1,
  name = "Minotaur Mage",
  description = "a minotaur mage",
  race = "blood",
  experience = 150,
  speed = 45,
  mana_cost = 0,
  health = 155,
  max_health = 155,
  outfit = {
    look_type = 23,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2866,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 15,
    },
    {
      name = "energyfield",
      delay = 9,
      range = 7,
      radius = 1,
      target = true,
      shoot = "energy",
    },
    {
      name = "fire",
      delay = 11,
      min = -35,
      max = -95,
      range = 7,
      radius = 1,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "energy",
      delay = 6,
      min = -15,
      max = -45,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
  },
  defenses = {
    armor = 18,
    defense = 40,
  },
  immunities = {
    fire = false,
    energy = true,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Learrn tha secrret uf deathhh!", yell = true },
    { text = "Kaplar!", yell = false },
  },
  loot = {
    { id = 2189, chance = 500 }, -- wand of cosmic energy
    { id = 2050, chance = 30000, count_max = 2 }, -- torch
    { id = 2649, chance = 15000 }, -- leather legs
    { id = 2461, chance = 10000 }, -- leather helmet
    { id = 2403, chance = 10000 }, -- knife
    { id = 2148, chance = 80000, count_max = 10 }, -- gold coin
    { id = 2817, chance = 70000 }, -- dead snake
    { id = 2404, chance = 4000 }, -- combat knife
    { id = 2648, chance = 2000 }, -- chain legs
    { id = 2684, chance = 10000, count_max = 7 }, -- carrot
    { id = 2465, chance = 4000 }, -- brass armor
  },
}
