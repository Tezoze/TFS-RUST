-- Generated from XML. Source: monsters/bonebeast.xml
return {
  schema = 1,
  name = "Bonebeast",
  description = "a bonebeast",
  race = "undead",
  experience = 580,
  speed = 69,
  mana_cost = 0,
  health = 515,
  max_health = 515,
  outfit = {
    look_type = 101,
    look_head = 0,
    look_body = 0,
    look_legs = 0,
    look_feet = 0,
    corpse = 3031,
  },
  change_target = { chance = 20 },
  target_strategy = { nearest = 100, weakest = 0, most_damage = 0, random = 0 },
  lose_target = { chance = 20 },
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
      skill = 75,
      attack = 47,
      poison_cycles = 110,
      skill_factor = 1100,
      skill_next_level = 100,
      skill_add_count = 2,
    },
    {
      name = "poisoncondition",
      delay = 8,
      radius = 3,
      cycle = 70,
      min_cycle = 10,
      target = false,
      effect = "poison",
    },
    {
      name = "lifedrain",
      delay = 7,
      min = -30,
      max = -50,
      radius = 3,
      target = false,
      effect = "redshimmer",
    },
    {
      name = "poison",
      delay = 10,
      min = -25,
      max = -65,
      range = 7,
      shoot = "poison",
      effect = "poison",
    },
  },
  defenses = {
    armor = 40,
    defense = 45,
    spells = {
      {
        name = "healing",
        delay = 9,
        min = 30,
        max = 60,
        effect = "greenspark",
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
    { text = "Cccchhhhhhhhh!", yell = false },
    { text = "Knooorrrrr!", yell = false },
  },
  loot = {
    { id = 2229, chance = 20000 }, -- skull
    { id = 2463, chance = 8000 }, -- plate armor
    { id = 2796, chance = 1500 }, -- green mushroom
    { id = 2148, chance = 30000, count_max = 90 }, -- gold coin
    { id = 2541, chance = 2000 }, -- bone shield
    { id = 2449, chance = 5000 }, -- bone club
    { id = 2230, chance = 50000 }, -- bone
    { id = 2231, chance = 10000 }, -- big bone
  },
}
