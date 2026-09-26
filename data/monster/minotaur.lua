-- Generated from XML. Source: monsters/minotaur.xml
return {
  schema = 1,
  name = "Minotaur",
  description = "a minotaur",
  race = "blood",
  experience = 50,
  speed = 44,
  mana_cost = 330,
  health = 100,
  max_health = 100,
  outfit = {
    look_type = 25,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4011,
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
      skill = 25,
      attack = 15,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 11,
    defense = 11,
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
    { text = "Kaplar!", yell = false },
  },
  loot = {
    { id = 3264, chance = 10000 }, -- sword
    { id = 3457, chance = 3000 }, -- shovel
    { id = 3410, chance = 20000 }, -- plate shield
    { id = 3577, chance = 10000 }, -- meat
    { id = 3286, chance = 13000 }, -- mace
    { id = 3559, chance = 15000 }, -- leather legs
    { id = 3031, chance = 25000, count_max = 15 }, -- gold coin
    { id = 3031, chance = 55000, count_max = 10 }, -- gold coin
    { id = 3352, chance = 5000 }, -- chain helmet
    { id = 3358, chance = 10000 }, -- chain armor
    { id = 3056, chance = 100 }, -- bronze amulet
    { id = 3354, chance = 8000 }, -- brass helmet
    { id = 3274, chance = 4000 }, -- axe
  },
}
