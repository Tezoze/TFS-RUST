-- Generated from XML. Source: monsters/sibang.xml
return {
  schema = 1,
  name = "Sibang",
  description = "a sibang",
  race = "blood",
  experience = 100,
  speed = 67,
  mana_cost = 0,
  health = 225,
  max_health = 225,
  outfit = {
    look_type = 118,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4339,
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
      skill = 33,
      attack = 20,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 4,
      min = -35,
      max = -55,
      range = 7,
      shoot = "smallstone",
    },
  },
  defenses = {
    armor = 15,
    defense = 32,
    spells = {
      {
        name = "speed",
        delay = 9,
        duration = 3000,
        speed = 75,
        speed_variation = 5,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "Eeeeek! Eeeeek", yell = false },
    { text = "Huh! Huh! Huh!", yell = false },
    { text = "Ahhuuaaa!", yell = false },
  },
  loot = {
    { id = 1781, chance = 30000, count_max = 3 }, -- small stone
    { id = 3586, chance = 20000, count_max = 5 }, -- orange
    { id = 3593, chance = 10000 }, -- melon
    { id = 3031, chance = 80000, count_max = 25 }, -- gold coin
    { id = 3589, chance = 20000, count_max = 5 }, -- coconut
    { id = 3352, chance = 4000 }, -- chain helmet
    { id = 3587, chance = 5000, count_max = 10 }, -- banana
    { id = 3587, chance = 30000, count_max = 2 }, -- banana
  },
}
