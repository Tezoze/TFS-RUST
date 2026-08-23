-- Generated from XML. Source: monsters/lich.xml
return {
  schema = 1,
  name = "Lich",
  description = "a lich",
  race = "undead",
  experience = 900,
  speed = 65,
  mana_cost = 0,
  health = 880,
  max_health = 880,
  outfit = {
    look_type = 99,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3025,
  },
  change_target = { chance = 3 },
  target_strategy = { nearest = 80, weakest = 10, most_damage = 10, random = 0 },
  lose_target = { chance = 3 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 1,
    run_health = 0,
  },
  attacks = {
    {
      name = "melee",
      skill = 40,
      attack = 40,
      poison_cycles = 400,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "poisoncondition",
      delay = 12,
      length = 8,
      spread = 0,
      cycle = 350,
      min_cycle = 50,
      effect = "greenspark",
    },
    {
      name = "lifedrain",
      delay = 12,
      min = -100,
      max = -200,
      length = 8,
      spread = 0,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 6,
      min = -120,
      max = -200,
      radius = 3,
      target = false,
      effect = "redshimmer",
    },
    {
      name = "speed",
      delay = 7,
      range = 7,
      duration = 30000,
      speed = -95,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 30,
      min = -50,
      max = -250,
      range = 1,
      effect = "blueshimmer",
    },
  },
  defenses = {
    armor = 50,
    defense = 60,
    spells = {
      {
        name = "healing",
        delay = 6,
        min = 50,
        max = 150,
        effect = "redshimmer",
      },
    },
  },
  immunities = {
    fire = true,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Death awaits all!", yell = false },
    { text = "Doomed be the living!", yell = false },
    { text = "Death and Decay!", yell = false },
    { text = "You will endure agony beyond thy death!", yell = false },
    { text = "Come to me my children!", yell = false },
    { text = "Pain sweet pain!", yell = false },
    { text = "Thy living flesh offends me!", yell = false },
  },
  summons = {
    max = 4,
    { name = "Bonebeast", delay = 6, max = 4 },
  },
  loot = {
    { id = 2143, chance = 2500 }, -- white pearl
    { id = 2479, chance = 500 }, -- strange helmet
    { id = 2401, chance = 60000 }, -- staff
    { id = 2175, chance = 10000 }, -- spellbook
    { id = 2214, chance = 1000 }, -- ring of healing
    { id = 2171, chance = 100 }, -- platinum amulet
    { id = 2178, chance = 500 }, -- mind stone
    { id = 2148, chance = 40000, count_max = 40 }, -- gold coin
    { id = 2148, chance = 30000, count_max = 80 }, -- gold coin
    { id = 2237, chance = 20000 }, -- dirty cape
    { id = 2535, chance = 200 }, -- castle shield
    { id = 2656, chance = 100 }, -- blue robe
    { id = 2144, chance = 5000 }, -- black pearl
  },
}
