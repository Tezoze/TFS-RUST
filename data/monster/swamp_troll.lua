-- Generated from XML. Source: monsters/swamp troll.xml
return {
  schema = 1,
  name = "Swamp Troll",
  description = "a swamp troll",
  race = "blood",
  experience = 25,
  speed = 24,
  mana_cost = 320,
  health = 55,
  max_health = 55,
  outfit = {
    look_type = 76,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2995,
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
    run_health = 15,
  },
  attacks = {
    {
      name = "melee",
      skill = 20,
      attack = 12,
      poison_cycles = 10,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 6,
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
    { text = "Grrrr", yell = false },
    { text = "Groar!", yell = false },
    { text = "Me strong! Me ate spinach!", yell = false },
  },
  loot = {
    { id = 2050, chance = 15000 }, -- torch
    { id = 2643, chance = 10000 }, -- leather boots
    { id = 2148, chance = 50000, count_max = 5 }, -- gold coin
    { id = 2580, chance = 100 }, -- fishing rod
    { id = 2667, chance = 60000 }, -- fish
    { id = 2379, chance = 30000 }, -- dagger
  },
}
