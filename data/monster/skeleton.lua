-- Generated from XML. Source: monsters/skeleton.xml
return {
  schema = 1,
  name = "Skeleton",
  description = "a skeleton",
  race = "undead",
  experience = 35,
  speed = 37,
  mana_cost = 300,
  health = 50,
  max_health = 50,
  outfit = {
    look_type = 33,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2843,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = true,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 14,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "lifedrain",
      delay = 12,
      min = -7,
      max = -13,
      range = 1,
    },
  },
  defenses = {
    armor = 2,
    defense = 9,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = false,
    invisible = false,
  },
  loot = {
    { id = 2473, chance = 8000 }, -- viking helmet
    { id = 2050, chance = 50000 }, -- torch
    { id = 2376, chance = 2000 }, -- sword
    { id = 2398, chance = 20000 }, -- mace
    { id = 2388, chance = 25000 }, -- hatchet
    { id = 2148, chance = 45000, count_max = 10 }, -- gold coin
    { id = 2511, chance = 12000 }, -- brass shield
    { id = 2230, chance = 50000 }, -- bone
  },
}
