-- Generated from XML. Source: monsters/blue djinn.xml
return {
  schema = 1,
  name = "Blue Djinn",
  description = "a blue djinn",
  race = "blood",
  experience = 190,
  speed = 70,
  mana_cost = 0,
  health = 330,
  max_health = 330,
  outfit = {
    look_type = 80,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3001,
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
      duration = 20000,
      monster = "Rabbit",
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
    { text = "Simsalabim", yell = false },
    { text = "Feel the power of my magic, tiny mortal!", yell = false },
    { text = "Be careful what you wish for.", yell = false },
    { text = "Wishes can come true.", yell = false },
  },
  loot = {
    { id = 2146, chance = 2500, count_max = 4 }, -- small sapphire
    { id = 2063, chance = 7500 }, -- small oil lamp
    { id = 2663, chance = 100 }, -- mystic turban
    { id = 2148, chance = 70000, count_max = 50 }, -- gold coin
    { id = 2684, chance = 25000 }, -- carrot
    { id = 1978, chance = 2500 }, -- book
    { id = 2745, chance = 500 }, -- blue rose
  },
}
