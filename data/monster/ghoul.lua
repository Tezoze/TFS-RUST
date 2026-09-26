-- Generated from XML. Source: monsters/ghoul.xml
return {
  schema = 1,
  name = "Ghoul",
  description = "a ghoul",
  race = "blood",
  experience = 85,
  speed = 32,
  mana_cost = 450,
  health = 100,
  max_health = 100,
  outfit = {
    look_type = 18,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4034,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = true,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 37,
      attack = 26,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "lifedrain",
      delay = 9,
      min = -15,
      max = -25,
      range = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 17,
    spells = {
      {
        name = "healing",
        delay = 8,
        min = 9,
        max = 15,
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
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  loot = {
    { id = 3492, chance = 80000, count_max = 6 }, -- worm
    { id = 3367, chance = 5000 }, -- viking helmet
    { id = 2920, chance = 60000 }, -- torch
    { id = 3114, chance = 3000 }, -- skull
    { id = 3377, chance = 4000 }, -- scale armor
    { id = 3052, chance = 200 }, -- life ring
    { id = 3291, chance = 15000 }, -- knife
    { id = 3031, chance = 75000, count_max = 30 }, -- gold coin
    { id = 3354, chance = 20000 }, -- brass helmet
  },
}
