-- Generated from XML. Source: monsters/smuggler.xml
return {
  schema = 1,
  name = "Smuggler",
  description = "a smuggler",
  race = "blood",
  experience = 48,
  speed = 48,
  mana_cost = 390,
  health = 130,
  max_health = 130,
  outfit = {
    look_type = 134,
    look_head = 95,
    look_body = 0,
    look_legs = 113,
    look_feet = 115,
    corpse = 4240,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 18,
  },
  attacks = {
    {
      name = "melee",
      skill = 23,
      attack = 19,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 10,
    defense = 13,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "I will silence you forever!", yell = false },
    { text = "You saw something you shouldn't!", yell = false },
  },
  loot = {
    { id = 2920, chance = 30000, count_max = 2 }, -- torch
    { id = 3264, chance = 5000 }, -- sword
    { id = 3294, chance = 10000 }, -- short sword
    { id = 3577, chance = 50000 }, -- meat
    { id = 3559, chance = 15000 }, -- leather legs
    { id = 3355, chance = 10000 }, -- leather helmet
    { id = 3291, chance = 10000 }, -- knife
    { id = 3582, chance = 10000 }, -- ham
    { id = 3031, chance = 80000, count_max = 10 }, -- gold coin
    { id = 3292, chance = 4000 }, -- combat knife
  },
}
