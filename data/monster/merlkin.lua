-- Generated from XML. Source: monsters/merlkin.xml
return {
  schema = 1,
  name = "Merlkin",
  description = "a merlkin",
  race = "blood",
  experience = 135,
  speed = 57,
  mana_cost = 0,
  health = 230,
  max_health = 230,
  outfit = {
    look_type = 117,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4271,
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
      skill = 25,
      attack = 15,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisonfield",
      delay = 7,
      range = 7,
      radius = 1,
      target = true,
      shoot = "energy",
    },
    {
      name = "energy",
      delay = 3,
      min = -15,
      max = -45,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
    {
      name = "fire",
      delay = 8,
      min = -30,
      max = -90,
      range = 7,
      shoot = "fire",
      effect = "firearea",
    },
  },
  defenses = {
    armor = 16,
    defense = 40,
    spells = {
      {
        name = "healing",
        delay = 11,
        min = 10,
        max = 40,
        effect = "blueshimmer",
      },
    },
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
    { text = "Ugh! Ugh! Ugh!", yell = false },
    { text = "Holy banana!", yell = false },
    { text = "Chakka! Chakka!", yell = false },
  },
  loot = {
    { id = 2188, chance = 1000 }, -- wand of plague
    { id = 2150, chance = 500 }, -- small amethyst
    { id = 2675, chance = 1000, count_max = 5 }, -- orange
    { id = 2162, chance = 5000 }, -- magic light wand
    { id = 2148, chance = 80000, count_max = 25 }, -- gold coin
    { id = 3966, chance = 100 }, -- banana staff
    { id = 2676, chance = 5000, count_max = 10 }, -- banana
    { id = 2676, chance = 30000, count_max = 2 }, -- banana
  },
}
