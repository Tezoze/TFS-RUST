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
    corpse = 4109,
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
    { id = 3412, chance = 15000 }, -- wooden shield
    { id = 3130, chance = 8000 }, -- twigs
    { id = 3277, chance = 20000 }, -- spear
    { id = 3272, chance = 15000 }, -- rapier
    { id = 3031, chance = 50000, count_max = 12 }, -- gold coin
    { id = 3578, chance = 18000 }, -- fish
    { id = 3562, chance = 12000 }, -- coat
    { id = 3270, chance = 9000 }, -- club
  },
}
