-- Generated from XML. Source: monsters/elder beholder.xml
return {
  schema = 1,
  name = "Elder Beholder",
  description = "an elder beholder",
  race = "blood",
  experience = 280,
  speed = 45,
  mana_cost = 0,
  health = 500,
  max_health = 500,
  outfit = {
    look_type = 108,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3052,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = true,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = false,
    target_distance = 4,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 45,
      attack = 16,
      skill_factor = 1200,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "speed",
      delay = 10,
      range = 7,
      duration = 20000,
      speed = -90,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "manadrain",
      delay = 19,
      min = -20,
      max = -40,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 18,
      min = -75,
      max = -85,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 18,
      min = -75,
      max = -85,
      range = 7,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 13,
      min = -30,
      max = -70,
      range = 7,
      shoot = "poison",
    },
    {
      name = "physical",
      delay = 12,
      min = -70,
      max = -90,
      range = 7,
      shoot = "death",
      effect = "mortarea",
    },
    {
      name = "fire",
      delay = 15,
      min = -60,
      max = -80,
      range = 7,
      shoot = "fire",
    },
    {
      name = "energy",
      delay = 14,
      min = -45,
      max = -75,
      range = 7,
      shoot = "energy",
    },
  },
  defenses = {
    armor = 13,
    defense = 26,
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = false,
    life_drain = true,
    paralyze = false,
    invisible = true,
  },
  voices = {
    { text = "653768764!", yell = false },
    { text = "Let me take a look at you!", yell = false },
    { text = "Inferior creatures, bow before my power!", yell = false },
    { text = "659978 54764!", yell = false },
  },
  summons = {
    max = 6,
    { name = "Crypt Shambler", delay = 9, max = 6 },
    { name = "Gazer", delay = 8, max = 6 },
  },
  loot = {
    { id = 2377, chance = 6000 }, -- two handed sword
    { id = 2509, chance = 6000 }, -- steel shield
    { id = 2175, chance = 1000 }, -- spellbook
    { id = 2394, chance = 10000 }, -- morning star
    { id = 2397, chance = 12000 }, -- longsword
    { id = 2148, chance = 70000, count_max = 35 }, -- gold coin
    { id = 2148, chance = 80000, count_max = 32 }, -- gold coin
    { id = 2148, chance = 90000, count_max = 24 }, -- gold coin
    { id = 2518, chance = 100 }, -- beholder shield
    { id = 3972, chance = 100 }, -- beholder helmet
  },
}
