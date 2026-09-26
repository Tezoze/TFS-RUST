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
    corpse = 4321,
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
    { id = 3345, chance = 500 }, -- templar scytheblade
    { id = 3264, chance = 5000 }, -- sword
    { id = 3351, chance = 2000 }, -- steel helmet
    { id = 3032, chance = 300 }, -- small emerald
    { id = 3294, chance = 10000 }, -- short sword
    { id = 3445, chance = 100 }, -- salamander shield
    { id = 3357, chance = 1000 }, -- plate armor
    { id = 3282, chance = 700 }, -- morning star
    { id = 3031, chance = 80000, count_max = 20 }, -- gold coin
  },
}
