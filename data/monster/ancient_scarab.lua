-- Generated from XML. Source: monsters/ancient scarab.xml
return {
  schema = 1,
  name = "Ancient Scarab",
  description = "an ancient scarab",
  race = "venom",
  experience = 720,
  speed = 69,
  mana_cost = 0,
  health = 1000,
  max_health = 1000,
  outfit = {
    look_type = 79,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3004,
  },
  change_target = { chance = 10 },
  target_strategy = { nearest = 70, weakest = 20, most_damage = 0, random = 10 },
  lose_target = { chance = 10 },
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
      skill = 80,
      attack = 50,
      poison_cycles = 100,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 1,
    },
    {
      name = "speed",
      delay = 4,
      radius = 5,
      duration = 20000,
      speed = -80,
      speed_variation = 20,
      target = false,
      effect = "poison",
    },
    {
      name = "poisoncondition",
      delay = 6,
      radius = 5,
      cycle = 450,
      min_cycle = 50,
      target = false,
      effect = "poison",
    },
    {
      name = "poisoncondition",
      delay = 6,
      radius = 5,
      cycle = 450,
      min_cycle = 50,
      target = false,
      effect = "poison",
    },
    {
      name = "speed",
      delay = 8,
      range = 7,
      duration = 25000,
      speed = -90,
      speed_variation = 10,
      shoot = "poison",
      effect = "poison",
    },
    {
      name = "poison",
      delay = 9,
      min = -15,
      max = -135,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
  },
  defenses = {
    armor = 36,
    defense = 33,
    spells = {
      {
        name = "speed",
        delay = 13,
        duration = 9000,
        speed = 85,
        speed_variation = 5,
        effect = "redshimmer",
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
  summons = {
    max = 3,
    { name = "Larva", delay = 7, max = 3 },
  },
  loot = {
    { id = 2149, chance = 600, count_max = 3 }, -- small emerald
    { id = 2150, chance = 1200, count_max = 4 }, -- small amethyst
    { id = 2540, chance = 500 }, -- scarab shield
    { id = 2159, chance = 1000 }, -- scarab coin
    { id = 2159, chance = 5000, count_max = 2 }, -- scarab coin
    { id = 2135, chance = 500 }, -- scarab amulet
    { id = 2463, chance = 10000 }, -- plate armor
    { id = 2162, chance = 10900 }, -- magic light wand
    { id = 2148, chance = 99900, count_max = 22 }, -- gold coin
    { id = 2148, chance = 75700, count_max = 66 }, -- gold coin
    { id = 2148, chance = 44400, count_max = 100 }, -- gold coin
    { id = 2440, chance = 300 }, -- daramanian waraxe
    { id = 2142, chance = 1000 }, -- ancient amulet
  },
}
