-- Generated from XML. Source: monsters/lizard templar.xml
return {
  schema = 1,
  name = "Lizard Templar",
  description = "a lizard templar",
  race = "blood",
  experience = 145,
  speed = 47,
  mana_cost = 0,
  health = 410,
  max_health = 410,
  outfit = {
    look_type = 113,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 4256,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 44,
      attack = 30,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
  },
  defenses = {
    armor = 26,
    defense = 20,
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
    { text = "Hissss!", yell = false },
  },
  loot = {
    { id = 3963, chance = 500 }, -- templar scytheblade
    { id = 2376, chance = 5000 }, -- sword
    { id = 2457, chance = 2000 }, -- steel helmet
    { id = 2149, chance = 300 }, -- small emerald
    { id = 2406, chance = 10000 }, -- short sword
    { id = 3975, chance = 100 }, -- salamander shield
    { id = 2463, chance = 1000 }, -- plate armor
    { id = 2394, chance = 700 }, -- morning star
    { id = 2148, chance = 80000, count_max = 20 }, -- gold coin
  },
}
