-- Generated from XML. Source: monsters/frost troll.xml
return {
  schema = 1,
  name = "Frost Troll",
  description = "a frost troll",
  race = "blood",
  experience = 23,
  speed = 30,
  mana_cost = 300,
  health = 55,
  max_health = 55,
  outfit = {
    look_type = 53,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2928,
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
    run_health = 10,
  },
  attacks = {
    {
      name = "melee",
      skill = 19,
      attack = 11,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 7,
    defense = 9,
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
    { text = "Brrrr", yell = false },
    { text = "Broar!", yell = false },
  },
  loot = {
    { id = 2512, chance = 15000 }, -- wooden shield
    { id = 2245, chance = 8000 }, -- twigs
    { id = 2389, chance = 20000 }, -- spear
    { id = 2384, chance = 15000 }, -- rapier
    { id = 2148, chance = 50000, count_max = 12 }, -- gold coin
    { id = 2667, chance = 18000 }, -- fish
    { id = 2651, chance = 12000 }, -- coat
    { id = 2382, chance = 9000 }, -- club
  },
}
