-- Generated from XML. Source: monsters/hunter.xml
return {
  schema = 1,
  name = "Hunter",
  description = "a hunter",
  race = "blood",
  experience = 150,
  speed = 65,
  mana_cost = 530,
  health = 150,
  max_health = 150,
  outfit = {
    look_type = 129,
    look_head = 95,
    look_body = 116,
    look_legs = 120,
    look_feet = 115,
    corpse = 3058,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = true,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 13,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 2,
      min = -50,
      max = -100,
      range = 7,
      shoot = "arrow",
    },
  },
  defenses = {
    armor = 8,
    defense = 10,
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
    { text = "Guess who we are hunting, hahaha!", yell = false },
  },
  loot = {
    { id = 2050, chance = 5000 }, -- torch
    { id = 2147, chance = 200 }, -- small ruby
    { id = 2690, chance = 11000, count_max = 2 }, -- roll
    { id = 2675, chance = 20000, count_max = 2 }, -- orange
    { id = 2649, chance = 14000 }, -- leather legs
    { id = 2461, chance = 10000 }, -- leather helmet
    { id = 2201, chance = 3000 }, -- dragon necklace
    { id = 2546, chance = 5000, count_max = 3 }, -- burst arrow
    { id = 2460, chance = 5000 }, -- brass helmet
    { id = 2465, chance = 5000 }, -- brass armor
    { id = 2456, chance = 30000 }, -- bow
    { id = 2544, chance = 40000, count_max = 12 }, -- arrow
    { id = 2544, chance = 70000, count_max = 10 }, -- arrow
  },
}
