-- Generated from XML. Source: monsters/warlock.xml
return {
  schema = 1,
  name = "Warlock",
  description = "a warlock",
  race = "blood",
  experience = 4000,
  speed = 75,
  mana_cost = 0,
  health = 3200,
  max_health = 3200,
  outfit = {
    look_type = 130,
    look_head = 0,
    look_body = 52,
    look_legs = 128,
    look_feet = 95,
    corpse = 3058,
  },
  change_target = { chance = 50 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 50 },
  flags = {
    hostile = true,
    summonable = false,
    illusionable = false,
    pushable = false,
    convinceable = false,
    can_push_items = true,
    can_push_creatures = true,
    target_distance = 4,
    run_health = 1000,
  },
  attacks = {
    {
      name = "melee",
      skill = 50,
      attack = 40,
      skill_factor = 1100,
      skill_next_level = 50,
      skill_add_count = 2,
    },
    {
      name = "energy",
      delay = 8,
      min = -145,
      max = -205,
      length = 8,
      spread = 0,
      effect = "energy",
    },
    {
      name = "firefield",
      delay = 5,
      range = 7,
      radius = 2,
      target = true,
      shoot = "fire",
    },
    {
      name = "firefield",
      delay = 7,
      range = 7,
      radius = 1,
      target = true,
      shoot = "fire",
    },
    {
      name = "fire",
      delay = 3,
      min = -90,
      max = -170,
      range = 7,
      radius = 3,
      target = true,
      shoot = "fire",
      effect = "firearea",
    },
    {
      name = "speed",
      delay = 9,
      range = 7,
      duration = 40000,
      speed = -80,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "manadrain",
      delay = 6,
      min = -35,
      max = -75,
      range = 7,
    },
    {
      name = "physical",
      delay = 2,
      min = -45,
      max = -105,
      range = 7,
      shoot = "energy",
    },
  },
  defenses = {
    armor = 32,
    defense = 50,
    spells = {
      {
        name = "invisible",
        delay = 10,
        duration = 20000,
        effect = "blueshimmer",
      },
      {
        name = "healing",
        delay = 4,
        min = 60,
        max = 100,
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
    life_drain = false,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Learn the secret of our magic! YOUR death!", yell = false },
    { text = "Even a rat is a better mage than you.", yell = false },
    { text = "We don't like intruders!", yell = false },
  },
  summons = {
    max = 1,
    { name = "Stone Golem", delay = 10, max = 1 },
  },
  loot = {
    { id = 2151, chance = 1100 }, -- talon
    { id = 2197, chance = 500 }, -- stone skin amulet
    { id = 2146, chance = 1400 }, -- small sapphire
    { id = 2436, chance = 7000 }, -- skull staff
    { id = 2123, chance = 200 }, -- ring of the sky
    { id = 1986, chance = 400 }, -- red tome
    { id = 2411, chance = 10000 }, -- poison dagger
    { id = 2178, chance = 2500 }, -- mind stone
    { id = 2600, chance = 13000 }, -- inkwell
    { id = 2466, chance = 300 }, -- golden armor
    { id = 2148, chance = 30000, count_max = 80 }, -- gold coin
    { id = 2167, chance = 3000 }, -- energy ring
    { id = 2792, chance = 3000 }, -- dark mushroom
    { id = 2124, chance = 1000 }, -- crystal ring
    { id = 2679, chance = 20000, count_max = 4 }, -- cherry
    { id = 2047, chance = 15000 }, -- candlestick
    { id = 2689, chance = 11000 }, -- bread
    { id = 2656, chance = 2000 }, -- blue robe
  },
}
