-- Generated from XML. Source: monsters/cave rat.xml
return {
  schema = 1,
  name = "Cave Rat",
  description = "a cave rat",
  race = "blood",
  experience = 10,
  speed = 35,
  mana_cost = 250,
  health = 30,
  max_health = 30,
  outfit = {
    look_type = 56,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2813,
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
    run_health = 3,
  },
  attacks = {
    {
      name = "melee",
      skill = 18,
      attack = 8,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 1,
    defense = 4,
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
    { text = "Meeeeep!", yell = false },
    { text = "Meep!", yell = false },
  },
  loot = {
    { id = 3976, chance = 50000, count_max = 3 }, -- worm
    { id = 2148, chance = 85000, count_max = 2 }, -- gold coin
    { id = 2687, chance = 1000 }, -- cookie
    { id = 2696, chance = 30000 }, -- cheese
  },
}
