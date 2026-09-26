-- Generated from XML. Source: monsters/stone golem.xml
return {
  schema = 1,
  name = "Stone Golem",
  description = "a stone golem",
  race = "undead",
  experience = 160,
  speed = 50,
  mana_cost = 590,
  health = 270,
  max_health = 270,
  outfit = {
    look_type = 67,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4133,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
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
      skill = 52,
      attack = 38,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 30,
    defense = 25,
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = true,
    invisible = false,
  },
  loot = {
    { id = 2920, chance = 5500 }, -- torch
    { id = 3409, chance = 7000 }, -- steel shield
    { id = 1781, chance = 13000, count_max = 4 }, -- small stone
    { id = 3377, chance = 5000 }, -- scale armor
    { id = 3039, chance = 100 }, -- red gem
    { id = 3050, chance = 5000 }, -- power ring
    { id = 3031, chance = 16000, count_max = 15 }, -- gold coin
    { id = 3007, chance = 200 }, -- crystal ring
    { id = 3283, chance = 1500 }, -- carlin sword
  },
}
