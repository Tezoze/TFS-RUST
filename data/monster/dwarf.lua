-- Generated from XML. Source: monsters/dwarf.xml
return {
  schema = 1,
  name = "Dwarf",
  description = "a dwarf",
  race = "blood",
  experience = 45,
  speed = 45,
  mana_cost = 320,
  health = 90,
  max_health = 90,
  outfit = {
    look_type = 69,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2960,
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
      skill = 23,
      attack = 14,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 8,
    defense = 14,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = false,
    paralyze = false,
    invisible = false,
  },
  voices = {
    { text = "Hail Durin!", yell = false },
  },
  loot = {
    { id = 2787, chance = 50000 }, -- white mushroom
    { id = 2484, chance = 8000 }, -- studded armor
    { id = 2553, chance = 10000 }, -- pick
    { id = 2597, chance = 8000 }, -- letter
    { id = 2649, chance = 10000 }, -- leather legs
    { id = 2388, chance = 25000 }, -- hatchet
    { id = 2148, chance = 45000, count_max = 8 }, -- gold coin
    { id = 2213, chance = 100 }, -- dwarven ring
    { id = 2530, chance = 10000 }, -- copper shield
    { id = 2386, chance = 15000 }, -- axe
  },
}
