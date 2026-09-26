-- Generated from XML. Source: monsters/orc shaman.xml
return {
  schema = 1,
  name = "Orc Shaman",
  description = "an orc shaman",
  race = "blood",
  experience = 110,
  speed = 30,
  mana_cost = 0,
  health = 115,
  max_health = 115,
  outfit = {
    look_type = 6,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4041,
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
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 20,
      attack = 13,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "fire",
      delay = 13,
      min = -5,
      max = -45,
      range = 7,
      radius = 1,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "energy",
      delay = 8,
      min = -20,
      max = -30,
      range = 7,
      shoot = "energy",
      effect = "energy",
    },
  },
  defenses = {
    armor = 8,
    defense = 10,
    spells = {
      {
        name = "healing",
        delay = 4,
        min = 27,
        max = 43,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = true,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Grak brrretz gulu.", yell = false },
    { text = "Huumans stinkk!", yell = false },
  },
  summons = {
    max = 4,
    { name = "Snake", delay = 4, max = 4 },
  },
  loot = {
    { id = 3072, chance = 1000 }, -- wand of plague
    { id = 3289, chance = 7000 }, -- staff
    { id = 3277, chance = 10000 }, -- spear
    { id = 3031, chance = 90000, count_max = 5 }, -- gold coin
    { id = 3597, chance = 11000, count_max = 2 }, -- corncob
    { id = 2853, chance = 11000 }, -- bag
    { id = 3352, chance = 9000 }, -- chain helmet
    { id = 3358, chance = 9000 }, -- chain armor
    { id = 2839, chance = 4500 }, -- book
  },
}
