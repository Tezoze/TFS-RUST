-- Generated from XML. Source: monsters/ghost.xml
return {
  schema = 1,
  name = "Ghost",
  description = "a ghost",
  race = "undead",
  experience = 120,
  speed = 40,
  mana_cost = 0,
  health = 150,
  max_health = 150,
  outfit = {
    look_type = 48,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2913,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 45,
      attack = 28,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 7,
      min = -25,
      max = -45,
      range = 1,
    },
  },
  defenses = {
    armor = 10,
    defense = 20,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = true,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = false,
  },
  voices = {
    { text = "Huh!", yell = false },
    { text = "Shhhhhh", yell = false },
    { text = "Buuuuuh", yell = false },
  },
  loot = {
    { id = 2165, chance = 200 }, -- stealth ring
    { id = 2804, chance = 15000 }, -- shadow herb
    { id = 2642, chance = 20000 }, -- sandals
    { id = 2394, chance = 11000 }, -- morning star
    { id = 2404, chance = 7000 }, -- combat knife
    { id = 2654, chance = 9000 }, -- cape
    { id = 1977, chance = 1500 }, -- book
    { id = 2532, chance = 800 }, -- ancient shield
  },
}
