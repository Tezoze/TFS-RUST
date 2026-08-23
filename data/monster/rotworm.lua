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
    corpse = 2824,
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
    { id = 3976, chance = 50000, count_max = 5 }, -- worm
    { id = 2376, chance = 3000 }, -- sword
    { id = 2666, chance = 20000 }, -- meat
    { id = 2398, chance = 4500 }, -- mace
    { id = 2480, chance = 1500 }, -- legion helmet
    { id = 2412, chance = 300 }, -- katana
    { id = 2671, chance = 20000 }, -- ham
    { id = 2148, chance = 30000, count_max = 12 }, -- gold coin
    { id = 2148, chance = 60000, count_max = 8 }, -- gold coin
    { id = 2530, chance = 1000 }, -- copper shield
  },
}
