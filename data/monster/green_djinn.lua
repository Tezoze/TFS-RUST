-- Generated from XML. Source: monsters/green djinn.xml
return {
  schema = 1,
  name = "Green Djinn",
  description = "a green djinn",
  race = "blood",
  experience = 190,
  speed = 70,
  mana_cost = 0,
  health = 330,
  max_health = 330,
  outfit = {
    look_type = 51,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 2989,
  },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
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
      skill = 50,
      attack = 30,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "outfit",
      delay = 6,
      range = 7,
      duration = 30000,
      monster = "Rat",
      effect = "blueshimmer",
    },
    {
      name = "drunk",
      delay = 5,
      range = 7,
      duration = 30000,
      drunkness = 120,
      shoot = "energy",
      effect = "teleport",
    },
    {
      name = "lifedrain",
      delay = 4,
      min = -55,
      max = -105,
      range = 7,
      shoot = "death",
    },
    {
      name = "energycondition",
      delay = 3,
      range = 7,
      cycle = 70,
      min_cycle = 20,
      shoot = "energy",
    },
    {
      name = "fire",
      delay = 2,
      min = -45,
      max = -75,
      range = 7,
      shoot = "fire",
    },
  },
  defenses = {
    armor = 20,
    defense = 30,
  },
  immunities = {
    fire = true,
    energy = true,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "I grant you a deathwish!", yell = false },
    { text = "Muhahahaha!", yell = false },
    { text = "I wish you a merry trip to hell!", yell = false },
    { text = "Tell me your last wish!", yell = false },
    { text = "Good wishes are for fairytales", yell = false },
  },
  loot = {
    { id = 2063, chance = 7500 }, -- small oil lamp
    { id = 2149, chance = 2700, count_max = 4 }, -- small emerald
    { id = 2663, chance = 100 }, -- mystic turban
    { id = 2747, chance = 10000 }, -- grave flower
    { id = 2148, chance = 70000, count_max = 50 }, -- gold coin
    { id = 2696, chance = 25000 }, -- cheese
    { id = 1980, chance = 2500 }, -- book
  },
}
