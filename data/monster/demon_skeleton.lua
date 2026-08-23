-- Generated from XML. Source: monsters/demon skeleton.xml
return {
  schema = 1,
  name = "Demon Skeleton",
  description = "a demon skeleton",
  race = "undead",
  experience = 240,
  speed = 50,
  mana_cost = 620,
  health = 400,
  max_health = 400,
  outfit = {
    look_type = 37,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2809,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 0, most_damage = 30, random = 0 },
  lose_target = { chance = 5 },
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
      skill = 70,
      attack = 45,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "lifedrain",
      delay = 10,
      min = -30,
      max = -50,
      range = 1,
    },
  },
  defenses = {
    armor = 25,
    defense = 35,
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = true,
    invisible = false,
  },
  loot = {
    { id = 2050, chance = 50000 }, -- torch
    { id = 2399, chance = 10000, count_max = 3 }, -- throwing star
    { id = 2194, chance = 300 }, -- mysterious fetish
    { id = 2178, chance = 200 }, -- mind stone
    { id = 2459, chance = 2000 }, -- iron helmet
    { id = 2515, chance = 100 }, -- guardian shield
    { id = 2148, chance = 30000, count_max = 25 }, -- gold coin
    { id = 2148, chance = 40000, count_max = 20 }, -- gold coin
    { id = 2513, chance = 1000 }, -- battle shield
    { id = 2417, chance = 3000 }, -- battle hammer
  },
}
