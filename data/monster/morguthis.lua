-- Generated from XML. Source: monsters/morguthis.xml
return {
  schema = 1,
  name = "Morguthis",
  description = "",
  race = "undead",
  experience = 3000,
  speed = 175,
  mana_cost = 0,
  health = 4800,
  max_health = 4800,
  outfit = {
    look_type = 84,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3016,
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 80, weakest = 10, most_damage = 10, random = 0 },
  lose_target = { chance = 5 },
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
      skill = 150,
      attack = 85,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "physical",
      delay = 16,
      min = -450,
      max = -750,
      radius = 3,
      target = false,
      effect = "mortarea",
    },
    {
      name = "physical",
      delay = 5,
      min = -300,
      max = -600,
      radius = 3,
      target = false,
      effect = "blackspark",
    },
    {
      name = "speed",
      delay = 4,
      range = 7,
      duration = 50000,
      speed = -100,
      speed_variation = 10,
      effect = "redshimmer",
    },
    {
      name = "lifedrain",
      delay = 15,
      min = -250,
      max = -550,
      range = 1,
    },
  },
  defenses = {
    armor = 75,
    defense = 85,
    spells = {
      {
        name = "invisible",
        delay = 13,
        duration = 1000,
        effect = "blueshimmer",
      },
      {
        name = "speed",
        delay = 15,
        duration = 5000,
        speed = 60,
        speed_variation = 5,
        effect = "redshimmer",
      },
      {
        name = "healing",
        delay = 8,
        min = 200,
        max = 300,
        effect = "blueshimmer",
      },
    },
  },
  immunities = {
    fire = false,
    energy = false,
    poison = true,
    physical = false,
    outfit = true,
    life_drain = true,
    paralyze = true,
    invisible = true,
  },
  voices = {
    { text = "Vengeance!", yell = false },
    { text = "You will make a fine trophy.", yell = false },
    { text = "Come and fight me, cowards!", yell = false },
    { text = "I am the supreme warrior!", yell = false },
    { text = "Let me hear the music of battle.", yell = false },
    { text = "Anotherone to bite the dust!", yell = false },
  },
  summons = {
    max = 2,
    { name = "Hero", delay = 15, max = 2 },
  },
  loot = {
    { id = 2350, chance = 100000 }, -- sword hilt
    { id = 2197, chance = 5000 }, -- stone skin amulet
    { id = 2645, chance = 100 }, -- steel boots
    { id = 2443, chance = 100 }, -- ravager's axe
    { id = 2430, chance = 5000 }, -- knight axe
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2136, chance = 100 }, -- demonbone amulet
    { id = 2144, chance = 10000 }, -- black pearl
  },
}
