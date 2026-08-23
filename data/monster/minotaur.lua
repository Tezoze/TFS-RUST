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
    corpse = 2830,
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
    { id = 2376, chance = 10000 }, -- sword
    { id = 2554, chance = 3000 }, -- shovel
    { id = 2510, chance = 20000 }, -- plate shield
    { id = 2666, chance = 10000 }, -- meat
    { id = 2398, chance = 13000 }, -- mace
    { id = 2649, chance = 15000 }, -- leather legs
    { id = 2148, chance = 25000, count_max = 15 }, -- gold coin
    { id = 2148, chance = 55000, count_max = 10 }, -- gold coin
    { id = 2458, chance = 5000 }, -- chain helmet
    { id = 2464, chance = 10000 }, -- chain armor
    { id = 2172, chance = 100 }, -- bronze amulet
    { id = 2460, chance = 8000 }, -- brass helmet
    { id = 2386, chance = 4000 }, -- axe
  },
}
