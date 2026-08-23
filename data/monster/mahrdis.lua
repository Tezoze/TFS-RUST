-- Generated from XML. Source: monsters/mahrdis.xml
return {
  schema = 1,
  name = "Mahrdis",
  description = "",
  race = "undead",
  experience = 3050,
  speed = 110,
  mana_cost = 0,
  health = 3900,
  max_health = 3900,
  outfit = {
    look_type = 86,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3016,
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
      skill = 60,
      attack = 45,
      poison_cycles = 65,
      skill_factor = 1500,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "firecondition",
      delay = 8,
      length = 8,
      spread = 3,
      cycle = 450,
      min_cycle = 50,
      effect = "explosionarea",
    },
    {
      name = "firefield",
      delay = 9,
      radius = 4,
      target = false,
      effect = "fire",
    },
    {
      name = "fire",
      delay = 3,
      min = -100,
      max = -800,
      radius = 3,
      target = false,
      effect = "explosion",
    },
    {
      name = "speed",
      delay = 8,
      range = 7,
      duration = 50000,
      speed = -90,
      speed_variation = 20,
      effect = "redshimmer",
    },
    {
      name = "fire",
      delay = 16,
      min = -300,
      max = -600,
      range = 7,
      shoot = "fire",
      effect = "fire",
    },
    {
      name = "firecondition",
      delay = 4,
      range = 1,
      cycle = 550,
      min_cycle = 250,
      effect = "firearea",
    },
    {
      name = "lifedrain",
      delay = 15,
      min = -50,
      max = -750,
      range = 1,
    },
  },
  defenses = {
    armor = 40,
    defense = 60,
    spells = {
      {
        name = "healing",
        delay = 5,
        min = 20,
        max = 800,
        effect = "fire",
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
    { text = "Burnnnnnnnnn!", yell = false },
    { text = "Fire, Fire!", yell = false },
    { text = "May my flames engulf you!", yell = false },
    { text = "The eternal flame demands its due!", yell = false },
    { text = "I am hotter than hot.", yell = false },
    { text = "Ashes to ashes!", yell = false },
  },
  summons = {
    max = 4,
    { name = "Fire Elemental", delay = 9, max = 4 },
  },
  loot = {
    { id = 2147, chance = 10000, count_max = 3 }, -- small ruby
    { id = 2156, chance = 1000 }, -- red gem
    { id = 2539, chance = 100 }, -- phoenix shield
    { id = 2168, chance = 5000 }, -- life ring
    { id = 2141, chance = 100 }, -- holy falcon
    { id = 2148, chance = 35000, count_max = 95 }, -- gold coin
    { id = 2148, chance = 50000, count_max = 85 }, -- gold coin
    { id = 2148, chance = 70000, count_max = 80 }, -- gold coin
    { id = 2432, chance = 200 }, -- fire axe
    { id = 2353, chance = 100000 }, -- burning heart
  },
}
