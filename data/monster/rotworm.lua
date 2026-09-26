-- Generated from XML. Source: monsters/rotworm.xml
return {
  schema = 1,
  name = "Rotworm",
  description = "a rotworm",
  race = "blood",
  experience = 40,
  speed = 18,
  mana_cost = 305,
  health = 65,
  max_health = 65,
  outfit = {
    look_type = 26,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4005,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = true,
    can_push_items = false,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 26,
      attack = 18,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 11,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = false,
    physical = false,
    outfit = true,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  loot = {
    { id = 3492, chance = 50000, count_max = 5 }, -- worm
    { id = 3264, chance = 3000 }, -- sword
    { id = 3577, chance = 20000 }, -- meat
    { id = 3286, chance = 4500 }, -- mace
    { id = 3374, chance = 1500 }, -- legion helmet
    { id = 3300, chance = 300 }, -- katana
    { id = 3582, chance = 20000 }, -- ham
    { id = 3031, chance = 30000, count_max = 12 }, -- gold coin
    { id = 3031, chance = 60000, count_max = 8 }, -- gold coin
    { id = 3430, chance = 1000 }, -- copper shield
  },
}
