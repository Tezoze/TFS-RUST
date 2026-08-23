-- Generated from XML. Source: monsters/marid.xml
return {
  schema = 1,
  name = "Marid",
  description = "a marid",
  race = "blood",
  experience = 300,
  speed = 77,
  mana_cost = 0,
  health = 550,
  max_health = 550,
  outfit = {
    look_type = 104,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3040,
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
      skill = 55,
      attack = 35,
      skill_factor = 2000,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "energycondition",
      delay = 6,
      radius = 3,
      cycle = 80,
      min_cycle = 20,
      target = false,
      effect = "energy",
    },
    {
      name = "outfit",
      delay = 6,
      range = 7,
      duration = 20000,
      monster = "Rabbit",
      effect = "blueshimmer",
    },
    {
      name = "drunk",
      delay = 5,
      range = 7,
      duration = 60000,
      drunkness = 120,
      shoot = "energy",
      effect = "teleport",
    },
    {
      name = "speed",
      delay = 8,
      range = 7,
      duration = 15000,
      speed = -75,
      speed_variation = 25,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 5,
      min = -60,
      max = -120,
      range = 7,
      shoot = "death",
    },
    {
      name = "energy",
      delay = 4,
      min = -65,
      max = -115,
      range = 7,
      shoot = "energy",
    },
    {
      name = "fire",
      delay = 2,
      min = -40,
      max = -110,
      range = 7,
      shoot = "fire",
    },
  },
  defenses = {
    armor = 24,
    defense = 35,
    spells = {
      {
        name = "healing",
        delay = 7,
        min = 50,
        max = 80,
        effect = "blueshimmer",
      },
    },
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
    { text = "Simsalabim", yell = false },
    { text = "Feel the power of my magic, tiny mortal!", yell = false },
    { text = "Be careful what you wish for.", yell = false },
    { text = "Wishes can come true.", yell = false },
  },
  summons = {
    max = 2,
    { name = "Blue Djinn", delay = 7, max = 2 },
  },
  loot = {
    { id = 2070, chance = 300 }, -- wooden flute
    { id = 2183, chance = 500 }, -- tempest rod
    { id = 2146, chance = 7000, count_max = 2 }, -- small sapphire
    { id = 2063, chance = 20000 }, -- small oil lamp
    { id = 2663, chance = 200 }, -- mystic turban
    { id = 2442, chance = 20000 }, -- heavy machete
    { id = 2148, chance = 50000, count_max = 80 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 50 }, -- gold coin
    { id = 2677, chance = 25000, count_max = 25 }, -- blueberry
    { id = 1872, chance = 2500 }, -- blue tapestry
    { id = 2158, chance = 100 }, -- blue gem
  },
}
